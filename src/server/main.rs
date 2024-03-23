use std::str;

use clap::{Parser, Subcommand};

use crate::server_tcp::run_tcp;
use crate::server_udp::run_udp;

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
}

#[derive(Parser)]
#[command(about, version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn main() -> std::io::Result<()> {
    let args = Cli::parse();
    return match args.command {
        Command::UDP { bind_addr } => run_udp(bind_addr),
        Command::TCP { bind_addr } => run_tcp(bind_addr),
    };
}
