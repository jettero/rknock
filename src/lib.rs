use ring::hmac;
use std::fs;

pub fn get_key(mut key_str: String) -> hmac::Key {
    if key_str.starts_with("@") {
        let fname = &key_str[1..];
        key_str = fs::read_to_string(fname).expect("couldn't read file");
        key_str = key_str.trim().to_string();
    }

    return hmac::Key::new(hmac::HMAC_SHA256, key_str.as_bytes());
}
