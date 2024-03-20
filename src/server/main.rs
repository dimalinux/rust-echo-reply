use std::fmt::Debug;
use std::io::{BufRead, Read, Write};
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::str;

use clap::{Parser, Subcommand};

const BIND_ADDR: &str = "127.0.0.1:2048";

fn event_loop_udp(socket: UdpSocket) -> std::io::Result<()> {
    let mut buf = [0; 2048];

    loop {
        let (amt, src) = match socket.recv_from(&mut buf) {
            Ok(result) => result,
            Err(err) => {
                // Handle the error here
                eprintln!("Error receiving data from socket: {}\n", err);
                return Err(err); // Or take some other recovery action
            }
        };

        let message_buf = &buf[0..amt];
        let message = str::from_utf8(message_buf).unwrap();
        println!("From: {:?}, size={} message:\n{:?}", src, amt, message);
        let amt = socket.send_to(message_buf, src)?;
        println!("sent {} bytes", amt)
    }
}

fn handle_tcp_client<R: Read, W: Write + Debug>(reader: R, writer: &mut W, peer_name: &String) {
    //let mut writer = conn; // writer is unbuffered
    //let mut writer = std::io::BufWriter::new(writer);

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

fn event_loop_tcp(listener: &mut TcpListener) -> std::io::Result<()> {
    loop {
        let socket = listener.accept()?; // add error handling
        let peer_name = socket.0.peer_addr().unwrap().to_string();
        println!("Accepted connection from {:?}", peer_name);
        let mut writer = socket.0;
        let reader = writer.try_clone()?;
        handle_tcp_client(reader, &mut writer, &peer_name);
        println!("Terminating connection with {:?}", peer_name);
    }
}

#[derive(Subcommand)]
enum Command {
    UDP {
        #[arg(short, long, default_value = BIND_ADDR)]
        bind_addr: std::net::SocketAddr,
    },
    TCP {
        #[arg(short, long, default_value = BIND_ADDR)]
        bind_addr: std::net::SocketAddr,
    },
}

#[derive(Parser)]
#[command(about, version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn run_udp(bind_addr: SocketAddr) -> std::io::Result<()> {
    let socket = match UdpSocket::bind(bind_addr) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Error binding socket: {}\n", err);
            return Err(err); // Or take some other recovery action
        }
    };
    println!("\nstart server\n");

    event_loop_udp(socket)
}

fn run_tcp(bind_addr: SocketAddr) -> std::io::Result<()> {
    let mut socket = match TcpListener::bind(bind_addr) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Error binding socket: {}\n", err);
            return Err(err); // Or take some other recovery action
        }
    };
    println!("\nstart server\n");

    event_loop_tcp(&mut socket)
}

fn main() -> std::io::Result<()> {
    let args = Cli::parse();
    return match args.command {
        Command::UDP { bind_addr } => run_udp(bind_addr),
        Command::TCP { bind_addr } => run_tcp(bind_addr),
    };
}


#[cfg(test)]
mod tests {
    use std::io::{BufWriter, Cursor, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;
    use crate::handle_tcp_client;

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
        let addr = socket.local_addr().unwrap();
        let mut conn = TcpStream::connect(addr).unwrap();
        let input = String::from("test\n");
        conn.write_all("test\n".as_bytes()).unwrap();
        conn.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let mut echo = String::new();
        conn.read_to_string(&mut echo).unwrap();
        assert_eq!(input, echo);
    }

}