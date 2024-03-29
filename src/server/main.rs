use std::net::SocketAddr;
use std::{str, thread};

use clap::{Parser, Subcommand};

use crate::server_tcp::run_tcp_server;
use crate::server_udp::run_udp_server;

mod server_tcp;
mod server_udp;

const BIND_ADDR: &str = "127.0.0.1:2048";

#[derive(Subcommand)]
enum Command {
    Udp {
        #[arg(short, long, default_value = BIND_ADDR)]
        bind_addr: std::net::SocketAddr,
    },
    Tcp {
        #[arg(short, long, default_value = BIND_ADDR)]
        bind_addr: std::net::SocketAddr,
    },
    Both {
        #[arg(short, long, default_value = BIND_ADDR)]
        bind_addr: std::net::SocketAddr,
    },
}

#[derive(Parser)]
#[command(about, version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

fn run_both_servers(bind_addr: &SocketAddr) -> std::io::Result<()> {
    let bind_addr_clone = bind_addr.clone();
    let udp_thread = thread::spawn(move || run_udp_server(&bind_addr_clone));

    run_tcp_server(bind_addr)?;

    udp_thread.join().unwrap()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let args = Cli::parse();
    match args.command {
        Some(Command::Udp { bind_addr }) => run_udp_server(&bind_addr),
        Some(Command::Tcp { bind_addr }) => run_tcp_server(&bind_addr),
        Some(Command::Both { bind_addr }) => run_both_servers(&bind_addr),
        None => run_both_servers(&BIND_ADDR.parse().unwrap()),
    }
}
