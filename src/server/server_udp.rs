use std::io::Result;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use log::{debug, info};

// UDP source addresses can be forged. We want to limit the packet
// size that we reflect.
const MAX_PACKET_SZ: usize = 2048;

fn udp_server_receive_loop(socket: UdpSocket, shutdown: Arc<AtomicBool>) -> Result<()> {
    let mut buf = [0; MAX_PACKET_SZ];

    while !shutdown.load(Ordering::SeqCst) {
        let (size, from_addr) = match socket.recv_from(&mut buf) {
            Ok(result) => result,
            Err(err) => {
                // If we've already been requested to shut down, the error unblocking
                // the listener was desired behavior.
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }
                return Err(err);
            }
        };

        let mut message = String::from_utf8_lossy(&buf[0..size]).to_string();
        if !message.ends_with('\n') {
            debug!("\nAdding newline to echo");
            message.push('\n');
        }
        debug!(
            "from: {} UDP, message: {}",
            from_addr,
            &message[0..message.len() - 1]
        );
        let amt = socket.send_to(message.as_bytes(), from_addr)?;
        debug!("sent {} bytes", amt);
    }

    Ok(())
}

pub fn run_udp_server(bind_addr: &SocketAddr, shutdown: Arc<AtomicBool>) -> Result<()> {
    let socket = UdpSocket::bind(bind_addr)?;
    info!("starting UDP server on {}", socket.local_addr()?);
    udp_server_receive_loop(socket, shutdown)
}

#[cfg(test)]
mod tests {
    use std::net::{SocketAddr, UdpSocket};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::thread;

    use crate::server_udp::{run_udp_server, udp_server_receive_loop};

    /// Returns a localhost SocketAddr on a free UDP port. OSes won't
    /// immediately recycle port numbers for security reasons when requesting an
    /// OS assigned port, so it's a safe-enough way to get a free port even when
    /// running unit tests in parallel.
    fn get_free_udp_addr() -> SocketAddr {
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    }

    /// Starts the servers, but immediately shuts them down
    #[test]
    fn test_run_udp_server_immediate_shutdown() {
        let socket = get_free_udp_addr();
        let shutdown = Arc::new(AtomicBool::new(true));
        run_udp_server(&socket, shutdown).unwrap()
    }

    #[test]
    fn test_udp_server_receive_loop() {
        let server_addr = get_free_udp_addr();
        let server_sock = UdpSocket::bind(server_addr).unwrap();
        let server_sock_clone = server_sock.try_clone().unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        let handler = thread::spawn(move || {
            udp_server_receive_loop(server_sock_clone, shutdown_clone).unwrap()
        });

        let client_sock = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();

        let message = "Hello\n".to_string();

        let bytes_sent = client_sock
            .send_to(message.as_bytes(), server_addr)
            .unwrap();
        assert_eq!(bytes_sent, message.len());

        let mut buf = [0; 10];
        let (echo_size, from) = client_sock.recv_from(&mut buf).unwrap();
        assert_eq!(echo_size, bytes_sent);
        assert_eq!(from, server_addr);

        shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
        server_sock.set_nonblocking(true).unwrap();

        handler.join().unwrap();
    }
}
