use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;

use tokio::net::lookup_host;

#[derive(Debug, Clone)]
pub struct Target {
    pub label: String,
    pub ip: IpAddr,
}

/// Expand positional targets and -iL file entries, then deduplicate resolved IPs.
/// Expansion has a strict cap to avoid accidental massive /0 or /64 scans.
pub async fn expand_targets(
    positional: &[String],
    input_file: Option<&Path>,
    max_hosts: usize,
) -> Result<Vec<Target>, String> {
    let mut names = positional.to_vec();
    if let Some(path) = input_file {
        let contents = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read target file '{}': {error}", path.display()))?;
        for line in contents.lines() {
            let non_comment = line.split('#').next().unwrap_or("");
            names.extend(non_comment.split_whitespace().map(str::to_owned));
        }
    }
    if names.is_empty() {
        return Err("no targets specified; pass TARGET [TARGET...] or -iL FILE".into());
    }

    let mut targets = Vec::new();
    let mut seen = HashSet::new();
    for name in names {
        if name.contains('/') {
            // Check the range's own size first, then apply the limit to UNIQUE
            // IPs. Overlap with earlier targets must not count twice.
            let ips = expand_cidr(&name, max_hosts)?;
            for ip in ips {
                if !seen.contains(&ip) && targets.len() >= max_hosts {
                    return Err(format!("target count exceeds --max-hosts {max_hosts}"));
                }
                insert(&mut targets, &mut seen, ip.to_string(), ip);
            }
        } else {
            let ip = resolve_target(&name).await?;
            // Duplicated explicit hostnames/IPs do not consume extra slots.
            if !seen.contains(&ip) && targets.len() >= max_hosts {
                return Err(format!("target count exceeds --max-hosts {max_hosts}"));
            }
            insert(&mut targets, &mut seen, name, ip);
        }
    }
    Ok(targets)
}

fn insert(targets: &mut Vec<Target>, seen: &mut HashSet<IpAddr>, label: String, ip: IpAddr) {
    if seen.insert(ip) {
        targets.push(Target { label, ip });
    }
}

async fn resolve_target(target: &str) -> Result<IpAddr, String> {
    if let Ok(ip) = target.parse::<IpAddr>() {
        return Ok(ip);
    }
    let mut addresses: Vec<_> = lookup_host((target, 0))
        .await
        .map_err(|error| format!("cannot resolve '{target}': {error}"))?
        .collect();
    // Retain the v0.1 behavior: prefer IPv4 for dual-stack hostnames.
    addresses.sort_by_key(|addr| !addr.is_ipv4());
    addresses
        .first()
        .map(|addr| addr.ip())
        .ok_or_else(|| format!("hostname '{target}' resolved to no addresses"))
}

/// Expand ALL addresses in a CIDR, including network and broadcast for IPv4.
/// /31 and /32 therefore include both / one address(es) respectively.
fn expand_cidr(spec: &str, remaining: usize) -> Result<Vec<IpAddr>, String> {
    let (base, prefix) = spec
        .split_once('/')
        .ok_or_else(|| format!("invalid CIDR '{spec}'"))?;
    let ip: IpAddr = base
        .parse()
        .map_err(|_| format!("invalid CIDR '{spec}': expected an IP address before '/'"))?;
    let prefix: u8 = prefix
        .parse()
        .map_err(|_| format!("invalid CIDR '{spec}': invalid prefix length"))?;
    let max_bits = match ip {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };
    if prefix > max_bits {
        return Err(format!(
            "invalid CIDR '{spec}': prefix must be 0-{max_bits}"
        ));
    }
    let host_bits = max_bits - prefix;
    // Using a guard before the shift also avoids `1 << 128` overflow.
    if host_bits == 128 || (1u128 << host_bits) > remaining as u128 {
        return Err(format!(
            "CIDR '{spec}' exceeds the remaining target limit ({remaining}); use a smaller range or raise --max-hosts (maximum 65536)"
        ));
    }
    let size = 1usize << host_bits;
    let mut expanded = Vec::with_capacity(size);
    match ip {
        IpAddr::V4(ipv4) => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix)
            };
            let base = u32::from(ipv4) & mask;
            for offset in 0..size {
                expanded.push(IpAddr::V4(Ipv4Addr::from(base + offset as u32)));
            }
        }
        IpAddr::V6(ipv6) => {
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix)
            };
            let base = u128::from(ipv6) & mask;
            for offset in 0..size {
                expanded.push(IpAddr::V6(Ipv6Addr::from(base + offset as u128)));
            }
        }
    }
    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_ipv4_subnets_including_endpoints() {
        let ips = expand_cidr("192.0.2.3/30", 4).unwrap();
        let strings: Vec<_> = ips.iter().map(ToString::to_string).collect();
        assert_eq!(
            strings,
            ["192.0.2.0", "192.0.2.1", "192.0.2.2", "192.0.2.3"]
        );
        assert_eq!(expand_cidr("192.0.2.7/32", 1).unwrap().len(), 1);
        assert_eq!(expand_cidr("192.0.2.4/31", 2).unwrap().len(), 2);
    }

    #[test]
    fn expands_ipv6_ranges() {
        let ips = expand_cidr("2001:db8::3/126", 4).unwrap();
        assert_eq!(ips.len(), 4);
        assert_eq!(ips[0].to_string(), "2001:db8::");
        assert_eq!(ips[3].to_string(), "2001:db8::3");
        assert_eq!(expand_cidr("::1/128", 1).unwrap().len(), 1);
    }

    #[test]
    fn rejects_invalid_or_oversized_cidr() {
        for spec in ["192.0.2.1/33", "::1/129", "not-ip/24", "192.0.2.1/abc"] {
            assert!(expand_cidr(spec, 4096).is_err(), "{spec}");
        }
        assert!(expand_cidr("10.0.0.0/8", 4096).is_err());
        assert!(expand_cidr("2001:db8::/64", 65536).is_err());
        assert!(expand_cidr("0.0.0.0/0", 4096).is_err());
        assert!(expand_cidr("::/0", 4096).is_err());
    }

    #[tokio::test]
    async fn reads_targets_file_and_ignores_comments() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("nmor-hosts-{}-{nonce}.txt", std::process::id()));
        std::fs::write(
            &path,
            "# local hosts\n127.0.0.1 127.0.0.2 # comment\n127.0.0.1\n",
        )
        .unwrap();
        let found = expand_targets(&[], Some(&path), 3).await;
        std::fs::remove_file(&path).unwrap();
        let found = found.unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].ip.to_string(), "127.0.0.1");
        assert_eq!(found[1].ip.to_string(), "127.0.0.2");
    }

    #[tokio::test]
    async fn permits_overlap_at_exact_target_limit() {
        let names = ["127.0.0.1".into(), "127.0.0.0/30".into()];
        let targets = expand_targets(&names, None, 4).await.unwrap();
        assert_eq!(targets.len(), 4);
    }

    #[tokio::test]
    async fn deduplicates_overlapping_targets() {
        let names = [
            "127.0.0.1/31".into(),
            "127.0.0.1".into(),
            "127.0.0.0".into(),
        ];
        let targets = expand_targets(&names, None, 4).await.unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].ip.to_string(), "127.0.0.0");
        assert_eq!(targets[1].ip.to_string(), "127.0.0.1");
    }
}
