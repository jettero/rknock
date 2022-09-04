use std::collections::HashMap;
use std::error::Error;
use std::process::{Command, ExitCode, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

extern crate strfmt;
use strfmt::strfmt;

extern crate log;
use env_logger::Env;
use log::{debug, error, info, LevelFilter};
use syslog::{BasicLogger, Facility, Formatter3164};

extern crate lru;
use lru::LruCache;

use clap::{arg, value_parser, App, ArgAction, ValueSource};
use config::Config;

use tokio::net::UdpSocket;
use tokio::task;

use rlib::{config_filez, grok_setting, is_default, read_from_file_sometimes, HMACFrobnicator};

async fn allow_ip(src: &String, command: &str) {
    let vars = HashMap::from([("ip".to_string(), src.to_string())]);
    let cmd = strfmt(command, &vars).unwrap();

    debug!("exec({}) ip={}", cmd, src);

    let debug_sleep = std::time::Duration::from_millis(
        std::env::var("KNOCK_DOOR_DEBUG_DELAY")
            .unwrap_or_else(|_| "0".to_string())
            .parse::<u64>()
            .unwrap_or(0),
    );

    debug!("sleep({})", debug_sleep.as_millis());
    if debug_sleep.as_millis() > 0 {
        // I can't think of anything that would make this useful outside debugs
        // but decided to leave KNOCK_DOOR_DEBUG_DELAY exposed regardless.
        std::thread::sleep(debug_sleep);
    }

    let child = Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to fork child process");

    let output = child.wait_with_output().expect("failed to wait for child");

    if !output.status.success() {
        error!(
            "fail({}) {}\n  stdout: {}\n  stderr: {}",
            &cmd,
            output.status, // e.g., "exit status: 1"
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        return;
    }

    info!("allowed {}", src);
}

async fn process_payload(
    amt: usize,
    src_wp: &String,
    buf: &[u8],
    hf: &mut HMACFrobnicator,
    nonce_cache: &mut LruCache<String, bool>,
) -> bool {
    let msg = String::from_utf8_lossy(buf);

    debug!("{} sent {} bytes, {:?}", src_wp, amt, msg); // {:?} has its own quotes

    match hf.verify(&msg) {
        Ok(snonce) => {
            if nonce_cache.get(&snonce).is_some() {
                debug!("rejecting reused nonce");
                // Arguably, an attacker could flood this cache with valid
                // nonces and roll this one right off so it could be reused;
                // but ... then in that case they can generate valid nonces, so
                // who really cares if they can flood this cache?
                return false;
            }
            nonce_cache.put(snonce.to_owned(), true);

            let epos = snonce.find('$').unwrap_or(snonce.len());
            let tnonce = snonce[..epos].to_string();
            match tnonce.parse::<u64>() {
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
            }
        }
        Err(_) => {
            debug!("invalid signature");
            return false;
        }
    };

    info!("{} VERIFIED", src_wp);
    true
}

#[tokio::main]
async fn listen_to_msgs(
    listen: String,
    hf: &mut HMACFrobnicator,
    command: &str,
    nonce_cache: &mut LruCache<String, bool>,
) {
    let mut buf = [0; 256];
    let socket = UdpSocket::bind(listen.as_str()).await.expect("couldn't bind to socket");

    // we use listen.as_str() above so we don't "move" listen to the bind()
    // if we did, we'd get an error about using listen after move on the next line
    info!("listening to {}", listen);

    loop {
        let (amt, src_addr) = socket.recv_from(&mut buf).await.expect("couldn't read from buffer");
        let src_with_port = src_addr.to_string();
        let src = src_with_port[..src_with_port.find(':').unwrap()].to_string();

        if process_payload(amt, &src_with_port, &buf[..amt], hf, nonce_cache).await {
            let a = src.to_owned();
            let b = command.to_owned();

            task::spawn(async move { allow_ip(&a, &b).await });
        }
    }
}

fn get_args() -> Result<(bool, bool, String, String, String), Box<dyn Error>> {
    let matches = App::new("door")
        .version("0.0.0")
        .author("Paul Miller <paul@jettero.pl>")
        .about("Watches the doors and listens for the secret codes")
        .arg(arg!(syslog: -S --syslog "log events and info to syslog instead of stdout").action(ArgAction::SetTrue))
        .arg(arg!(verbose: -v --verbose "print DEBUG level events instead of INFO").action(ArgAction::SetTrue))
        .arg(
            arg!(config: -C --config <CONFIG> "read this config file for settings")
            .value_parser(value_parser!(String))
            .multiple(true)
            .required(false) // I hate this:
            .default_values(&config_filez().iter().map(|a| a.as_str()).collect::<Vec<&str>>())
        )
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
            .default_value("secret")
        )
        .arg(
            arg!(command: -c --command <SHELL_COMMAND> "The command to execute after a verified message is received. \
            Can also be set via KNOCK_DOOR_COMMAND. Note that the source IP will be passed via format!() \
            to this command string, so brace characters must be escaped (doubled) and the command should contain \
            {ip} if applicable to the command. A leading '@' character
            indicates the this value is a file from which to read the
            command.")
            .value_parser(value_parser!(String))
            .required(false)
            .default_value("sudo nft add element inet firewall knock {{ {ip} timeout 5s }}")
        )
        .get_matches();

    let filez = matches.get_many::<String>("config").expect("defaulted by clap");
    let def = is_default!(matches, "config");
    let mut config = Config::builder();
    for item in filez {
        config = config.add_source(config::File::with_name(item).required(!def));
    }
    config = config.add_source(config::Environment::with_prefix("KNOCK"));

    let settings = config.build()?;

    let verbose: bool = grok_setting!(matches, settings, "verbose", bool);
    let syslog: bool = grok_setting!(matches, settings, "syslog", bool);
    let key: String = grok_setting!(matches, settings, "secret", String);
    let listen: String = grok_setting!(matches, settings, "listen", String);
    let command: String = read_from_file_sometimes(&grok_setting!(matches, settings, "command", String));

    Ok((verbose, syslog, key, listen, command))
}

fn main() -> ExitCode {
    let (verbose, syslog, key_str, listen, command) = match get_args() {
        Ok(v) => v,
        Err(error) => {
            eprintln!("error building config: {error:?}");
            return ExitCode::from(27);
        }
    };
    let mut hf = HMACFrobnicator::new(&key_str);
    let mut nonce_cache: LruCache<String, bool> = LruCache::new(100);

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
            // TODO: KNOCK_DOOR_LOG_LEVEL and LOG_STYLE should probably be available via configs...
            // do we even need Env::default? what about env_logger itself?
            .filter_or("KNOCK_DOOR_LOG_LEVEL", if verbose { "debug" } else { "info" })
            .write_style_or("KNOCK_DOOR_LOG_STYLE", "always");

        env_logger::init_from_env(env);
    }

    listen_to_msgs(listen, &mut hf, &command, &mut nonce_cache);

    ExitCode::from(0)
}
