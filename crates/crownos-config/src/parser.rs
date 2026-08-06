use std::io;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::util::{hash_bytes, path_for, record_written};

/// Read a section, falling back to `T::default()`.
///
/// A missing file is treated as "not configured yet": the default value is
/// written out so the file exists for the user to edit. A file that exists but
/// fails to parse is left alone — clobbering a config the user is halfway
/// through hand-editing would be worse than ignoring it for one read.
///
/// `T: Serialize` is required only for that "materialise the default" step;
/// every settings type is round-trippable anyway.
pub fn load<T: DeserializeOwned + Serialize + Default>(section: &str) -> T {
    let path = path_for(section);
    match std::fs::read_to_string(&path) {
        Ok(text) => ron::from_str(&text).unwrap_or_default(),
        Err(_) => {
            let value = T::default();
            let _ = save(section, &value);
            value
        }
    }
}

/// Write a section as pretty RON, atomically.
///
/// The bytes go to `<file>.tmp` in the same directory and are then renamed
/// over the target, so a reader (or a watcher) never observes a half-written
/// file. Parent directories are created if missing.
///
/// The written bytes are also recorded for [echo suppression](self#echo-suppression),
/// so subscribers of this section do not get notified about this app's own write.
pub fn save<T: Serialize>(section: &str, value: &T) -> io::Result<()> {
    let text = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let path = path_for(section);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Record before the rename: the inotify event can land on the watcher
    // thread the instant the rename returns.
    record_written(section, hash_bytes(text.as_bytes()));

    let tmp = path.with_extension("ron.tmp");
    std::fs::write(&tmp, text.as_bytes())?;
    std::fs::rename(&tmp, &path)
}
