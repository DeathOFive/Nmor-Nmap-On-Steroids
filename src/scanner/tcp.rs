use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::{StreamExt, stream};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortState {
    Open,
    Closed,
    Filtered,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub struct PortResult {
    pub port: u16,
    pub state: PortState,
}

pub struct ScanSummary {
    pub results: Vec<PortResult>,
    pub elapsed: Duration,
}

/// Existing API: cap a single target's concurrent TCP connects.
#[allow(dead_code)]
pub async fn scan(
    address: IpAddr,
    ports: &[u16],
    timeout_per_port: Duration,
    concurrency: usize,
) -> ScanSummary {
    scan_with_limit(
        address,
        ports,
        timeout_per_port,
        concurrency,
        Arc::new(Semaphore::new(concurrency)),
    )
    .await
}

/// Multi-host scanner: every host shares the same global socket semaphore.
/// `per_host_window` only limits queued futures for each host.
pub async fn scan_with_limit(
    address: IpAddr,
    ports: &[u16],
    timeout_per_port: Duration,
    per_host_window: usize,
    global_sockets: Arc<Semaphore>,
) -> ScanSummary {
    let started = Instant::now();
    let mut results = stream::iter(ports.iter().copied())
        .map(|port| {
            let sockets = Arc::clone(&global_sockets);
            async move {
                let address = SocketAddr::new(address, port);
                scan_one_limited(address, timeout_per_port, sockets).await
            }
        })
        .buffer_unordered(per_host_window)
        .collect::<Vec<_>>()
        .await;
    results.sort_unstable_by_key(|result| result.port);
    ScanSummary {
        results,
        elapsed: started.elapsed(),
    }
}

/// Take a shared socket permit before attempting a connection. Held only
/// during that attempt, so discovery and scanning share one global limit.
pub async fn scan_one_limited(
    address: SocketAddr,
    timeout_per_port: Duration,
    global_sockets: Arc<Semaphore>,
) -> PortResult {
    match global_sockets.acquire().await {
        Ok(_permit) => scan_one(address, timeout_per_port).await,
        Err(_) => PortResult {
            port: address.port(),
            state: PortState::Error,
        },
    }
}

pub async fn scan_one(address: SocketAddr, timeout_per_port: Duration) -> PortResult {
    let state = match timeout(timeout_per_port, TcpStream::connect(address)).await {
        Ok(Ok(_)) => PortState::Open,
        Ok(Err(err)) if err.kind() == ErrorKind::ConnectionRefused => PortState::Closed,
        Ok(Err(err)) if err.kind() == ErrorKind::TimedOut => PortState::Filtered,
        Ok(Err(_)) => PortState::Error,
        Err(_) => PortState::Filtered,
    };
    PortResult {
        port: address.port(),
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn detects_an_open_local_port() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let result = scan_one(address, Duration::from_millis(500)).await;
        assert_eq!(result.port, address.port());
        assert_eq!(result.state, PortState::Open);
    }

    #[tokio::test]
    async fn maintains_sorted_results() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let open_port = listener.local_addr().unwrap().port();
        let other_port = if open_port == 65535 {
            65534
        } else {
            open_port + 1
        };
        let result = scan(
            IpAddr::from([127, 0, 0, 1]),
            &[other_port, open_port],
            Duration::from_millis(500),
            2,
        )
        .await;
        assert!(result.results[0].port < result.results[1].port);
        assert!(
            result
                .results
                .iter()
                .any(|p| p.port == open_port && p.state == PortState::Open)
        );
    }

    #[tokio::test]
    async fn shared_limit_is_released_after_scan() {
        let sockets = Arc::new(Semaphore::new(1));
        let result = scan_with_limit(
            IpAddr::from([127, 0, 0, 1]),
            &[1, 2, 3],
            Duration::from_millis(100),
            3,
            Arc::clone(&sockets),
        )
        .await;
        assert_eq!(result.results.len(), 3);
        assert_eq!(sockets.available_permits(), 1);
    }
}
