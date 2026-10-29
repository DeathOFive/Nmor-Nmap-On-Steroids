//! End-to-end tests run the built Nmor binary against the local loopback only.
use std::net::TcpListener;
use std::process::Command;

#[test]
fn scan_multiple_duplicate_targets_only_once() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_nmor"))
        .args(["-sT", "-Pn", "-p", &port, "127.0.0.1", "127.0.0.1/32"])
        .output()
        .expect("run Nmor");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Targets: 1"), "{stdout}");
    assert!(stdout.contains(&format!("{port}/tcp")), "{stdout}");
    assert!(stdout.contains("1 open"), "{stdout}");
}

#[test]
fn discovery_only_with_explicit_tcp_probe_finds_listener() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_nmor"))
        .args(["-sn", "--discovery-ports", &port, "127.0.0.1"])
        .output()
        .expect("run Nmor");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Host responded to TCP discovery probe"),
        "{stdout}"
    );
    assert!(stdout.contains("0 port-scanned"), "{stdout}");
}

#[test]
fn rejects_oversized_range_before_any_scans() {
    let output = Command::new(env!("CARGO_BIN_EXE_nmor"))
        .args(["-sn", "10.0.0.0/8"])
        .output()
        .expect("run Nmor");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--max-hosts"));
}
