//! On-disk configuration for CrownOS desktop apps.
//!
//! Every settings *section* is one RON file inside the CrownOS config
//! directory — `~/.config/crownos/appearance.ron`, `~/.config/crownos/wifi.ron`,
//! and so on. That flat `<section>.ron` layout is a CrownOS convention: the
//! settings menu name and the file name are the same string, so a user who
//! wants to edit "Display" by hand knows to open `~/.config/crownos/display.ron`.
//!
//! ```ignore
//! use crownos_config::{load, save, subscribe_typed};
//! use crownos_config::schema::Appearance;
//!
//! // Read (missing file -> `Default`, and the default is written out so the
//! // user has something to edit).
//! let mut appearance: Appearance = load(Appearance::SECTION);
//!
//! // Write (atomic: temp file + rename).
//! appearance.dark_mode = true;
//! save(Appearance::SECTION, &appearance).unwrap();
//!
//! // React to edits made by another app — or by `$EDITOR`.
//! let sub = subscribe_typed::<Appearance, _>(Appearance::SECTION, |new| {
//!     println!("appearance changed: {new:?}");
//! });
//! // ... dropping `sub` unregisters the callback.
//! ```
//!
//! In a [xilem] app, prefer the [`xilem_view`] module over calling
//! [`subscribe_typed`] directly — it plumbs changes into your app state
//! through the normal view-tree message path instead of a background callback.
//!
//! # Echo suppression
//!
//! The watcher and the writer share a table of "what did *we* last write".
//! When [`save`] stores a section it records a hash of the bytes; when the
//! inotify event for that same write arrives, the watcher sees a matching hash
//! and stays quiet. Without this, an app that saves on every slider tick would
//! immediately get its own write back as an external change and fight itself.

mod config;
mod parser;
pub mod schema;
mod util;
mod watch;

#[cfg(feature = "xilem")]
pub mod xilem_view;

pub use crate::config::{config_dir, CONFIG_DIR_ENV};
pub use parser::{load, save};
pub use util::{hash_bytes, last_written, path_for, record_written};
pub use watch::{subscribe, subscribe_typed, Subscription};

/// Every settings type, also reachable as `crownos_config::schema::*`.
pub use schema::{
    AccentColor, Appearance, Bluetooth, Display, DisplayScale, Notifications, Power, PowerProfile,
    Sound, Wifi,
};
