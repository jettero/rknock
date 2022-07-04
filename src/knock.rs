use std::net::UdpSocket;
use data_encoding::BASE64;
use ring::hmac;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let pass = "secret";
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("systemtime fucked").as_secs();
    let nonce = format!("{}", now);
    let key = hmac::Key::new(hmac::HMAC_SHA256, pass.as_bytes());
    let tag = hmac::sign(&key, nonce.as_bytes());
    let msg = format!("{}:{}", nonce, BASE64.encode(tag.as_ref()));

    println!("sending: {}", msg);

    let socket = UdpSocket::bind("localhost:1234").expect("couldn't bind to address");
    socket.connect("localhost:20022").expect("connect function failed");
    socket.send(msg.as_bytes()).expect("couldn't send message");
}
