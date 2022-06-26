use std::net::UdpSocket;
use data_encoding::BASE64;
use ring::hmac;

fn main() {
    let nonce = "nonce";
    let pass = "supz";
    let key = hmac::Key::new(hmac::HMAC_SHA256, pass.as_bytes());
    let tag = hmac::sign(&key, nonce.as_bytes());
    let msg = format!("{}:{}", nonce, BASE64.encode(tag.as_ref()));

    println!("sending: {}", msg);

    let socket = UdpSocket::bind("localhost:1234").expect("couldn't bind to address");
    socket.connect("localhost:22022").expect("connect function failed");
    socket.send(msg.as_bytes()).expect("couldn't send message");
}
