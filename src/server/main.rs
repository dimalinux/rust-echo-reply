use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::{env, str, thread};

use clap::{Parser, Subcommand};
use log::LevelFilter;

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

fn run_both_servers(bind_addr: &SocketAddr, shutdown: Arc<AtomicBool>) -> std::io::Result<()> {
    let bind_addr_clone = *bind_addr;
    let udp_thread = thread::spawn(move || run_udp_server(&bind_addr_clone));

    run_tcp_server(bind_addr, shutdown)?;

    udp_thread.join().unwrap()?;
    Ok(())
}

fn init_logging() {
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        env::set_var(
            env_logger::DEFAULT_FILTER_ENV,
            LevelFilter::Trace.to_string(),
        );
    }
    env_logger::init();
}

fn main() -> std::io::Result<()> {
    let shutdown = Arc::new(AtomicBool::new(false));

    init_logging();

    let args = Cli::parse();
    match args.command {
        Some(Command::Udp { bind_addr }) => run_udp_server(&bind_addr),
        Some(Command::Tcp { bind_addr }) => run_tcp_server(&bind_addr, shutdown.clone()),
        Some(Command::Both { bind_addr }) => run_both_servers(&bind_addr, shutdown.clone()),
        None => run_both_servers(&BIND_ADDR.parse().unwrap(), shutdown.clone()),
    }
}
