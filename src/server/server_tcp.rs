use std::fmt::Debug;
use std::io::{BufRead, Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn handle_tcp_client<R: Read, W: Write + Debug>(reader: R, writer: &mut W, peer_name: &String) {
    // Add read timeout (5 minutes?)
    let mut reader = std::io::BufReader::new(reader);

    loop {
        let mut line = String::new();
        let len = match reader.read_line(&mut line) {
            Ok(0) => {
                println!("Client {} closed connection", peer_name);
                break;
            }
            Ok(result) => result,
            Err(err) => {
                eprintln!("Error kind is {}\n", err.kind());
                eprintln!("Error receiving data from client: {}\n", err);
                break;
            }
        };

        println!("From: {}, str-size={} byte-size={} read-ln-size={} ends-with-newline={} message:\n{:?}", peer_name,
                 line.len(), line.as_bytes().len(), len, line.ends_with("\n"), line);
        if !line.ends_with("\n") {
            println!("Adding newline to echo");
            line.push_str("\n");
        }
        writer.write_all(line.as_bytes()).unwrap(); // todo: add error handling
        writer.flush().unwrap();
        println!("sent {} bytes\n{:?}", line.len(), writer);
    }
}

fn event_loop_tcp(listener: &mut TcpListener, shutdown: Arc<AtomicBool>) -> std::io::Result<()> {
    while !shutdown.load(Ordering::SeqCst) {
        let mut socket = match listener.accept() {
            Ok(result) => result,
            Err(err) => {
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }
                return Err(err);
            }
        };
        let peer_name = socket.0.peer_addr().unwrap().to_string();
        println!("Accepted connection from {:?}", peer_name);
        handle_tcp_client(socket.0.try_clone()?, &mut socket.0, &peer_name);
        println!("Terminating connection with {:?}", peer_name);
    }

    // TODO: Print some message here
    return Ok(());
}

pub fn run_tcp(bind_addr: SocketAddr) -> std::io::Result<()> {
    let mut socket = match TcpListener::bind(bind_addr) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Error binding socket: {}\n", err);
            return Err(err); // Or take some other recovery action
        }
    };

    // TODO: Global and have a signal handler change it?
    let shutdown = Arc::new(AtomicBool::new(false));
    println!("\nstart server\n");

    event_loop_tcp(&mut socket, shutdown.clone())
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufWriter, Cursor, Write};
    use std::net::{Shutdown, TcpListener, TcpStream};
    use std::os::fd::{AsFd, AsRawFd};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use crate::server_tcp::event_loop_tcp;
    use crate::server_tcp::handle_tcp_client;

    #[test]
    fn test_handle_tcp_client() {
        let input = "line1\nline2\n";
        let reader = Cursor::new(input.as_bytes().to_vec());
        let mut writer = BufWriter::new(Vec::new());
        let peer_name = String::from("127.0.0.1:1024");
        handle_tcp_client(reader, &mut writer, &peer_name);
        println!("XXX {:?} XXX", writer);
        let output = String::from_utf8(writer.into_inner().unwrap()).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn test_event_loop_tcp() {
        let mut socket = TcpListener::bind("127.0.0.1:0").unwrap();
        let socket_fd = socket.as_fd().as_raw_fd();
        let addr = socket.local_addr().unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown2 = shutdown.clone();
        let handler = thread::spawn(move || match event_loop_tcp(&mut socket, shutdown2) {
            Ok(result) => result,
            Err(err) => {
                panic!("{}", err);
            }
        });
        thread::sleep(Duration::from_millis(1000));

        let mut conn = TcpStream::connect(addr).unwrap();
        let input = String::from("test\n");
        conn.write_all(input.as_bytes()).unwrap();
        conn.shutdown(Shutdown::Write).unwrap();
        let mut echo = String::new();
        let mut reader = std::io::BufReader::new(conn);
        reader.read_line(&mut echo).unwrap();
        assert_eq!(input, echo);
        shutdown.store(true, Ordering::Relaxed);
        nix::unistd::close(socket_fd).unwrap();
        handler.join().unwrap();
    }
}
