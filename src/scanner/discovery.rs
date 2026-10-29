use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use futures::{StreamExt, stream};
use tokio::sync::Semaphore;

use super::tcp::{self, PortState};

/// TCP-based host discovery: an established connection OR a TCP refusal/RST
/// means an endpoint responded. Timeouts and other errors are inconclusive.
/// This is not raw SYN, ICMP, or Nmap-equivalent host discovery.
pub async fn responds_to_tcp(
    address: IpAddr,
    ports: &[u16],
    per_probe_timeout: Duration,
    global_sockets: Arc<Semaphore>,
) -> bool {
    stream::iter(ports.iter().copied())
        .map(|port| {
            let sockets = Arc::clone(&global_sockets);
            async move {
                let result = tcp::scan_one_limited(
                    SocketAddr::new(address, port),
                    per_probe_timeout,
                    sockets,
                )
                .await;
                matches!(result.state, PortState::Open | PortState::Closed)
            }
        })
        .buffer_unordered(ports.len().clamp(1, 8))
        .any(|responded| async move { responded })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn recognizes_local_tcp_responses() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        assert!(
            responds_to_tcp(
                address.ip(),
                &[address.port()],
                Duration::from_millis(500),
                Arc::new(Semaphore::new(2)),
            )
            .await
        );
    }
}
