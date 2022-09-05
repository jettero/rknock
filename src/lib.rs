use std::env;
use std::fs;
use std::path::Path;

use data_encoding::BASE64;
use sha2::{Digest, Sha256};

pub fn read_from_file_sometimes(blah: &str) -> String {
    let blah_str: String = blah.to_string();

    if blah_str.starts_with('@') {
        let fname = &blah_str[1..];
        return fs::read_to_string(fname)
            .expect("couldn't read file")
            .trim()
            .to_string();
    }

    blah_str
}

pub struct HMACFrobnicator {
    key: String,
}

impl HMACFrobnicator {
    pub fn new(key: &str) -> Self {
        HMACFrobnicator {
            key: read_from_file_sometimes(key),
        }
    }

    pub fn signature(&mut self, msg: &str) -> String {
        let internal = format!("{}:{}", msg, self.key);
        let mut hasher = Sha256::new();
        hasher.update(internal.as_bytes());
        let res = hasher.finalize(); // GenericArray<u8, usize>
        BASE64.encode(&res[..])
    }

    pub fn sign(&mut self, msg: &str) -> String {
        format!("{}:{}", msg, self.signature(msg))
    }

    pub fn verify(&mut self, msg: &str) -> Result<String, String> {
        let buf = msg.as_bytes();
        let mut mpart: Option<&[u8]> = None;
        let mut spart: Option<&[u8]> = None;
        for (i, &v) in buf.iter().enumerate() {
            if v == b':' {
                mpart = Some(&buf[..i]);
                spart = Some(&buf[i + 1..]);
                break;
            }
        }

        match (mpart, spart) {
            (Some(m), Some(s)) => {
                let lhs = String::from_utf8_lossy(m);
                let rhs = String::from_utf8_lossy(s);
                if self.signature(&lhs) == rhs {
                    Ok(lhs.to_string())
                } else {
                    Err("invalid signature".to_owned())
                }
            }
            _ => Err("invalid message format".to_owned()),
        }
    }
}

pub fn config_filez(env_override_prefix: &str) -> Vec<String> {
    if let Ok(v) = env::var(format!("{env_override_prefix}_CONFIG_SEARCH")) {
        return v.split(",").map(|a| a.to_string()).collect::<Vec<String>>();
    }

    let mut ret: Vec<String> = vec![
        // TODO: we're carefully platform independent by using config_dir and
        // Path::*, but then we do this??  Yeah, well, these programs probably
        // only work on loonix anyway. Also, config_dir doesn't help with these. Meh.
        "/etc/rknock.toml".to_string(),
        "/etc/rknock/config.toml".to_string(),
    ];

    if let Some(v) = dirs::config_dir() {
        ret.push(
            Path::new(&v)
                .join("rknock")
                .join("config.toml")
                .to_string_lossy()
                .to_string(),
        )
    }

    if let Ok(v) = env::var("HOME") {
        ret.push(Path::new(&v).join(".rknock.toml").to_string_lossy().to_string())
    }

    ret
}

#[macro_export]
macro_rules! is_default {
    ($matches:expr, $field:literal) => {
        match $matches.value_source($field) {
            Some(ValueSource::DefaultValue) => true,
            _ => false,
        }
    };
}

#[macro_export]
macro_rules! grok_setting {
    ($matches:expr, $settings:expr, $field:literal, $T:ty) => {
        match $matches.value_source($field) {
            Some(ValueSource::CommandLine) => $matches.get_one::<$T>($field).expect("works").to_owned(),
            Some(ValueSource::DefaultValue) => match $settings.get::<$T>($field) {
                Ok(v) => v,
                Err(_) => $matches.get_one::<$T>($field).expect("works").to_owned(),
            },
            _ => todo!(),
        }
    };
}

//---------=: TEST
#[cfg(test)]
mod tests {
    use super::*;

    /* echo -n 1234:; echo -n "1234:secret key" | sha256sum | cut -d' ' -f1 \
         | xxd -r -p | uuencode -m supz | head -n 2 | tail -n 1
       1234:iKC5sOqv+cjt3IG3qfQ/B4Xwyvz7069Zl7hGN+7ea2E=
    */
    static KNOWN: &'static str = "1234:iKC5sOqv+cjt3IG3qfQ/B4Xwyvz7069Zl7hGN+7ea2E=";
    static K_BAD: &'static str = "1234:iKC6sOqv+cjt3IG3qfQ/B4Xwyvz7069Zl7hGN+7ea2E=";

    #[test]
    fn sign_something() {
        let mut hmt = HMACFrobnicator::new("secret key");
        let msg = hmt.sign("1234");

        assert_eq!(msg, KNOWN);
    }

    #[test]
    fn verify_something() -> Result<(), String> {
        let mut hmt = HMACFrobnicator::new("secret key");

        match hmt.verify(KNOWN) {
            Ok(_) => Ok(()),
            Err(_) => Err("should have passed".to_owned()),
        }
    }

    #[test]
    fn fail_verify_something() -> Result<(), String> {
        let mut hmt = HMACFrobnicator::new("secret key");

        // here we have to reverse the result
        match hmt.verify(K_BAD) {
            Err(_) => Ok(()),
            Ok(_) => Err("should have failed".to_owned()),
        }
    }

    #[test]
    fn config_filez_works() -> Result<(), String> {
        let k = "KNOCK_STRING_THING";
        let f1 = config_filez(k);

        assert_eq!(f1[0], "/etc/rknock.toml");

        env::set_var(format!("{k}_CONFIG_SEARCH"), "supz,mang");
        let f2 = config_filez(k);

        assert_eq!(f2[0], "supz");
        assert_eq!(f2[1], "mang");

        Ok(())
    }
}
