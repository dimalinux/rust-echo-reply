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
) -> Result<()> {
    // TODO: Add a timeout to close idle connections?
    let mut buf = [0u8; 4096];
    loop {
        let size = match reader.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => n,
            Err(err) => return Err(err),
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
                info!("Accepted connection from {peer}");
                pool.execute(move || {
                    // TODO: look into removing the unwrap calls
                    let mut socket = socket.into_std().unwrap();
                    socket.set_nonblocking(false).unwrap();
                    match handle_tcp_client(socket.try_clone().unwrap(), &mut socket, &peer_name) {
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
        handle_tcp_client(reader, &mut writer, &peer_name).unwrap();
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
        handle_tcp_client(reader, &mut writer, &peer_name).unwrap();
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
}
