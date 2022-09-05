use exec::execvp;

use std::env;
use std::error::Error;
use std::net::{Ipv4Addr, UdpSocket};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use clap::{arg, crate_authors, crate_version, value_parser, App, ArgAction, ArgMatches, ValueSource};
use config::Config;

use rlib::{config_filez, grok_setting, is_default, HMACFrobnicator};

trait Pfft {
    fn my_get_matches(self) -> ArgMatches;
}

impl Pfft for App<'_> {
    #[cfg(not(test))]
    fn my_get_matches(self) -> ArgMatches {
        self.get_matches()
    }

    #[cfg(test)]
    fn my_get_matches(self) -> ArgMatches {
        if let Ok(v) = env::var("_JUST_TESTING_MAIN_args") {
            self.get_matches_from(v.split(",").map(|a| a.to_string()).collect::<Vec<String>>())
        } else {
            self.get_matches()
        }
    }
}

fn get_args() -> Result<(bool, bool, String, String, bool, u64), Box<dyn Error>> {
    let matches = App::new("knock")
        .version(crate_version!())
        .author(crate_authors!(", "))
        .about("Knocks on doors")
        .arg(arg!(verbose: -v --verbose "say what's happening on stdout").action(ArgAction::SetTrue))
        .arg(
            arg!(go: -g --go "after sending the knock codes, immedaitely execvp(ssh) to the host")
                .action(ArgAction::SetTrue)
        )
        .arg(
            arg!(config: -C --config <CONFIG> "read this config file for settings")
            .value_parser(value_parser!(String))
            .required(false)
            .multiple(true) // I hate this:
            .default_values(&config_filez("KNOCK").iter().map(|a| a.as_str()).collect::<Vec<&str>>())
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
            arg!(time_code: --"time-code" <TIMESTAMP> "use this timestamp instead of the current time")
                .value_parser(value_parser!(u64))
                .required(false)
                .default_value("0")
        )
        .arg(
            arg!(no_salt: --"no-salt" "this salt portion of the nonce isn't strictly necessary and can be disabled")
                .action(ArgAction::SetTrue)
                .required(false)
        )
        .my_get_matches();

    let filez = matches.get_many::<String>("config").expect("defaulted by clap");
    let def = is_default!(matches, "config");
    let mut config = Config::builder();
    for item in filez {
        config = config.add_source(config::File::with_name(item).required(!def));
    }
    config = config.add_source(config::Environment::with_prefix("KNOCK"));

    let settings = config.build()?;

    let target: String = grok_setting!(matches, settings, "target", String);
    let key: String = grok_setting!(matches, settings, "secret", String);
    let verbose: bool = grok_setting!(matches, settings, "verbose", bool);
    let go: bool = grok_setting!(matches, settings, "go", bool);
    let disable_salt: bool = grok_setting!(matches, settings, "no_salt", bool);
    let time_code: u64 = grok_setting!(matches, settings, "time_code", u64);

    // if verbose {
    //     println!("options:");
    //     println!("  target:    {target:?}");
    //     println!("  secret:    {key:?}");
    //     println!("  verbose:   {verbose:?}");
    //     println!("  go:        {go:?}");
    //     println!("  no-salt:   {disable_salt:?}");
    //     println!("  time-code: {time_code:?}");
    // }

    Ok((verbose, go, key, target, disable_salt, time_code))
}

macro_rules! my_sock_err {
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
    let (verbose, go, key_str, mut target, disable_salt, time_code) = match get_args() {
        Ok(v) => v,
        Err(error) => {
            eprintln!("error building config: {error:?}");
            return ExitCode::from(27);
        }
    };
    let mut hf = HMACFrobnicator::new(&key_str);
    let now = if time_code > 0 {
        time_code
    } else {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("systemtime fucked")
            .as_secs()
    };

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

    #[cfg(test)]
    if let Ok(v) = env::var("_JUST_TESTING_MAIN_msg") {
        if v == "1" {
            env::set_var("_JUST_TESTING_MAIN_msg", &msg);
        }
    }

    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).expect("couldn't bind to 0.0.0.0:0 address");
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
            Err(_) => my_sock_err!(socket, format!("failed to send {msg:?} to {target:?}"), 2),
        },
        Err(_) => my_sock_err!(socket, format!("failed to connect to {target:?}"), 3),
    }
}

//---------=: TEST
#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn unsalted_knock() -> Result<(), Box<dyn Error>> {
        env::set_var("KNOCK_CONFIG_SEARCH", "/dev/null");
        env::set_var("_JUST_TESTING_MAIN_msg", "1");
        env::set_var("_JUST_TESTING_MAIN_args", "___,--secret=spooky,--no-salt,--time-code=7");

        main();

        assert_eq!(
            env::var("_JUST_TESTING_MAIN_msg")?,
            "7:4ysptJn/m3dPxisFiC36xbacV02Nf32pCwrJ18KXOcs="
        );

        Ok(())
    }
}
