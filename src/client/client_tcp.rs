use std::io;
use std::io::{BufRead, Write};

fn tcp_client_loop(
    cli_input: &mut dyn BufRead,
    cli_output: &mut dyn Write,
    server_read: &mut dyn BufRead,
    server_write: &mut dyn Write,
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
