use std::net::UdpSocket;

fn main() {
    let socket = UdpSocket::bind("localhost:1234").expect("couldn't bind to address");
    socket.connect("localhost:22").expect("connect function failed");
    socket.send(b"supz\n").expect("couldn't send message");
}
