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
    UDP {
        #[arg(short, long, default_value = BIND_ADDR)]
        bind_addr: std::net::SocketAddr,
    },
    TCP {
        #[arg(short, long, default_value = BIND_ADDR)]
        bind_addr: std::net::SocketAddr,
    },
    BOTH {
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
    return match args.command {
        Some(Command::UDP { bind_addr }) => run_udp_server(&bind_addr),
        Some(Command::TCP { bind_addr }) => run_tcp_server(&bind_addr),
        Some(Command::BOTH { bind_addr }) => run_both_servers(&bind_addr),
        None => run_both_servers(&BIND_ADDR.parse().unwrap()),
    };
}
