use std::net::{SocketAddr, UdpSocket};
use std::str;

fn event_loop_udp(socket: UdpSocket) -> std::io::Result<()> {
    let mut buf = [0; 2048];

    loop {
        let (size, from_addr) = match socket.recv_from(&mut buf) {
            Ok(result) => result,
            Err(err) => {
                // Handle the error here
                eprintln!("Error receiving data from socket: {}\n", err);
                return Err(err); // Or take some other recovery action
            }
        };

        let message_buf = &buf[0..size];
        let message = str::from_utf8(message_buf).unwrap();
        println!("from: {:?} UDP, sz: {} message: {:?}", from_addr, size, message);
        let amt = socket.send_to(message_buf, from_addr)?;
        println!("sent {} bytes", amt)
    }
}

pub fn run_udp_server(bind_addr: &SocketAddr) -> std::io::Result<()> {
    let socket = match UdpSocket::bind(bind_addr) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Error binding socket: {}\n", err);
            return Err(err); // Or take some other recovery action
        }
    };

    println!("starting UDP server on {}", socket.local_addr()?);

    event_loop_udp(socket)
}
