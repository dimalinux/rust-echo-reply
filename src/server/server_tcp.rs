use std::{
    fmt::Debug,
    io::{Read, Result, Write},
    net::SocketAddr,
};

use log::{info, warn};
use threadpool::ThreadPool;
use tokio::{net::TcpListener, select};
use tokio_util::sync::CancellationToken;

// max number of TCP clients that we will serve simultaneously
const MAX_TCP_CLIENTS: usize = 100;

fn handle_tcp_client<R: Read, W: Write + Debug>(
    mut reader: R,
    writer: &mut W,
    peer_name: &String,
    run_state: &CancellationToken,
) -> Result<()> {
    let mut buf = [0u8; 4096];
    loop {
        // Check if we've been asked to shut down
        if run_state.is_cancelled() {
            info!("Shutting down connection with {peer_name} due to server shutdown");
            return Ok(());
        }

        let size = match reader.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => n,
            Err(err) => {
                // If the read timed out, loop back to check cancellation
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut
                {
                    continue;
                }
                return Err(err);
            }
        };
        info!("received: {peer_name} TCP, bytes: {size}");
        writer.write_all(&buf[..size])?;
        writer.flush()?;
        info!("sent: {peer_name} TCP, bytes: {size}");
    }
}

async fn handle_tcp_client_connections(
    listener: &TcpListener,
    run_state: CancellationToken,
) -> Result<()> {
    let pool = ThreadPool::new(MAX_TCP_CLIENTS);

    loop {
        let accept_result = select! {
            biased;
            () = run_state.cancelled() => {
                info!("Shutting down TCP server");
                pool.join();
                return Ok(());
            },
            result = listener.accept() => result,
        };

        match accept_result {
            Ok((socket, peer)) => {
                let peer_name = peer.to_string();
                let run_state_clone = run_state.clone();
                info!("Accepted connection from {peer}");
                pool.execute(move || {
                    // TODO: look into removing the unwrap calls
                    let mut socket = socket.into_std().unwrap();
                    socket.set_nonblocking(false).unwrap();
                    // Set a read timeout so we can periodically check for shutdown
                    if let Err(e) = socket.set_read_timeout(Some(std::time::Duration::from_secs(1)))
                    {
                        warn!("Failed to set read timeout for {peer_name}: {e}");
                        return;
                    }
                    match handle_tcp_client(
                        socket.try_clone().unwrap(),
                        &mut socket,
                        &peer_name,
                        &run_state_clone,
                    ) {
                        Ok(()) => {
                            info!("Closed connection with {peer_name}");
                        }
                        Err(err) => {
                            warn!("Closed connection with {peer_name} on error: {err}");
                        }
                    }
                });
            }
            Err(e) => {
                warn!("client accept error: {e}");
            }
        }
    }
}

pub async fn run_tcp_server(bind_addr: &SocketAddr, run_state: CancellationToken) -> Result<()> {
    let socket = TcpListener::bind(bind_addr).await?;
    info!("starting TCP server on {}", socket.local_addr()?);
    handle_tcp_client_connections(&socket, run_state).await
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufWriter, Cursor, Write},
        net,
        net::{Shutdown, SocketAddr},
        string::String,
        time::Duration,
    };

    use tokio::net::{TcpListener, TcpStream};
    use tokio_util::sync::CancellationToken;

    use crate::{
        init_logging,
        server_tcp::{handle_tcp_client, handle_tcp_client_connections, run_tcp_server},
    };

    #[ctor::ctor]
    fn init() {
        init_logging();
    }

    /// Returns a localhost `SocketAddr` on a free TCP port. OSes won't
    /// immediately recycle port numbers for security reasons when requesting an
    /// OS assigned port, so it's a safe-enough way to get a free port even when
    /// running unit tests in parallel.
    fn get_free_tcp_addr() -> SocketAddr {
        let listener = net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    }

    #[tokio::test]
    async fn test_handle_tcp_client() {
        let input = "line1\nline2";
        let reader = Cursor::new(input.as_bytes().to_vec());
        let mut writer = BufWriter::new(Vec::new());
        let peer_name = std::string::String::from("127.0.0.1:1024");
        let run_state = CancellationToken::new();
        handle_tcp_client(reader, &mut writer, &peer_name, &run_state).unwrap();
        let output = String::from_utf8(writer.into_inner().unwrap()).unwrap();
        let expected_output = input.to_string();
        assert_eq!(expected_output, output);
    }

    #[tokio::test]
    async fn test_handle_tcp_client_binary_input() {
        // Verify that arbitrary binary data is echoed unchanged.
        let input: &[u8] = &[0x00, 0xFF, 0x10, 0x20];
        let reader = Cursor::new(input.to_vec());
        let mut writer = BufWriter::new(Vec::new());
        let peer_name = String::from("127.0.0.1:1024");
        let run_state = CancellationToken::new();
        handle_tcp_client(reader, &mut writer, &peer_name, &run_state).unwrap();
        let output = writer.into_inner().unwrap();
        assert_eq!(input, output.as_slice());
    }

    #[tokio::test]
    async fn test_handle_tcp_client_connections() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let run_state = CancellationToken::new();
        let run_state_clone = run_state.clone();
        let handler = tokio::spawn(async move {
            handle_tcp_client_connections(&listener, run_state_clone)
                .await
                .unwrap();
        });
        tokio::time::sleep(Duration::from_millis(1000)).await;

        let conn = TcpStream::connect(addr).await.unwrap();
        let mut raw_conn = conn.into_std().unwrap();
        raw_conn.set_nonblocking(false).unwrap();
        let input = String::from("test\n");
        raw_conn.write_all(input.as_bytes()).unwrap();
        raw_conn.shutdown(Shutdown::Write).unwrap();
        let mut reader = std::io::BufReader::new(raw_conn);
        tokio::task::spawn_blocking(move || {
            let mut echo = String::new();
            reader.read_line(&mut echo).unwrap();
            assert_eq!(input, echo);
        })
        .await
        .unwrap();
        run_state.cancel();
        handler.await.unwrap();
    }

    #[tokio::test]
    async fn test_handle_tcp_client_connections_server_start_cancelled() {
        let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let run_state = CancellationToken::new();
        run_state.cancel();
        // call below should immediately return with no error
        handle_tcp_client_connections(&socket, run_state)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_run_tcp_server() {
        let server_addr = get_free_tcp_addr();
        let run_state = CancellationToken::new();
        let run_state_clone = run_state.clone();
        let tcp_server_task = tokio::spawn(async move {
            run_tcp_server(&server_addr, run_state_clone).await.unwrap();
        });

        // let the server start
        tokio::time::sleep(Duration::from_millis(1000)).await;
        run_state.cancel();

        tcp_server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_run_tcp_server_port_in_use() {
        // Occupy a random port and then pass its address to run_tcp_server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_addr = listener.local_addr().unwrap();
        let run_state = CancellationToken::new();
        let err = run_tcp_server(&server_addr, run_state).await.err().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);
    }

    #[tokio::test]
    async fn test_tcp_server_shutdown_with_active_connection() {
        // This test verifies that the TCP server shuts down properly even when
        // there's an active client connection that's idle.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let run_state = CancellationToken::new();
        let run_state_clone = run_state.clone();

        let handler = tokio::spawn(async move {
            handle_tcp_client_connections(&listener, run_state_clone)
                .await
                .unwrap();
        });

        // Give the server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect a client but don't send any data
        let conn = TcpStream::connect(addr).await.unwrap();
        let _raw_conn = conn.into_std().unwrap();

        // Give the connection time to be accepted
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Now cancel the server
        run_state.cancel();

        // The server should shut down within a reasonable time (2 seconds max)
        // even though the client is still connected but idle
        let result = tokio::time::timeout(Duration::from_secs(2), handler).await;

        assert!(
            result.is_ok(),
            "Server should shut down within 2 seconds even with idle connection"
        );
        assert!(result.unwrap().is_ok(), "Server should shut down cleanly");
    }
}
