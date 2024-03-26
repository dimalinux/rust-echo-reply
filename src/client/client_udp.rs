use std::time::Duration;
use crate::{CLIENT_ADDR, MAX_BUF_SZ};

pub fn run_udp_client(server_addr: std::net::SocketAddr, message: String) -> std::io::Result<()> {
    // Get a client socket to send from on a random UDP port
    let socket = std::net::UdpSocket::bind(CLIENT_ADDR.to_string())?;

    let message_sz = socket.send_to(message.as_bytes(), server_addr)?;
    println!("\nsent echo of {} bytes to server\n", message_sz);

    let mut buf = [0; MAX_BUF_SZ];
    socket
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("Could not set a read timeout");
    let (amt, src) = match socket.recv_from(&mut buf) {
        Ok(result) => result,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::WouldBlock {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "no response from server",
                ));
            }
            // Handle the error here
            eprintln!("Error kind is {}\n", err.kind());
            eprintln!("Error receiving data from socket: {}\n", err);
            return Err(err); // Or take some other recovery action
        }
    };
    let echo = std::str::from_utf8(&buf[..amt]).unwrap();
    println!("Echo from: {:?}, size: {:?}\n{}\n", src, amt, echo);
    Ok(())
}
