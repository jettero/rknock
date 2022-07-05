use clap::{arg, value_parser, App, ArgAction};
use data_encoding::BASE64;
use ring::hmac;
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::str::from_utf8;
use std::time::{SystemTime, UNIX_EPOCH};

extern crate log;
use log::{info, LevelFilter};
use syslog::{BasicLogger, Facility, Formatter3164};

static mut HORRIBLE_PURE_EVIL_GLOBAL_FLAG_VERBOSE: bool = false;

macro_rules! vs {
    ($($say:expr),*) => {
        if unsafe{ HORRIBLE_PURE_EVIL_GLOBAL_FLAG_VERBOSE } {
            info!($( $say ),*)
        }
    }
}

fn split_payload(buf: &[u8]) -> Result<(&[u8], &[u8]), std::io::ErrorKind> {
    for (i, &v) in buf.iter().enumerate() {
        if v == b':' {
            return Ok((&buf[..i], &buf[i + 1..]));
        }
    }

    Err(ErrorKind::NotFound)
}

fn process_payload(amt: usize, src: String, buf: &[u8], key: &hmac::Key) -> bool {
    vs!("{} sent {} bytes, \"{}\"", src, amt, String::from_utf8_lossy(&buf));

    let (nonce, tag) = match split_payload(&buf) {
        Ok(v) => v,
        Err(e) => {
            vs!("invalid payload: {}", e);
            return false;
        }
    };
    let snonce = match from_utf8(nonce) {
        Ok(s) => s,
        Err(e) => {
            vs!("invalid nonce(!utf8): {}", e);
            return false;
        }
    };
    let dtag = match BASE64.decode(tag) {
        Ok(b) => b,
        Err(e) => {
            vs!("invalid tag: {}", e);
            return false;
        }
    };

    match hmac::verify(&key, &nonce, &dtag) {
        Ok(_) => match snonce.parse::<u64>() {
            Ok(inonce) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("systemtime fucked")
                    .as_secs();
                if inonce != now && inonce != (now - 1) {
                    vs!("invalid nonce(!now)");
                    return false;
                }
            }
            Err(_) => {
                vs!("invalid nonce(!u64)");
                return false;
            }
        },
        Err(_) => {
            vs!("invalid signature");
            return false;
        }
    };

    info!("{} VERIFIED", src); // info! not vs! ... we really want to talk about this
    return true;
}

fn listen_to_msgs(listen: String, key: &hmac::Key) {
    let mut buf = [0; 256];
    let socket = UdpSocket::bind(listen.as_str()).expect("couldn't bind to socket");

    // we use listen.as_str() above so we don't "move" listen to the bind()
    // if we did, we'd get an error about using listen after move on the next line
    vs!("listening to {}", listen);

    loop {
        let (amt, src) = socket.recv_from(&mut buf).expect("couldn't read from buffer");

        if process_payload(amt, src.to_string(), &buf[..amt], &key) {
            info!("TODO: accept actions");
        }
    }
}

fn get_args() -> (String, String) {
    let matches = App::new("door is door")
        .version("0.0.0")
        .author("Paul Miller <paul@jettero.pl>")
        .about("Watches the doors and listens for the secret codes")
        .arg(arg!(verbose: -v --verbose).action(ArgAction::SetTrue))
        .arg(
            arg!(listen: -l --listen)
                .value_parser(value_parser!(String))
                .default_value("localhost:20022"),
        )
        .arg(
            arg!(secret: -s --secret)
                .value_parser(value_parser!(String))
                .default_value("secret"),
        )
        .get_matches();

    unsafe {
        HORRIBLE_PURE_EVIL_GLOBAL_FLAG_VERBOSE = *matches.get_one::<bool>("verbose").expect("defaulted by clap");
    }

    let key = matches
        .get_one::<String>("secret")
        .expect("defaulted by clap")
        .to_string();
    let listen = matches
        .get_one::<String>("listen")
        .expect("defaulted by clap")
        .to_string();

    return (key, listen);
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

    let (key_str, listen) = get_args();
    let key = hmac::Key::new(hmac::HMAC_SHA256, key_str.as_bytes());

    listen_to_msgs(listen, &key);

    Ok(())
}
