use clap::{arg, crate_authors, crate_version, value_parser, App, ArgAction};
use data_encoding::BASE64;
use exec::execvp;
use ring::hmac;
use std::net::{Ipv4Addr, UdpSocket};
use std::time::{SystemTime, UNIX_EPOCH};
use std::env;

mod lib;
use lib::get_key;

fn get_args() -> (bool, bool, String, String) {
    let matches = App::new("knock")
        .version(crate_version!())
        .author(crate_authors!(", "))
        .about("knock on doors")
        .arg(arg!(verbose: -v --verbose "say what's happening on stdout").action(ArgAction::SetTrue))
        .arg(
            arg!(go: -g --go "after sending the knock codes, immedaitely execvp(ssh) to the host")
                .action(ArgAction::SetTrue)
        )
        .arg(
            arg!(target: -t --target <HOSTNAME> "destination host to knock \
                 (the port can also be specified after a colon). This value can \
                 also be set via the KNOCK_TARGET environment variable.")
                .value_parser(value_parser!(String))
                .required(false)
                .default_value(&env::var("KNOCK_TARGET").unwrap_or("localhost:20022".to_string()))

        )
        .arg(
            arg!(secret: -s --secret <SEMI_SECRET_CODE> "The secret code used in the knock. Note that this will be \
                 visible to anyone that can run 'ps' or even just read /proc. If the secret code starts with \
                 an '@' character, it's assumed to be a filename from which the secret should be read. The secret \
                 can also be set in the environment variable KNOCK_SECRET.")
            .value_parser(value_parser!(String))
            .required(false)
            .default_value(&env::var("KNOCK_SECRET").unwrap_or("secret".to_string()))
        )
        .get_matches();

    let verbose = *matches.get_one::<bool>("verbose").expect("defaulted by clap");
    let go = *matches.get_one::<bool>("go").expect("defaulted by clap");

    let key = matches
        .get_one::<String>("secret")
        .expect("defaulted by clap")
        .to_string();

    let target = matches
        .get_one::<String>("target")
        .expect("defaulted by clap")
        .to_string();

    return (verbose, go, key, target);
}

fn main() {
    let (verbose, go, key_str, mut target) = get_args();
    let key = get_key(key_str);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("systemtime fucked")
        .as_secs();
    let nonce = format!("{}", now);
    let tag = hmac::sign(&key, nonce.as_bytes());
    let msg = format!("{}:{}", nonce, BASE64.encode(tag.as_ref()));

    if !target.contains(":") {
        target = target + ":20022"
    }

    if verbose {
        println!("send(\"{}\") → {}", msg, target);
    }

    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).expect("couldn't bind to 0.0.0.0:0 address");

    socket.connect(&target).expect("connect function failed");
    socket.send(msg.as_bytes()).expect("couldn't send message");

    if go {
       let err = execvp("ssh", &["ssh", &target]);
       panic!("execvp(ssh {}) error: {}", target, err);
    }
}
