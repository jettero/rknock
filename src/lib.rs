use ring::hmac;
use std::fs;

use data_encoding::BASE64;
use sha2::{Digest, Sha256};

pub fn get_key(mut key_str: String) -> hmac::Key {
    if key_str.starts_with("@") {
        let fname = &key_str[1..];
        key_str = fs::read_to_string(fname).expect("couldn't read file");
        key_str = key_str.trim().to_string();
    }

    return hmac::Key::new(hmac::HMAC_SHA256, key_str.as_bytes());
}


pub struct HMACFrobnicator {
    key: String
}

impl HMACFrobnicator {
    pub fn new(key: &str) -> Self {
        let mut key_str: String = key.to_string();
        if key_str.starts_with("@") {
            let fname = &key_str[1..];
            key_str = fs::read_to_string(fname).expect("couldn't read file");
            key_str = key_str.trim().to_string();
        }
        HMACFrobnicator {
            key: key_str,
        }
    }

    pub fn sign(&mut self, msg: &str) -> String {
        let internal = format!("{}:{}", msg, self.key);
        let mut hasher = Sha256::new();
        hasher.update(internal.as_bytes());
        let res = hasher.finalize(); // GenericArray<u8, usize>
        return format!("{}:{}", msg, BASE64.encode(&res[..]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sign_something() {
        let mut hmt = HMACFrobnicator::new("secret key");
        let msg = hmt.sign("1234");

        assert_eq!(msg, "1234:iKC5sOqv+cjt3IG3qfQ/B4Xwyvz7069Zl7hGN+7ea2E=");
        /* echo -n 1234:; echo -n "1234:secret key" | sha256sum | cut -d' ' -f1 \
             | xxd -r -p | uuencode -m supz | head -n 2 | tail -n 1
           1234:iKC5sOqv+cjt3IG3qfQ/B4Xwyvz7069Zl7hGN+7ea2E=
        */
    }
}
