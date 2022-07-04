use clap::{App, arg, ArgAction, value_parser};
use data_encoding::BASE64;
use ring::hmac;
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::str::from_utf8;

extern crate log;
use syslog::{Facility, Formatter3164, BasicLogger};
use log::{LevelFilter, info};

fn split_payload(buf: &[u8]) -> Result<(&[u8], &[u8]), std::io::ErrorKind> {
    for (i, &v) in buf.iter().enumerate() {
        if v == b':' {
            return Ok((&buf[..i], &buf[i + 1..]));
        }
    }

    Err(ErrorKind::NotFound)
}

fn recv_one_payload(verbose: bool, socket: UdpSocket, key: &hmac::Key) {
    let mut buf = [0; 256];
    let (amt, src) = socket.recv_from(&mut buf).expect("couldn't read from buffer");
    let (nonce, tag) = split_payload(&buf[..amt]).unwrap();

    if verbose {
        info!("heard something, checking");
    }

    let dtag = BASE64.decode(tag).unwrap();
    let verified = match hmac::verify(&key, &nonce, &dtag) {
        Ok(_) => true,
        Err(_) => false,
    };

    // TOOD: inonce should represent epoch seconds eventually
    // let inonce: u64 = from_utf8(&nonce).unwrap().parse::<u64>().unwrap();

    if verbose {
        let snonce = from_utf8(nonce).unwrap();
        let stag: &str = from_utf8(&tag).unwrap();

        println!(
            "heard: amt={} src={} nonce={} dtag={} {}",
            amt,
            src,
            snonce,
            stag,
            match verified {
                true => "verified",
                false => "invalid signature",
            }
        );
    }
}

fn listen_to_msgs(verbose: bool, listen: String, key: &hmac::Key) {
    let socket = UdpSocket::bind(listen.as_str()).expect("couldn't bind to socket");

    if verbose {
        // we use listen.as_str() above so we don't "move" listen to the bind()
        // if we did, we'd get an error about using listen after move on the next line
        info!("listening to {}", listen);
    }

    while true {
        recv_one_payload(verbose, socket, &key);
    }
}

fn get_args() -> (bool, String, String) {
    let matches = App::new("door is door")
        .version("0.0.0")
        .author("Paul Miller <paul@jettero.pl>")
        .about("Watches the doors and listens for the secret codes")
        .arg(arg!(verbose: -v --verbose).action(ArgAction::SetTrue))
        .arg(arg!(listen: -l --listen).value_parser(value_parser!(String)).default_value("localhost:20022"))
        .arg(arg!(secret: -s --secret).value_parser(value_parser!(String)).default_value("secret"))
        .get_matches();

    let verbose = matches.get_one::<bool>("verbose").expect("defaulted by clap");
    let key = matches.get_one::<String>("secret").expect("defaulted by clap").to_string();
    let listen = matches.get_one::<String>("listen").expect("defaulted by clap").to_string();

    return (*verbose, key, listen);
}

fn main() -> Result<(), ring::error::Unspecified> {
    let formatter = Formatter3164 {
        facility: Facility::LOG_DAEMON,
        process: "knock-door".into(),
        hostname: None,
        pid: 0,
    };

    let logger = syslog::unix(formatter).unwrap();

    log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .expect("logging setup failure");

    let (verbose, key_str, listen) = get_args();
    let key = hmac::Key::new(hmac::HMAC_SHA256, key_str.as_bytes());

    listen_to_msgs(verbose, listen, &key);

    Ok(())
}
