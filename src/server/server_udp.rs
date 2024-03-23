use std::net::{SocketAddr, UdpSocket};
use std::str;

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

pub fn run_udp(bind_addr: SocketAddr) -> std::io::Result<()> {
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
