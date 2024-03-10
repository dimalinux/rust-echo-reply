use std::net::UdpSocket;
use std::str;

const LISTEN_ADDR: &str = "127.0.0.1:2048";

fn main() -> std::io::Result<()> {
    // TODO: Better error handling
    let socket = UdpSocket::bind(LISTEN_ADDR.to_string())?;
    println!("\nstart server\n");

    loop {
        let mut buf = [0; 2048];
        let (amt, src) = socket.recv_from(&mut buf)?;
        let message = str::from_utf8(&buf[0..amt]).unwrap();
        println!("From: {:?}, buf: {:?}", src, message);

        socket.send_to(&buf, src)?;
    }
}