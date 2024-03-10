use std::net::UdpSocket;

const SERVER_ADDR: &str = "127.0.0.1:2048";
const CLIENT_ADDR: &str = "127.0.0.1:0"; // random port

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut message = "Default message";
    if args.len() >= 2 {
        message = &args[1];
    }

    // Get a client socket to send from on a random UDP port
    let socket = UdpSocket::bind(CLIENT_ADDR.to_string())?;

    let message_sz = socket.send_to(message.as_bytes(), SERVER_ADDR)?;
    println!("\nsent echo of {} bytes to server\n", message_sz);

    let mut buf = [0; 2048];
    let (amt, src) = socket.recv_from(&mut buf)?;
    let echo = std::str::from_utf8(&buf[..amt]).unwrap();
    println!("Echo from: {:?}, size: {:?}\n{}\n", src, amt, echo);
    Ok(())
}