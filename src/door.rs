use data_encoding::BASE64;
use ring::hmac;
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::str::from_utf8;

fn split_payload(buf: &[u8]) -> Result<(&[u8], &[u8]), std::io::ErrorKind> {
    for (i, &v) in buf.iter().enumerate() {
        if v == b':' {
            return Ok((&buf[..i], &buf[i + 1..]));
        }
    }
    Err(ErrorKind::NotFound)
}

fn main() -> Result<(), ring::error::Unspecified> {
    let pass = "supz";
    let key = hmac::Key::new(hmac::HMAC_SHA256, pass.as_bytes());
    let socket = UdpSocket::bind("localhost:22022").expect("couldn't bind to socket");

    let mut buf = [0; 256];
    let (amt, src) = socket
        .recv_from(&mut buf)
        .expect("couldn't read from buffer");
    let (nonce, tag) = split_payload(&buf[..amt]).unwrap();

    let dtag = BASE64.decode(tag).unwrap();

    let verified = match hmac::verify(&key, nonce, &dtag) {
        Ok(_) => true,
        Err(_) => false,
    };

    println!(
        "heard: amt={} src={} nonce={} tag={} {}",
        amt,
        src,
        from_utf8(nonce).unwrap(),
        from_utf8(tag).unwrap(),
        match verified {
            true => "verified",
            false => "FAILCOPTER",
        }
    );

    Ok(())
}
