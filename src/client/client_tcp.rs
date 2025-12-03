use std::{
    io,
    io::{BufRead, Write},
    net::{SocketAddr, TcpStream},
};

fn tcp_client_loop(
    user_input: &mut dyn BufRead,  // reads command line user input
    user_output: &mut dyn Write,   // writes to the user's terminal
    server_read: &mut dyn BufRead, // reads echos from the server
    server_write: &mut dyn Write,  // writes a message to the server to be echoed
) -> io::Result<()> {
    loop {
        let mut user_line = String::new();
        let n = user_input.read_line(&mut user_line)?;
        if n == 0 {
            break;
        }
        if !user_line.ends_with('\n') {
            println!("\n[Adding newline to echo]");
            user_line.push('\n');
        }

        server_write.write_all(user_line.as_bytes())?;
        server_write.flush().unwrap();

        let mut echo_line = String::new();
        _ = server_read.read_line(&mut echo_line)?;

        if !echo_line.ends_with('\n') {
            println!("\n[Adding newline to echo]");
            echo_line.push('\n');
        }

        user_output.write_all(format!("ECHO: {echo_line}").as_bytes())?;
    }
    Ok(())
}

pub fn run_tcp_client(
    user_input: &mut dyn BufRead,
    user_output: &mut dyn Write,
    server_addr: SocketAddr,
) -> io::Result<()> {
    // Get a client socket to send from on a random UDP port
    let mut conn = match TcpStream::connect(server_addr) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Error connecting to {server_addr:?}: {err}\n");
            return Err(err); // Or take some other recovery action
        }
    };

    let peer_addr = conn.peer_addr()?;
    let unbuf_reader = conn.try_clone()?;
    let mut reader = io::BufReader::new(unbuf_reader);

    println!("Connected to {peer_addr} TCP");
    println!("Enter text, newlines separate echo messages, control-d to quit.");
    tcp_client_loop(user_input, user_output, &mut reader, &mut conn)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufReader, BufWriter, Cursor, ErrorKind::ConnectionRefused},
        net,
        net::TcpListener,
        thread,
    };

    use super::*;

    #[test]
    fn test_tcp_client_loop() {
        let mut user_input = BufReader::new(Cursor::new(b"client1\nclient2".to_vec()));
        let mut user_output = BufWriter::new(Vec::new());
        let mut server_read = BufReader::new(Cursor::new(b"server1\nserver2".to_vec()));
        let mut server_write = BufWriter::new(Vec::new());
        tcp_client_loop(
            &mut user_input,
            &mut user_output,
            &mut server_read,
            &mut server_write,
        )
        .unwrap();
        let cli_output = String::from_utf8(user_output.into_inner().unwrap()).unwrap();
        // While a real server echos would echo what the client sends, the client echos what the server
        // even if it does not match.
        assert_eq!("ECHO: server1\nECHO: server2\n", cli_output);
    }

    #[test]
    fn test_run_tcp_client() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let server_addr = listener.local_addr().unwrap();
        let handler = thread::spawn(move || {
            let (conn, _) = listener.accept().unwrap();
            conn.shutdown(net::Shutdown::Both).unwrap();
            drop(conn);
            drop(listener);
        });

        let mut user_input = BufReader::new(Cursor::new(b"".to_vec()));
        let mut user_output = BufWriter::new(Vec::new());

        run_tcp_client(&mut user_input, &mut user_output, server_addr).unwrap();
        handler.join().unwrap();
    }

    #[test]
    fn test_run_tcp_client_error() {
        // Get a free TCP port that no one will be listening on
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let server_addr = listener.local_addr().unwrap();
        drop(listener);

        let mut user_input = BufReader::new(Cursor::new(b"".to_vec()));
        let mut user_output = BufWriter::new(Vec::new());

        let err = run_tcp_client(&mut user_input, &mut user_output, server_addr).unwrap_err();
        assert_eq!(err.kind(), ConnectionRefused);
    }
}
