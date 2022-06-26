
use std::net::UdpSocket;
use data_encoding::BASE64;
use std::str::from_utf8;
use ring::hmac;

fn main() -> Result<(), std::io::Error> {
    let pass = "supz";
    let _key = hmac::Key::new(hmac::HMAC_SHA256, pass.as_bytes());
    let socket = UdpSocket::bind("localhost:22022")?;

    let mut buf = [0; 256];
    let (amt, src) = socket.recv_from(&mut buf)?;
    let chunks = from_utf8(&buf).unwrap().split(":");

    println!("heard: amt={} src={} chunks={:?}", amt, src, chunks);
    Ok(())
}
