use md5::{Digest as _, Md5};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

const DS_SALT: &str = "6s25p5ox5y14umn1p61aqyyvbvvl3lrt";
const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const RANDOM_STRING_LENGTH: usize = 6;

fn random_string() -> String {
    let mut rng = rand::thread_rng();
    (0..RANDOM_STRING_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}

fn hash(string: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(string.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn generate_ds() -> String {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let random = random_string();
    let hash = hash(&format!("salt={}&t={}&r={}", DS_SALT, time, random));

    format!("{},{},{}", time, random, hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_string() {
        let s = random_string();
        assert_eq!(s.len(), RANDOM_STRING_LENGTH);
        assert!(s.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_ds_generation() {
        const HASH_LENGTH: usize = 32;

        let ds = generate_ds();
        let parts: Vec<&str> = ds.split(',').collect();

        assert_eq!(parts.len(), 3);

        assert!(parts[0].parse::<u64>().is_ok());

        assert_eq!(parts[1].len(), RANDOM_STRING_LENGTH);
        assert!(parts[1].chars().all(|c| c.is_ascii_alphanumeric()));

        assert_eq!(parts[2].len(), HASH_LENGTH);
        assert!(parts[2].chars().all(|c| c.is_ascii_hexdigit()));
    }
}
