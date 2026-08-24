//! The shared shape of CrownOS settings.
//!
//! Every app in the desktop reads and writes the same types from the same
//! files, so the settings panel, the status bar and a shell script all agree
//! on what `~/.config/crownos/sound.ron` means. Each type carries the section name it
//! belongs to:
//!
//! ```ignore
//! use crownos_config::schema::Sound;
//!
//! let sound: Sound = crownos_config::load(Sound::SECTION); // ~/.config/crownos/sound.ron
//! ```
//!
//! Adding a field to one of these structs is backwards compatible for readers
//! of *newer* files only if the field has a default; prefer extending a struct
//! over introducing a parallel one, and remember that a value which fails to
//! parse silently falls back to [`Default`].
//!
//! # Keys
//!
//! Each struct is declared with [`section!`](crate::section), which also emits
//! a [`Key`](crate::Key) type per field and an enum listing them all — that is
//! what [`subscribe_key`](crate::subscribe_key) takes instead of a section
//! string plus a field name.
//!
//! The key enums are re-exported here (`AppearanceKey`, `SoundKey`, ...) but
//! the per-field key types are not: `Enabled` exists in three sections, so they
//! stay behind their module and read as `wifi::Enabled` or
//! `bluetooth::Enabled` at the call site.
//!
//! ```ignore
//! use crownos_config::schema::{appearance, AppearanceKey};
//!
//! let sub = crownos_config::subscribe_key(appearance::DarkMode, |on| set_theme(on));
//! for key in AppearanceKey::ALL {
//!     println!("{key}"); // appearance.dark_mode, appearance.accent, ...
//! }
//! ```

pub mod appearance;
pub mod bluetooth;
pub mod compositor;
pub mod display;
pub mod input;
pub mod keybinds;
pub mod notification;
pub mod power;
pub mod sound;
pub mod wifi;

pub use appearance::{AccentColor, AnimationProfile, Appearance, AppearanceKey};
pub use bluetooth::{Bluetooth, BluetoothKey};
pub use compositor::{
    Compositor, CompositorKey, LayoutMode, OutputSetting, OutputTransform, WindowRule,
};
pub use display::{Display, DisplayKey, DisplayScale};
pub use input::{Input, InputKey};
pub use keybinds::{Binding, Keybinds, KeybindsKey};
pub use notification::{Notifications, NotificationsKey};
pub use power::{Power, PowerKey, PowerProfile};
pub use sound::{Sound, SoundKey};
pub use wifi::{Wifi, WifiKey};
