use std::{io::Result, net::SocketAddr, str};

use clap::{Parser, Subcommand};
use log::LevelFilter;
use tokio_util::sync::CancellationToken;

use crate::{
    server_tcp::run_tcp_server, server_udp::run_udp_server, signal_handler::run_signal_handler,
};

mod server_tcp;
mod server_udp;
mod signal_handler;

const BIND_ADDR: &str = "127.0.0.1:2048";

#[derive(Subcommand)]
enum Command {
    Udp {
        #[arg(short, long, default_value = BIND_ADDR)]
        bind_addr: SocketAddr,
    },
    Tcp {
        #[arg(short, long, default_value = BIND_ADDR)]
        bind_addr: SocketAddr,
    },
    Both {
        #[arg(short, long, default_value = BIND_ADDR)]
        bind_addr: SocketAddr,
    },
}

#[derive(Parser)]
#[command(about, version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

async fn run_both_servers(bind_addr: &SocketAddr, run_state: CancellationToken) -> Result<()> {
    let bind_addr_clone = *bind_addr;
    let run_state_clone = run_state.clone();

    let udp_task =
        tokio::spawn(async move { run_udp_server(&bind_addr_clone, run_state_clone).await });
    run_tcp_server(bind_addr, run_state).await?;

    udp_task.await?
}

fn init_logging() {
    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Trace)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let run_state = run_signal_handler();

    let args = Cli::parse();
    match args.command {
        Some(Command::Udp { bind_addr }) => run_udp_server(&bind_addr, run_state).await,
        Some(Command::Tcp { bind_addr }) => run_tcp_server(&bind_addr, run_state).await,
        Some(Command::Both { bind_addr }) => run_both_servers(&bind_addr, run_state).await,
        None => run_both_servers(&BIND_ADDR.parse().unwrap(), run_state).await,
    }
}

#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;

    use crate::run_both_servers;

    #[tokio::test]
    async fn test_both_servers() {
        let run_state = CancellationToken::new();
        let run_state_clone = run_state.clone();
        // Cancel the servers after a second
        let handle = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            run_state_clone.cancel();
        });
        run_both_servers(&"127.0.0.1:0".parse().unwrap(), run_state)
            .await
            .unwrap();

        handle.await.unwrap();
    }
}
