use std::fmt::Debug;
use std::io::{BufRead, Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use log::{debug, error, info, warn};
use threadpool::ThreadPool;

// max number of TCP clients that we will serve simultaneously
const MAX_TCP_CLIENTS: usize = 100;

fn handle_tcp_client<R: Read, W: Write + Debug>(
    reader: R,
    writer: &mut W,
    peer_name: &String,
) -> std::io::Result<()> {
    // TODO: Add read timeout (5 minutes?)
    let mut reader = std::io::BufReader::new(reader);

    loop {
        let mut line = String::new();
        let size = match reader.read_line(&mut line) {
            Ok(0) => return Ok(()),
            Ok(result) => result,
            Err(err) => return Err(err),
        };

        info!(
            "from: {:?} TCP, sz: {} message: {:?}",
            peer_name, size, line
        );
        if !line.ends_with('\n') {
            debug!("\nAdding newline to echo");
            line.push('\n');
        }
        writer.write_all(line.as_bytes())?;
        writer.flush()?;
        info!("sent {} bytes\n{:?}", line.len(), writer);
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
                // If we've already been requested to shut down, the error unblocking
                // the listener was desired behavior.
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }
                return Err(err);
            }
        };
        let peer_name = socket.0.peer_addr().unwrap().to_string();
        info!("Accepted connection from {:?}", peer_name);
        pool.execute(move || {
            match handle_tcp_client(socket.0.try_clone().unwrap(), &mut socket.0, &peer_name) {
                Ok(_) => {
                    info!("Closed connection with {}", peer_name);
                }
                Err(err) => {
                    warn!("Closed connection with {} on error: {}", peer_name, err)
                }
            }
        });
    }

    pool.join();
    Ok(())
}

pub fn run_tcp_server(bind_addr: &SocketAddr, shutdown: Arc<AtomicBool>) -> std::io::Result<()> {
    let mut socket = match TcpListener::bind(bind_addr) {
        Ok(result) => result,
        Err(err) => {
            error!("Error binding socket: {}\n", err);
            return Err(err); // Or take some other recovery action
        }
    };

    info!("starting UDP server on {}", socket.local_addr()?);

    handle_tcp_client_connections(&mut socket, shutdown.clone())
}

#[cfg(test)]
mod tests {
    use crate::init_logging;
    use crate::server_tcp::handle_tcp_client_connections;
    use crate::server_tcp::{handle_tcp_client, run_tcp_server};
    use std::io::{BufRead, BufWriter, Cursor, Write};
    use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
    use std::string::String;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[ctor::ctor]
    fn init() {
        init_logging();
    }

    /// Returns a localhost SocketAddr on a free TCP port. OSes won't
    /// immediately recycle port numbers for security reasons when requesting an
    /// OS assigned port, so it's a safe-enough way to get a free port even when
    /// running unit tests in parallel.
    fn get_free_tcp_addr() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    }

    #[test]
    fn test_handle_tcp_client() {
        let input = "line1\nline2";
        let reader = Cursor::new(input.as_bytes().to_vec());
        let mut writer = BufWriter::new(Vec::new());
        let peer_name = std::string::String::from("127.0.0.1:1024");
        handle_tcp_client(reader, &mut writer, &peer_name).unwrap();
        let output = String::from_utf8(writer.into_inner().unwrap()).unwrap();
        let expected_output = input.to_string() + "\n";
        assert_eq!(expected_output, output);
    }

    #[test]
    fn test_handle_tcp_client_non_utf8_input() {
        let invalid_utf8: &[u8] = &[0xC0, 0x80];
        let reader = Cursor::new(invalid_utf8);
        let mut writer = BufWriter::new(Vec::new());
        let peer_name = String::from("127.0.0.1:1024");
        let err = handle_tcp_client(reader, &mut writer, &peer_name).err();
        assert_eq!(err.unwrap().kind(), std::io::ErrorKind::InvalidData);
        let output = String::from_utf8(writer.into_inner().unwrap()).unwrap();
        assert_eq!("", output);
    }

    #[test]
    fn test_handle_tcp_client_connections() {
        let mut socket = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();
        let handler = thread::spawn(move || {
            handle_tcp_client_connections(&mut socket, shutdown_clone).unwrap()
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
        // server thread is blocked on an accept(), unblock it so the thread can be joined
        let _ = TcpStream::connect(addr).unwrap();
        handler.join().unwrap();
    }

    #[test]
    fn test_handle_tcp_client_connections_accept_error_propagated() {
        let mut socket = TcpListener::bind("127.0.0.1:0").unwrap();
        // accept on the socket will immediately error with EWOULDBLOCK
        socket.set_nonblocking(true).unwrap();

        // ensure that the accept error is propagated, as we haven't set the shutdown flag
        let shutdown = Arc::new(AtomicBool::new(false));
        let err = handle_tcp_client_connections(&mut socket, shutdown)
            .err()
            .unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
    }

    #[test]
    fn test_run_tcp_server() {
        let server_addr = get_free_tcp_addr();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();
        let tcp_thread = thread::spawn(move || {
            run_tcp_server(&server_addr, shutdown_clone).unwrap();
        });
        // let the server start
        thread::sleep(Duration::from_millis(1000));
        shutdown.store(true, Ordering::Relaxed);
        // server thread is blocked on an accept(), unblock it so the thread can be joined
        let _ = TcpStream::connect(server_addr).unwrap();

        tcp_thread.join().unwrap();
    }

    #[test]
    fn test_run_tcp_server_port_in_use() {
        // Occupy a random port and then pass its address to run_tcp_server
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let server_addr = listener.local_addr().unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));
        let err = run_tcp_server(&server_addr, shutdown).err().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);
    }
}
