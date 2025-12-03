use std::{io::Result, net::SocketAddr};

use log::{debug, info};
use tokio::{net::UdpSocket, select};
use tokio_util::sync::CancellationToken;

// UDP source addresses can be forged. We want to limit the packet
// size that we reflect.
const MAX_PACKET_SZ: usize = 2048;

async fn udp_server_receive_loop(socket: UdpSocket, run_state: CancellationToken) -> Result<()> {
    let mut buf = [0; MAX_PACKET_SZ];

    loop {
        let recv_result = select! {
            biased;
            () = run_state.cancelled() => {
                // Shutdown message received, exit even if we are blocked on the
                // recv_from call below.
                return Ok(());
            },
            result = socket.recv_from(&mut buf) => result,
        };
        let (size, from_addr) = recv_result?;
        let mut message = String::from_utf8_lossy(&buf[0..size]).to_string();
        if !message.ends_with('\n') {
            debug!("Adding newline to echo");
            message.push('\n');
        }
        debug!(
            "from: {} UDP, message: {}",
            from_addr,
            &message[0..message.len() - 1]
        );
        let amt = socket.send_to(message.as_bytes(), from_addr).await?;
        debug!("sent {amt} bytes");
    }
}

pub async fn run_udp_server(bind_addr: &SocketAddr, run_state: CancellationToken) -> Result<()> {
    let socket = UdpSocket::bind(bind_addr).await?;
    info!("starting UDP server on {}", socket.local_addr()?);
    udp_server_receive_loop(socket, run_state).await
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use tokio::{net::UdpSocket, time::sleep};
    use tokio_util::sync::CancellationToken;

    use crate::server_udp::{run_udp_server, udp_server_receive_loop};

    /// Returns a localhost `SocketAddr` on a free UDP port. OSes won't
    /// immediately recycle port numbers for security reasons when requesting an
    /// OS assigned port, so it's a safe-enough way to get a free port even when
    /// running unit tests in parallel.
    fn get_free_udp_addr() -> SocketAddr {
        let listener = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    }

    /// Starts the servers, but immediately shuts them down
    #[tokio::test]
    async fn test_run_udp_server_immediate_shutdown() {
        let socket = get_free_udp_addr();
        let run_state = CancellationToken::new();
        run_state.cancel();
        run_udp_server(&socket, run_state).await.unwrap();
    }

    #[tokio::test]
    async fn test_udp_server_receive_loop() {
        let server_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_sock.local_addr().unwrap();
        let run_state = CancellationToken::new();

        let run_state_clone = run_state.clone();
        //let server_sock_clone = server_sock.try_clone().unwrap();
        let handler = tokio::spawn(async move {
            udp_server_receive_loop(server_sock, run_state_clone)
                .await
                .unwrap();
        });

        let client_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let message = "Hello\n".to_string();

        let bytes_sent = client_sock
            .send_to(message.as_bytes(), server_addr)
            .await
            .unwrap();
        assert_eq!(bytes_sent, message.len());

        let mut buf = [0; 10];
        let (echo_size, from) = client_sock.recv_from(&mut buf).await.unwrap();
        assert_eq!(echo_size, bytes_sent);
        assert_eq!(from, server_addr);

        sleep(tokio::time::Duration::from_millis(100)).await;
        run_state.cancel();

        handler.await.unwrap();
    }
}
