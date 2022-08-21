use clap::{arg, value_parser, App, ArgAction};
use data_encoding::BASE64;
use ring::hmac;
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::str::from_utf8;
use std::time::{SystemTime, UNIX_EPOCH};

extern crate strfmt;
use strfmt::strfmt;
use std::collections::HashMap;

extern crate log;
use env_logger::Env;
use log::{debug, info, LevelFilter};
use std::env;
use syslog::{BasicLogger, Facility, Formatter3164};

mod lib;
use lib::get_key;

fn split_payload(buf: &[u8]) -> Result<(&[u8], &[u8]), std::io::ErrorKind> {
    for (i, &v) in buf.iter().enumerate() {
        if v == b':' {
            return Ok((&buf[..i], &buf[i + 1..]));
        }
    }

    Err(ErrorKind::NotFound)
}

fn process_payload(amt: usize, src: &String, buf: &[u8], key: &hmac::Key) -> bool {
    debug!("{} sent {} bytes, \"{}\"", src, amt, String::from_utf8_lossy(&buf));

    let (nonce, tag) = match split_payload(&buf) {
        Ok(v) => v,
        Err(e) => {
            debug!("invalid payload: {}", e);
            return false;
        }
    };

    let snonce = match from_utf8(nonce) {
        Ok(s) => s,
        Err(e) => {
            debug!("invalid nonce(!utf8): {}", e);
            return false;
        }
    };

    let dtag = match BASE64.decode(tag) {
        Ok(b) => b,
        Err(e) => {
            debug!("invalid tag: {}", e);
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
                    debug!("invalid nonce(!now)");
                    return false;
                }
            }
            Err(_) => {
                debug!("invalid nonce(!u64)");
                return false;
            }
        },
        Err(_) => {
            debug!("invalid signature");
            return false;
        }
    };

    info!("{} VERIFIED", src);
    return true;
}

fn listen_to_msgs(listen: String, key: &hmac::Key, command: &String) {
    let mut buf = [0; 256];
    let socket = UdpSocket::bind(listen.as_str()).expect("couldn't bind to socket");

    // we use listen.as_str() above so we don't "move" listen to the bind()
    // if we did, we'd get an error about using listen after move on the next line
    info!("listening to {}", listen);

    loop {
        let (amt, src_addr) = socket.recv_from(&mut buf).expect("couldn't read from buffer");
        let src_with_port = src_addr.to_string();
        let src = src_with_port[..src_with_port.find(":").unwrap()].to_string();

        if process_payload(amt, &src, &buf[..amt], &key) {
            let vars = HashMap::from([ ("ip".to_string(), src) ]);
            let cmd = strfmt(&command, &vars).unwrap();
            info!("exec({})", &cmd);
        }
    }
}

fn get_args() -> (bool, bool, String, String, String) {
    let matches = App::new("door")
        .version("0.0.0")
        .author("Paul Miller <paul@jettero.pl>")
        .about("Watches the doors and listens for the secret codes")
        .arg(arg!(syslog: -S --syslog "log events and info to syslog instead of stdout").action(ArgAction::SetTrue))
        .arg(arg!(verbose: -v --verbose "print DEBUG level events instead of INFO").action(ArgAction::SetTrue))
        .arg(
            arg!(listen: -l --listen <ADDRINFO> "the IP and port on which to listen")
                .value_parser(value_parser!(String))
                .required(false)
                .default_value("0.0.0.0:20022")
        )
        .arg(
            arg!(secret: -s --secret <SECRET> "The secret code used in the knock. Note that this will be \
                 visible to anyone that can run 'ps' or even just read /proc. If the secret code starts with \
                 an '@' character, it's assumed to be a filename from which the secret should be read. The secret \
                 can also be set in the environment variable KNOCK_DOOR_SECRET.")
            .value_parser(value_parser!(String))
            .required(false)
            .default_value(&env::var("KNOCK_DOOR_SECRET").unwrap_or("secret".to_string()))
        )
        .arg(
            arg!(command: -c --command <SHELL_COMMAND> "The command to execute after a verified message is received. \
            Can also be set via KNOCK_DOOR_COMMAND. Note that the source IP will be passed via format!() \
            to this command string, so brace characters must be escaped (doubled) and the command should contain \
            {{ip}} if applicable to the command.")
            .value_parser(value_parser!(String))
            .required(false)
            .default_value(&env::var("KNOCK_DOOR_COMMAND").unwrap_or(
                    "nft add element inet firewall knock '{{ {ip} timeout 5s }}'".to_string()
                    ))
            )
        .get_matches();

    let verbose = *matches.get_one::<bool>("verbose").expect("defaulted by clap");
    let syslog = *matches.get_one::<bool>("syslog").expect("defaulted by clap");

    let key = matches
        .get_one::<String>("secret")
        .expect("defaulted by clap")
        .to_string();

    let listen = matches
        .get_one::<String>("listen")
        .expect("defaulted by clap")
        .to_string();

    let command = matches
        .get_one::<String>("command")
        .expect("defaulted by clap")
        .to_string();

    return (verbose, syslog, key, listen, command);
}

fn main() -> Result<(), ring::error::Unspecified> {
    let (verbose, syslog, key_str, listen, command) = get_args();
    let key = get_key(key_str);

    /*
     * rust really hates globals
     *
     * If we avoid using a global VERBOSE — to avoid using unsafe {} and static
     * mut … then this code won't look like such horrible evil; but it'll still
     * be using unsafe globals; just, hidden behind a std library.
     *
     * Reading this with some slight disbelief? This is exactly how they do it
     * in the std library:
     *
     *    https://docs.rs/log/0.3.8/src/log/lib.rs.html
     *
     * (Sometimes globals are completely appropriate. Fuck you rust … or at the
     * very least, fuck the people that think they're not.)
     *
     */

    if syslog {
        let formatter = Formatter3164 {
            facility: Facility::LOG_DAEMON,
            process: "knock-door".into(),
            hostname: None,
            pid: 0,
        };

        let logger = syslog::unix(formatter).unwrap();

        log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
            .map(|()| {
                log::set_max_level(match verbose {
                    true => LevelFilter::Debug,
                    false => LevelFilter::Info,
                })
            })
            .expect("logging setup failure");
    } else {
        let env = Env::default()
            .filter_or("KNOCK_DOOR_LOG_LEVEL", if verbose { "debug" } else { "info" })
            .write_style_or("KNOCK_DOOR_LOG_STYLE", "always");

        env_logger::init_from_env(env);
    }

    listen_to_msgs(listen, &key, &command);

    Ok(())
}
