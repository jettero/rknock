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
}

pub fn config_file() -> String {
    let path = match dirs::config_dir() {
        Some(p) => Path::new(&p).join("rknock").join("config.toml"),
        None => match dirs::home_dir() {
            Some(p) => Path::new(&p).join(".rknock.toml"),
            None => match env::var("HOME") {
                Ok(p) => Path::new(&p).join(".rknock.toml"),
                Err(_) => Path::new(".").join("rknock.toml"),
            },
        },
    };
    path.to_string_lossy().to_string()
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
