use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::{
    hash::{DefaultHasher, Hasher},
    path::PathBuf,
};

use crate::config_dir;

/// Hash used everywhere in this crate to compare file contents. Not stable
/// across processes; it only answers "are these the same bytes?" within a run.
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

/// Full path of a section's file: `config_dir()/<section>.ron`.
///
/// `section` is the lowercase menu name - see the `SECTION` constants in
/// [`schema`], e.g. [`schema::Wifi::SECTION`].
pub fn path_for(section: &str) -> PathBuf {
    config_dir().join(format!("{section}.ron"))
}

/// section -> hash of the bytes [`save`](crate::save) last wrote for it.
static LAST_WRITTEN: LazyLock<Mutex<HashMap<String, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Claim `hash` as "written by us". [`save`](crate::save) does this for you;
/// call it directly only if you bypass `save` and write a section yourself.
pub fn record_written(section: &str, hash: u64) {
    let mut map = LAST_WRITTEN
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    map.insert(section.to_owned(), hash);
}

/// Hash of the bytes this process last wrote for `section`. `None` means
/// whatever is on disk came from somewhere else.
pub fn last_written(section: &str) -> Option<u64> {
    let map = LAST_WRITTEN
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    map.get(section).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_bytes_consistency() {
        let input = b"hello world";

        let hash1 = hash_bytes(input);
        let hash2 = hash_bytes(input);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_bytes_different_inputs() {
        let hash1 = hash_bytes(b"section_a");
        let hash2 = hash_bytes(b"section_b");

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_bytes_empty() {
        let hash = hash_bytes(b"");
        assert_eq!(hash, hash_bytes(&[]));
    }

    #[test]
    fn test_path_for() {
        let section = "wifi";
        let path = path_for(section);

        // Path should end with "<section>.ron"
        assert_eq!(path.file_name().unwrap(), "wifi.ron");

        // Path should start with `config_dir()`
        assert!(path.starts_with(config_dir()));
    }
}
