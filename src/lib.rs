use std::fs;

use data_encoding::BASE64;
use sha2::{Digest, Sha256};

pub struct HMACFrobnicator {
    key: String,
}

impl HMACFrobnicator {
    pub fn new(key: &str) -> Self {
        let mut key_str: String = key.to_string();
        if key_str.starts_with('@') {
            let fname = &key_str[1..];
            key_str = fs::read_to_string(fname).expect("couldn't read file");
            key_str = key_str.trim().to_string();
        }
        HMACFrobnicator { key: key_str }
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
