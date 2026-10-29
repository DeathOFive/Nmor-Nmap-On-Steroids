mod cli;
mod ports;
mod scanner;
mod targets;

use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::{StreamExt, stream};
use tokio::sync::Semaphore;

use crate::cli::Cli;
use crate::scanner::tcp::{self, PortState, ScanSummary};
use crate::targets::Target;

#[derive(Clone, Copy)]
struct Settings {
    timeout: Duration,
    scan_concurrency: usize,
    skip_discovery: bool,
    discovery_only: bool,
}

enum Outcome {
    NoTcpResponse,
    DiscoveryOnly,
    Scanned(ScanSummary),
}

struct HostReport {
    target: Target,
    outcome: Outcome,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Cli::parse_compat();

    if !(1..=4096).contains(&args.concurrency) {
        eprintln!("nmor: --concurrency must be in the range 1-4096");
        return ExitCode::from(2);
    }
    if !(1..=128).contains(&args.host_concurrency) {
        eprintln!("nmor: --host-concurrency must be in the range 1-128");
        return ExitCode::from(2);
    }
    if !(1..=65536).contains(&args.max_hosts) {
        eprintln!("nmor: --max-hosts must be in the range 1-65536");
        return ExitCode::from(2);
    }
    if args.timeout_ms == 0 {
        eprintln!("nmor: --timeout must be greater than zero");
        return ExitCode::from(2);
    }

    let ports = match ports::parse_ports(&args.ports) {
        Ok(ports) => ports,
        Err(error) => {
            eprintln!("nmor: invalid scan ports: {error}");
            return ExitCode::from(2);
        }
    };
    let discovery_ports = match ports::parse_ports(&args.discovery_ports) {
        Ok(ports) => ports,
        Err(error) => {
            eprintln!("nmor: invalid discovery ports: {error}");
            return ExitCode::from(2);
        }
    };

    let targets =
        match targets::expand_targets(&args.targets, args.input_file.as_deref(), args.max_hosts)
            .await
        {
            Ok(targets) => targets,
            Err(error) => {
                eprintln!("nmor: {error}");
                return ExitCode::from(2);
            }
        };

    let selected_hosts = targets.len();
    let settings = Settings {
        timeout: Duration::from_millis(args.timeout_ms),
        scan_concurrency: args.concurrency,
        skip_discovery: args.skip_discovery,
        discovery_only: args.discovery_only,
    };
    let sockets = Arc::new(Semaphore::new(args.concurrency));

    println!("Nmor v{} | TCP connect (-sT)", env!("CARGO_PKG_VERSION"));
    println!(
        "Targets: {selected_hosts} | scan ports per target: {}",
        ports.len()
    );
    if args.skip_discovery {
        println!("Discovery: skipped (-Pn); targets are assumed up");
    } else {
        println!(
            "Discovery: TCP connect probes on {} port(s); a refusal also proves a response",
            discovery_ports.len()
        );
    }
    if args.discovery_only {
        println!("Mode: discovery only (-sn); no port scans");
    }
    println!(
        "Global TCP concurrency: {} | host concurrency: {} | connect timeout: {}ms",
        args.concurrency, args.host_concurrency, args.timeout_ms
    );

    let started = Instant::now();
    let settings = &settings;
    let scan_ports = &ports;
    let probe_ports = &discovery_ports;
    let mut reports = stream::iter(targets)
        .map(|target| {
            let sockets = Arc::clone(&sockets);
            async move { evaluate_target(target, settings, scan_ports, probe_ports, sockets).await }
        })
        .buffer_unordered(args.host_concurrency);

    let mut answered = 0usize;
    let mut no_response = 0usize;
    let mut scanned = 0usize;
    let mut open = 0usize;
    let mut closed = 0usize;
    let mut filtered = 0usize;
    let mut errors = 0usize;

    while let Some(report) = reports.next().await {
        println!(
            "\nNmor report for {} ({})",
            report.target.label, report.target.ip
        );
        match report.outcome {
            Outcome::NoTcpResponse => {
                no_response += 1;
                println!("No TCP response to discovery probes; port scan skipped (try -Pn).");
            }
            Outcome::DiscoveryOnly => {
                if settings.skip_discovery {
                    println!("Host assumed up (-Pn; no actual discovery probe).");
                } else {
                    answered += 1;
                    println!("Host responded to TCP discovery probe.");
                }
            }
            Outcome::Scanned(summary) => {
                scanned += 1;
                if settings.skip_discovery {
                    println!("Discovery skipped (-Pn); scanning regardless of host status.");
                } else {
                    answered += 1;
                    println!("Host responded to TCP discovery probe.");
                }
                println!("PORT       STATE");
                for result in &summary.results {
                    if result.state == PortState::Open {
                        let name = format!("{}/tcp", result.port);
                        println!("{name:<10} open");
                    }
                    match result.state {
                        PortState::Open => open += 1,
                        PortState::Closed => closed += 1,
                        PortState::Filtered => filtered += 1,
                        PortState::Error => errors += 1,
                    }
                }
                if !summary
                    .results
                    .iter()
                    .any(|port| port.state == PortState::Open)
                {
                    println!("No open TCP ports found in selected set.");
                }
                println!(
                    "Scanned {} ports in {:.2?}",
                    summary.results.len(),
                    summary.elapsed
                );
            }
        }
    }

    println!(
        "\nNmor done: {selected_hosts} target(s), {answered} discovery-response, {no_response} no-response, {scanned} port-scanned in {:.2?}",
        started.elapsed()
    );
    if !settings.discovery_only {
        println!(
            "Port results: {open} open, {closed} closed, {filtered} timed out/filtered, {errors} other errors"
        );
    }
    if settings.skip_discovery {
        println!("Note: -Pn treats hosts as up but does not verify their availability.");
    } else if no_response > 0 {
        println!(
            "Note: no TCP discovery response does NOT prove a host is down; -Pn bypasses discovery."
        );
    }
    ExitCode::SUCCESS
}

async fn evaluate_target(
    target: Target,
    settings: &Settings,
    scan_ports: &[u16],
    probe_ports: &[u16],
    sockets: Arc<Semaphore>,
) -> HostReport {
    let responded = if settings.skip_discovery {
        true
    } else {
        scanner::discovery::responds_to_tcp(
            target.ip,
            probe_ports,
            settings.timeout,
            Arc::clone(&sockets),
        )
        .await
    };
    let outcome = if !responded {
        Outcome::NoTcpResponse
    } else if settings.discovery_only {
        Outcome::DiscoveryOnly
    } else {
        Outcome::Scanned(
            tcp::scan_with_limit(
                target.ip,
                scan_ports,
                settings.timeout,
                settings.scan_concurrency,
                sockets,
            )
            .await,
        )
    };
    HostReport { target, outcome }
}
