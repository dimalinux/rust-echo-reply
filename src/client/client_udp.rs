use std::io;
use std::io::{BufRead, Write};
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use crate::{CLIENT_ADDR, MAX_BUF_SZ};

fn udp_client_loop(
    cli_input: &mut dyn BufRead,
    cli_output: &mut dyn Write,
    client_sock: UdpSocket,
    server_addr: SocketAddr,
) -> io::Result<()> {
    println!("Echo destination: {} UDP", server_addr);
    println!("Enter text, newlines separate echo messages, control-d to quit.");

    loop {
        let mut message = String::new();
        let size = cli_input.read_line(&mut message)?;
        if size == 0 {
            break;
        }

        if !message.ends_with('\n') {
            println!("\nAdding newline to outbound echo");
            message.push('\n');
        }

        let _ = client_sock.send_to(message.as_bytes(), server_addr)?;
        message.clear();

        client_sock
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("Could not set a read timeout");

        let mut buf = [0; MAX_BUF_SZ];
        let (echo_size, from) = match client_sock.recv_from(&mut buf) {
            Ok(result) => result,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "no response from server",
                    ));
                }
                return Err(err); // Or take some other recovery action
            }
        };
        let mut echo = String::from_utf8_lossy(&buf[..echo_size]).to_string();
        if !echo.ends_with('\n') {
            echo.push_str("\nNEWLINE ADDED\n");
        }

        // Only include the peer address in the output if the message came from
        // an unexpected peer.
        let mut peer_name = String::new();
        if from != server_addr {
            peer_name = from.to_string();
            peer_name.push(' ');
        }
        cli_output.write_fmt(format_args!("ECHO: {}{}", peer_name, echo))?;
    }
    Ok(())
}

pub fn run_udp_client(server_addr: std::net::SocketAddr) -> std::io::Result<()> {
    // Get a client socket to send from on a random UDP port
    let socket = std::net::UdpSocket::bind(CLIENT_ADDR.to_string())?;
    udp_client_loop(
        &mut io::stdin().lock(),
        &mut io::stdout(),
        socket,
        server_addr,
    )
}
