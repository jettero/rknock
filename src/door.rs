use clap::{App, Arg};
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
    let matches = App::new("door is door")
        .version("0.0.0")
        .author("Paul Miller <paul@jettero.pl>")
        .about("Watches the doors and listens for the secret codes")
        .arg(Arg::with_name("verbose").long("verbose").short('v')
        .arg(Arg::with_name("listen").long("listen").short('l').takes_value(true)
             .help("where to listen for the codes. default: localhost:22"))
        .arg(Arg::with_name("secret").long("secret").short('s').takes_value(true)
             .help("do not use this in production, use a config file or something; handy for testing. default: secret"))
        .get_matches();

    let verbose = matches.value_of("verbose");
    let key = hmac::Key::new(hmac::HMAC_SHA256, matches.value_of("secret").unwrap_or("secret").as_bytes());
    let socket = UdpSocket::bind(matches.value_of("listen").unwrap_or("localhost:22"))
        .expect("couldn't bind to socket");

    let mut buf = [0; 256];
    let (amt, src) = socket.recv_from(&mut buf).expect("couldn't read from buffer");
    let (nonce, tag) = split_payload(&buf[..amt]).unwrap();
    let dtag = BASE64.decode(tag).unwrap();

    let verified = match hmac::verify(&key, &nonce, &dtag) {
        Ok(_) => true,
        Err(_) => false,
    };

    if args.verbose {
        let inonce: u64 = from_utf8(&nonce).unwrap().parse::<u64>().unwrap();
        let stag: &str = from_utf8(&tag).unwrap();
        println!(
            "heard: amt={} src={} nonce={} dtag={} {}",
            amt,
            src,
            inonce,
            stag,
            match verified {
                true => "verified",
                false => "FAILCOPTER",
            }
        );
    }

    Ok(())
}
