use std::{
    io::{BufRead, Error, ErrorKind::Other, Result, Write},
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

use crate::{CLIENT_ADDR, MAX_BUF_SZ};

fn udp_client_loop(
    cli_input: &mut dyn BufRead,
    cli_output: &mut dyn Write,
    client_sock: &UdpSocket,
    server_addr: SocketAddr,
) -> Result<()> {
    println!("Echo destination: {server_addr} UDP");
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
                    return Err(Error::new(Other, format!("no response from {server_addr}")));
                }
                return Err(err);
            }
        };

        let mut echo = String::from_utf8_lossy(&buf[..echo_size]).to_string();
        if !echo.ends_with('\n') {
            _ = cli_output.write(b"NEWLINE ADDED\n")?;
            echo.push('\n');
        }

        // Only include the peer address in the output if the message came from
        // an unexpected peer.
        let mut peer_name = String::new();
        if from != server_addr {
            peer_name = from.to_string();
            peer_name.push(' ');
        }
        cli_output.write_fmt(format_args!("ECHO: {peer_name}{echo}"))?;
    }
    Ok(())
}

pub fn run_udp_client(
    user_input: &mut dyn BufRead,
    user_output: &mut dyn Write,
    server_addr: SocketAddr,
) -> Result<()> {
    // Get a client socket to send from on a random UDP port
    let socket = std::net::UdpSocket::bind(CLIENT_ADDR.to_string())?;
    udp_client_loop(user_input, user_output, &socket, server_addr)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufReader, BufWriter, Cursor},
        net::UdpSocket,
        string::String,
        thread,
    };

    use super::*;

    #[test]
    fn test_run_udp_client() {
        let server_sock = UdpSocket::bind("127.0.0.1:0").unwrap();
        let server_addr = server_sock.local_addr().unwrap();

        // Create a second server socket to test printing additional information
        // when the echo comes from an address other than the one we sent to.
        let server_sock2 = UdpSocket::bind("127.0.0.1:0").unwrap();
        let server_addr2 = server_sock2.local_addr().unwrap();

        let mut user_input = BufReader::new(Cursor::new(b"client1\nclient2".to_vec()));
        let mut user_output = BufWriter::new(Vec::new());

        let handler = thread::spawn(move || {
            let mut buf = [0; 1024];
            let (len, from) = server_sock.recv_from(&mut buf).unwrap();
            assert_eq!(
                "client1\n",
                String::from_utf8_lossy(&buf[..len]).to_string()
            );
            server_sock.send_to(b"server1\n", from).unwrap();
            let (len, from) = server_sock.recv_from(&mut buf).unwrap();
            assert_eq!(
                "client2\n",
                String::from_utf8_lossy(&buf[..len]).to_string()
            );
            // send the 2nd echo response from a different address
            server_sock2.send_to(b"server2", from).unwrap();
        });

        run_udp_client(&mut user_input, &mut user_output, server_addr).unwrap();
        let expected_output = format!(
            "ECHO: server1\nNEWLINE ADDED\nECHO: 127.0.0.1:{} server2\n",
            server_addr2.port()
        );
        assert_eq!(
            expected_output,
            String::from_utf8(user_output.into_inner().unwrap()).unwrap(),
        );
        handler.join().unwrap();
    }

    #[test]
    fn test_run_udp_client_no_response() {
        // Get an unused local address. We'll send an message to it, but there won't
        // be any server to respond.
        let server_addr = UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();

        let mut user_input = BufReader::new(Cursor::new(b"client1\n".to_vec()));
        let mut user_output = BufWriter::new(Vec::new());

        let err = run_udp_client(&mut user_input, &mut user_output, server_addr).unwrap_err();
        assert_eq!(err.to_string(), format!("no response from {server_addr}"));
    }
}
