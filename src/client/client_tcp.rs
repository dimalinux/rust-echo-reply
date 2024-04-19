use std::io;
use std::io::{BufRead, Write};

fn tcp_client_loop(
    cli_input: &mut dyn BufRead,   // reads command line user input
    cli_output: &mut dyn Write,    // writes to the user's terminal
    server_read: &mut dyn BufRead, // reads echos from the server
    server_write: &mut dyn Write,  // writes a message to the server to be echoed
) -> io::Result<()> {
    let mut line = String::new();
    loop {
        let n = cli_input.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        if !line.ends_with('\n') {
            println!("\nAdding newline to echo");
            line.push('\n');
        }

        server_write.write_all(line.as_bytes())?;
        server_write.flush().unwrap();
        line.clear();

        match server_read.read_line(&mut line) {
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

        if !line.ends_with('\n') {
            println!("\nAdding newline to echo");
            line.push('\n');
        }

        cli_output.write_all(format!("ECHO: {}", line).as_bytes())?;
        line.clear();
    }
    Ok(())
}

pub fn run_tcp_client(server_addr: std::net::SocketAddr) -> io::Result<()> {
    // Get a client socket to send from on a random UDP port
    let mut conn = match std::net::TcpStream::connect(server_addr) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Error kind is {}\n", err.kind());
            eprintln!("Error connecting to {:?}: {}\n", server_addr, err);
            return Err(err); // Or take some other recovery action
        }
    };

    let peer_addr = conn.peer_addr()?;
    let unbuf_reader = conn.try_clone()?;
    let mut reader = io::BufReader::new(unbuf_reader);

    println!("Connected to {} TCP", peer_addr);
    println!("Enter text, newlines separate echo messages, control-d to quit.");
    tcp_client_loop(
        &mut io::stdin().lock(),
        &mut io::stdout(),
        &mut reader,
        &mut conn,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, BufWriter, Cursor};
    use std::net;

    #[test]
    fn test_tcp_client_loop() {
        let mut cli_input = BufReader::new(Cursor::new("client1\nclient2\n".as_bytes().to_vec()));
        let mut cli_output = BufWriter::new(Vec::new());
        let mut server_read = BufReader::new(Cursor::new("server1\nserver2\n".as_bytes().to_vec()));
        let mut server_write = BufWriter::new(Vec::new());
        tcp_client_loop(
            &mut cli_input,
            &mut cli_output,
            &mut server_read,
            &mut server_write,
        )
        .unwrap();
        let cli_output = String::from_utf8(cli_output.into_inner().unwrap()).unwrap();
        // While a real server echos would echo what the client sends, the client echos what the server
        // even if it does not match.
        assert_eq!("ECHO: server1\nECHO: server2\n", cli_output);
    }

    #[test]
    fn test_run_tcp_client_error() {
        // Get a free TCP port that no one will be listening on
        let listener = net::TcpListener::bind("127.0.0.1:0").unwrap();
        let server_addr = listener.local_addr().unwrap();
        drop(listener);

        let err = run_tcp_client(server_addr).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::ConnectionRefused);
    }
}
