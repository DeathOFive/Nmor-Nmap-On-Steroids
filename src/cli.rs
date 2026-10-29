use std::ffi::OsString;
use std::path::PathBuf;

use clap::Parser;

/// Familiar Nmap-style options are normalized before Clap parses the command.
#[derive(Debug, Parser)]
#[command(
    name = "nmor",
    version,
    about = "Nmor Linux-first (TCP scanning + host discovery)"
)]
pub struct Cli {
    /// IP addresses, hostnames, and IPv4/IPv6 CIDR ranges (space-separated)
    #[arg(value_name = "TARGET", num_args = 0..)]
    pub targets: Vec<String>,

    /// Read targets from a file; whitespace-separated, # comments accepted (-iL)
    #[arg(long = "input-file", value_name = "FILE")]
    pub input_file: Option<PathBuf>,

    /// TCP ports: 80,443 or 20-25 or 1-1024 or - (all ports)
    #[arg(
        short = 'p',
        long = "ports",
        default_value = "22,80,443,8080,8443",
        value_name = "PORTS",
        allow_hyphen_values = true
    )]
    pub ports: String,

    /// TCP connect scan (-sT); this is the only scan engine in v0.2
    #[arg(long = "tcp-connect")]
    pub tcp_connect: bool,

    /// Skip host discovery (-Pn); treat all targets as up
    #[arg(long = "skip-discovery")]
    pub skip_discovery: bool,

    /// Only perform host discovery; do not scan ports (-sn)
    #[arg(long = "discovery-only")]
    pub discovery_only: bool,

    /// TCP ports used to discover hosts (-PS80,443); refused connections count as a response
    #[arg(
        long = "discovery-ports",
        default_value = "22,80,443",
        value_name = "PORTS"
    )]
    pub discovery_ports: String,

    /// Time limit for each TCP connection attempt (milliseconds)
    #[arg(long = "timeout", default_value_t = 1000, value_name = "MS")]
    pub timeout_ms: u64,

    /// Maximum in-flight TCP sockets across ALL targets (1-4096)
    #[arg(long = "concurrency", default_value_t = 256, value_name = "N")]
    pub concurrency: usize,

    /// Maximum number of hosts scanned/discovered simultaneously (1-128)
    #[arg(long = "host-concurrency", default_value_t = 16, value_name = "N")]
    pub host_concurrency: usize,

    /// Limit total expanded target addresses (1-65536, default 4096)
    #[arg(long = "max-hosts", default_value_t = 4096, value_name = "N")]
    pub max_hosts: usize,
}

impl Cli {
    pub fn parse_compat() -> Self {
        Self::parse_from(normalize_args(std::env::args_os()))
    }
}

/// Clap supports single-letter short options; normalize Nmap's combined flags.
/// `-PS80,443` selects TCP discovery probes for ports 80 and 443.
fn normalize_args<I>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    let mut normalized = Vec::new();
    for arg in args {
        match arg.to_str() {
            Some("-sT") => normalized.push(OsString::from("--tcp-connect")),
            Some("-Pn") => normalized.push(OsString::from("--skip-discovery")),
            Some("-sn") => normalized.push(OsString::from("--discovery-only")),
            Some("-iL") => normalized.push(OsString::from("--input-file")),
            Some("-p-") => {
                normalized.push(OsString::from("--ports"));
                normalized.push(OsString::from("-"));
            }
            Some("-PS") => {
                normalized.push(OsString::from("--discovery-ports"));
                normalized.push(OsString::from("80"));
            }
            Some(s) => {
                if let Some(value) = s.strip_prefix("-PS").filter(|v| !v.is_empty()) {
                    normalized.push(OsString::from("--discovery-ports"));
                    normalized.push(OsString::from(value));
                } else if let Some(value) = s.strip_prefix("-iL").filter(|v| !v.is_empty()) {
                    normalized.push(OsString::from("--input-file"));
                    normalized.push(OsString::from(value));
                } else {
                    normalized.push(arg);
                }
            }
            None => normalized.push(arg),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(words: &[&str]) -> Cli {
        let args = words.iter().map(|word| OsString::from(*word));
        Cli::try_parse_from(normalize_args(args)).expect("valid CLI arguments")
    }

    #[test]
    fn parses_multiple_targets_and_compat_flags() {
        let args = parse(&[
            "nmor",
            "-sT",
            "-Pn",
            "-p",
            "80,443",
            "127.0.0.1",
            "10.0.0.0/30",
        ]);
        assert!(args.tcp_connect);
        assert!(args.skip_discovery);
        assert_eq!(args.ports, "80,443");
        assert_eq!(
            args.targets.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["127.0.0.1", "10.0.0.0/30"]
        );
    }

    #[test]
    fn parses_discovery_only_and_tcp_probes() {
        let args = parse(&["nmor", "-sn", "-PS80,8080", "127.0.0.1/30"]);
        assert!(args.discovery_only);
        assert_eq!(args.discovery_ports, "80,8080");
    }

    #[test]
    fn parses_input_file_and_all_ports() {
        let args = parse(&["nmor", "-iL", "hosts.txt", "-p-", "localhost"]);
        assert_eq!(args.input_file, Some(PathBuf::from("hosts.txt")));
        assert_eq!(args.ports, "-");
        assert_eq!(
            args.targets.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["localhost"]
        );
    }

    #[test]
    fn accepts_attached_port_value() {
        let args = parse(&["nmor", "-p80,443", "127.0.0.1"]);
        assert_eq!(args.ports, "80,443");
    }

    #[test]
    fn accepts_default_tcp_probe_option() {
        let args = parse(&["nmor", "-PS", "127.0.0.1"]);
        assert_eq!(args.discovery_ports, "80");
        assert_eq!(
            args.targets.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["127.0.0.1"]
        );
    }
}
