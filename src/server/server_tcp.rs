use std::fmt::Debug;
use std::io::{BufRead, Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use threadpool::ThreadPool;

// max number of TCP clients that we will serve simultaneously
const MAX_TCP_CLIENTS: usize = 100;

fn handle_tcp_client<R: Read, W: Write + Debug>(reader: R, writer: &mut W, peer_name: &String) {
    // Add read timeout (5 minutes?)
    let mut reader = std::io::BufReader::new(reader);

    loop {
        let mut line = String::new();
        let size = match reader.read_line(&mut line) {
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

        println!(
            "from: {:?} TCP, sz: {} message: {:?}",
            peer_name, size, line
        );
        if !line.ends_with('\n') {
            println!("\nAdding newline to echo");
            line.push('\n');
        }
        writer.write_all(line.as_bytes()).unwrap(); // todo: add error handling
        writer.flush().unwrap();
        println!("sent {} bytes\n{:?}", line.len(), writer);
    }
}

fn handle_tcp_client_connections(
    listener: &mut TcpListener,
    shutdown: Arc<AtomicBool>,
) -> std::io::Result<()> {
    let pool = ThreadPool::new(MAX_TCP_CLIENTS);

    while !shutdown.load(Ordering::SeqCst) {
        let mut socket = match listener.accept() {
            Ok(result) => result,
            Err(err) => {
                // If we've already been requested to shutdown, the error unblocking
                // the listener was desired behavior, so we don't propagate it.
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }
                return Err(err);
            }
        };
        let peer_name = socket.0.peer_addr().unwrap().to_string();
        println!("Accepted connection from {:?}", peer_name);
        pool.execute(move || {
            handle_tcp_client(socket.0.try_clone().unwrap(), &mut socket.0, &peer_name);
            println!("Terminating connection with {}", peer_name);
        });
    }

    Ok(())
}

pub fn run_tcp_server(bind_addr: &SocketAddr) -> std::io::Result<()> {
    let mut socket = match TcpListener::bind(bind_addr) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Error binding socket: {}\n", err);
            return Err(err); // Or take some other recovery action
        }
    };

    // TODO: Global and have a signal handler change it?
    let shutdown = Arc::new(AtomicBool::new(false));
    println!("starting UDP server on {}", socket.local_addr()?);

    handle_tcp_client_connections(&mut socket, shutdown.clone())
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

    use crate::server_tcp::handle_tcp_client;
    use crate::server_tcp::handle_tcp_client_connections;

    #[test]
    fn test_handle_tcp_client() {
        let input = "line1\nline2\n";
        let reader = Cursor::new(input.as_bytes().to_vec());
        let mut writer = BufWriter::new(Vec::new());
        let peer_name = String::from("127.0.0.1:1024");
        handle_tcp_client(reader, &mut writer, &peer_name);
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
        let handler =
            thread::spawn(
                move || match handle_tcp_client_connections(&mut socket, shutdown2) {
                    Ok(result) => result,
                    Err(err) => {
                        panic!("{}", err);
                    }
                },
            );
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
