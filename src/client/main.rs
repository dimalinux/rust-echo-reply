use clap::{Parser, Subcommand};

mod client_tcp;
use client_tcp::run_tcp_client;
mod client_udp;
use client_udp::run_udp_client;

const SERVER_ADDR: &str = "127.0.0.1:2048";
const CLIENT_ADDR: &str = "127.0.0.1:0"; // random port

const MAX_BUF_SZ: usize = 2048;

#[derive(Subcommand)]
enum Command {
    UDP {
        #[arg(short, long, default_value = SERVER_ADDR)]
        server_addr: std::net::SocketAddr,
        //#[arg(short, long, default_value = "Default UDP Message")]
        //message: String,
    },
    TCP {
        #[arg(short, long, default_value = SERVER_ADDR)]
        server_addr: std::net::SocketAddr,
        //#[arg(short, long, default_value = "Default TCP Message")]
        //message: String,
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
        } => {
            //println!("message is {}", message);
            //println!("server address is {}", server_addr);
            return run_udp_client(server_addr, String::from("Default UDP message"));
        }
        Command::TCP {
            server_addr,
        } => {
            //println!("server address is {}", server_addr);
            return run_tcp_client(server_addr);
        }
    };
}
