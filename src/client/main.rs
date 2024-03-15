use std::time::Duration;
use clap::{Parser, Subcommand};

const SERVER_ADDR: &str = "127.0.0.1:2048";
const CLIENT_ADDR: &str = "127.0.0.1:0"; // random port

const MAX_BUF_SZ: usize = 2048;

fn run_udp(server_addr: std::net::SocketAddr, message: String) -> std::io::Result<()> {
    // Get a client socket to send from on a random UDP port
    let socket = std::net::UdpSocket::bind(CLIENT_ADDR.to_string())?;

    let message_sz = socket.send_to(message.as_bytes(), server_addr)?;
    println!("\nsent echo of {} bytes to server\n", message_sz);

    let mut buf = [0; MAX_BUF_SZ];
    socket.set_read_timeout(Some(Duration::from_secs(1)))
        .expect("Could not set a read timeout");
    let (amt, src) = match socket.recv_from(&mut buf) {
        Ok(result) => result,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::WouldBlock {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other, "no response from server")
                );
            }
            // Handle the error here
            eprintln!("Error kind is {}\n", err.kind());
            eprintln!("Error receiving data from socket: {}\n", err);
            return Err(err); // Or take some other recovery action
        }
    };
    let echo = std::str::from_utf8(&buf[..amt]).unwrap();
    println!("Echo from: {:?}, size: {:?}\n{}\n", src, amt, echo);
    Ok(())
}

#[derive(Subcommand)]
enum Command {
    UDP {
        #[arg(short, long, default_value = SERVER_ADDR)]
        server_addr: std::net::SocketAddr,
        #[arg(short, long, default_value = "Default UDP Message")]
        message: String,
    },
    TCP {
        #[arg(short, long, default_value = SERVER_ADDR)]
        server_addr: std::net::SocketAddr,
        #[arg(short, long, default_value = "Default TCP Message")]
        message: String,
    },
}

#[derive(Parser)]
#[command(about, version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn main() -> std::io::Result<()> {
    let args = Cli::parse();
    match args.command {
        Command::UDP {
            server_addr,
            message
        } => {
            println!("message is {}", message);
            println!("server address is {}", server_addr);
            return run_udp(server_addr, message);
        }
        Command::TCP {
            server_addr,
            message
        } => {
            println!("message is {}", message);
            println!("server address is {}", server_addr);
        }
    };
    Ok(())
}