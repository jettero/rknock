use clap::{arg, crate_authors, crate_version, value_parser, App, ArgAction};
use data_encoding::BASE64;
use exec::execvp;
use ring::hmac;
use std::net::{Ipv4Addr, UdpSocket};
use std::time::{SystemTime, UNIX_EPOCH};

fn get_args() -> (bool, bool, String, String) {
    let matches = App::new("knock")
        .version(crate_version!())
        .author(crate_authors!(", "))
        .about("knock on doors")
        .arg(arg!(verbose: -v --verbose).action(ArgAction::SetTrue))
        .arg(arg!(goto: -g --goto-host).action(ArgAction::SetTrue))
        .arg(
            arg!(target: -t --target)
                .value_parser(value_parser!(String))
                .default_value("localhost:20022"),
        )
        .arg(
            arg!(secret: -s --secret)
                .value_parser(value_parser!(String))
                .default_value("secret"),
        )
        .get_matches();

    let verbose = *matches.get_one::<bool>("verbose").expect("defaulted by clap");
    let goto = *matches.get_one::<bool>("goto").expect("defaulted by clap");

    let key = matches
        .get_one::<String>("secret")
        .expect("defaulted by clap")
        .to_string();

    let target = matches
        .get_one::<String>("target")
        .expect("defaulted by clap")
        .to_string();

    return (verbose, goto, key, target);
}

fn main() {
    let (verbose, goto, key_str, target) = get_args();
    let key = hmac::Key::new(hmac::HMAC_SHA256, key_str.as_bytes());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("systemtime fucked")
        .as_secs();
    let nonce = format!("{}", now);
    let tag = hmac::sign(&key, nonce.as_bytes());
    let msg = format!("{}:{}", nonce, BASE64.encode(tag.as_ref()));

    if verbose {
        println!("sending: {}", msg);
    }

    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).expect("couldn't bind to address");
    socket.connect(&target).expect("connect function failed");
    socket.send(msg.as_bytes()).expect("couldn't send message");

    if goto {
        let err = execvp("ssh", &["ssh", &target]);
        println!("execvp(ssh {}) error: {}", target, err);
    }
}
