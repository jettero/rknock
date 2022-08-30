use clap::{arg, crate_authors, crate_version, value_parser, App, ArgAction};
use exec::execvp;
use std::env;
use std::net::{Ipv4Addr, UdpSocket};
use std::time::{SystemTime, UNIX_EPOCH};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use rlib::HMACFrobnicator;

fn get_args() -> (bool, bool, String, String, bool) {
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
                .default_value(&env::var("KNOCK_TARGET").unwrap_or_else(|_| "localhost:20022".to_string()))

        )
        .arg(
            arg!(secret: -s --secret <SEMI_SECRET_CODE> "The secret code used in the knock. Note that this will be \
                 visible to anyone that can run 'ps' or even just read /proc. If the secret code starts with \
                 an '@' character, it's assumed to be a filename from which the secret should be read. The secret \
                 can also be set in the environment variable KNOCK_SECRET.")
            .value_parser(value_parser!(String))
            .required(false)
            .default_value(&env::var("KNOCK_SECRET").unwrap_or_else(|_| "secret".to_string()))
        )
        .arg(
            arg!(no_salt: --"no-salt" "this salt portion of the nonce isn't strictly necessary and can be disabled")
                .action(ArgAction::SetTrue)
                .required(false)
        )
        .get_matches();

    let verbose = *matches.get_one::<bool>("verbose").expect("defaulted by clap");
    let go = *matches.get_one::<bool>("go").expect("defaulted by clap");
    let disable_salt = *matches.get_one::<bool>("no_salt").expect("defaulted by clap");

    let key = matches
        .get_one::<String>("secret")
        .expect("defaulted by clap")
        .to_string();

    let target = matches
        .get_one::<String>("target")
        .expect("defaulted by clap")
        .to_string();

    (verbose, go, key, target, disable_salt)
}

fn main() {
    let (verbose, go, key_str, mut target, disable_salt) = get_args();
    let mut hf = HMACFrobnicator::new(&key_str);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("systemtime fucked")
        .as_secs();

    let nonce = if disable_salt {
        format!("{}", now)
    } else {
        let salt: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(13)
            .map(char::from)
            .collect();
        format!("{}${}", now, salt)
    };

    let msg = hf.sign(&nonce);

    if !target.contains(':') {
        target += ":20022"
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
