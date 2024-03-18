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

fn handle_tcp_client<R: Read, W: Write>(reader: R, writer: W, peer_name: String) {
    let mut line = String::new();
    //let mut writer = conn; // writer is unbuffered
    let mut writer = std::io::BufWriter::new(writer);

    // Add read timeout (5 minutes?)
    let mut reader = std::io::BufReader::new(reader);

    loop {
        let len = match reader.read_line(&mut line) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("Error kind is {}\n", err.kind());
                eprintln!("Error receiving data from client: {}\n", err);
                break;
            },
        };
        println!("From: {}, str-size={} byte-size={} read-ln-size={} message:\n{:?}", peer_name,
                 line.len(), line.as_bytes().len(), len, line);
        writer.write_all(line.as_bytes()).unwrap(); // todo: add error handling
        println!("sent {} bytes", line.len())
    }
}


fn event_loop_tcp(listener: &mut TcpListener) -> std::io::Result<()> {
    loop {
        let socket = listener.accept()?; // add error handling
        let peer_name = socket.0.peer_addr().unwrap().to_string();
        println!("Accepted connection from {:?}", peer_name);
        let reader = socket.0;
        let writer = reader.try_clone()?;
        handle_tcp_client(reader, writer, peer_name);
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
        Command::UDP {
            bind_addr,
        } => {
            run_udp(bind_addr)
        }
        Command::TCP {
            bind_addr,
        } => {
            run_tcp(bind_addr)
        }
    }
}

