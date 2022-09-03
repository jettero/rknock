use exec::execvp;

use std::env;
use std::net::{Ipv4Addr, UdpSocket};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use clap::{arg, crate_authors, crate_version, value_parser, App, ArgAction, ValueSource};
use config::Config;

use rlib::{config_file, grok_setting, HMACFrobnicator};

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
            arg!(config: -c --config <CONFIG> "read this config file for settings")
            .value_parser(value_parser!(String))
            .required(false)
            .default_value(&config_file())
        )
        .arg(
            arg!(target: -t --target <HOSTNAME> "destination host to knock \
                 (the port can also be specified after a colon). This value can \
                 also be set via the KNOCK_TARGET environment variable.")
                .value_parser(value_parser!(String))
                .required(false)
                .default_value("localhost:20022")

        )
        .arg(
            arg!(secret: -s --secret <SEMI_SECRET_CODE> "The secret code used in the knock. Note that this will be \
                 visible to anyone that can run 'ps' or even just read /proc. If the secret code starts with \
                 an '@' character, it's assumed to be a filename from which the secret should be read. The secret \
                 can also be set in the environment variable KNOCK_SECRET.")
            .value_parser(value_parser!(String))
            .required(false)
            .default_value("secret")
        )
        .arg(
            arg!(no_salt: --"no-salt" "this salt portion of the nonce isn't strictly necessary and can be disabled")
                .action(ArgAction::SetTrue)
                .required(false)
        )
        .get_matches();

    let settings = Config::builder()
        .add_source(config::File::with_name(
            matches.get_one::<String>("config").expect("defaulted by clap"),
        ))
        .add_source(config::Environment::with_prefix("KNOCK"))
        .build()
        .unwrap();

    let target: String = grok_setting!(matches, settings, "target", String);
    let key: String = grok_setting!(matches, settings, "secret", String);
    let verbose: bool = grok_setting!(matches, settings, "verbose", bool);
    let go: bool = grok_setting!(matches, settings, "go", bool);
    let disable_salt: bool = grok_setting!(matches, settings, "no_salt", bool);

    // if verbose {
    //     println!("options:");
    //     println!("  target:  {target:?}");
    //     println!("  secret:  {key:?}");
    //     println!("  verbose: {verbose:?}");
    //     println!("  go:      {go:?}");
    //     println!("  no-salt: {disable_salt:?}");
    // }

    (verbose, go, key, target, disable_salt)
}

macro_rules! my_err {
    ($thing:expr, $preamble:expr, $code:expr) => {
        match $thing.take_error() {
            Ok(Some(error)) => {
                eprintln!("{}: {error:?}", $preamble);
                ExitCode::from($code)
            }
            Ok(None) => {
                eprintln!("{}: host not found", $preamble);
                ExitCode::from($code)
            }
            Err(error) => {
                eprintln!("{}: {error:?}", $preamble);
                ExitCode::from($code)
            }
        }
    };
}

fn main() -> ExitCode {
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
    // let addr = match target.to_socket_addrs() {
    //     Ok(mut addrs) => match addrs.next() {
    //         Some(v) => v,
    //         None => {
    //             eprintln!("error resolving target {:?}: host not found", target);
    //             return ExitCode::from(7);
    //         }
    //     },
    //     Err(e) => {
    //         eprintln!("error resolving target {:?}: {:?}", target, e);
    //         return ExitCode::from(7);
    //     }
    // };

    match socket.connect(&target) {
        Ok(_) => match socket.send(msg.as_bytes()) {
            Ok(_) => {
                if go {
                    let err = execvp("ssh", &["ssh", &target]);
                    eprintln!("execvp(ssh {target}) error: {err:?}");
                    return ExitCode::from(1);
                }
                ExitCode::from(0)
            }
            Err(_) => my_err!(socket, format!("failed to send {msg:?} to {target:?}"), 2),
        },
        Err(_) => my_err!(socket, format!("failed to connect to {target:?}"), 3),
    }
}
