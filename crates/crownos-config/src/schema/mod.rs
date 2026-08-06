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

pub mod appearance;
pub mod bluetooth;
pub mod display;
pub mod notification;
pub mod power;
pub mod sound;
pub mod wifi;

pub use appearance::{AccentColor, Appearance};
pub use bluetooth::Bluetooth;
pub use display::{Display, DisplayScale};
pub use notification::Notifications;
pub use power::{Power, PowerProfile};
pub use sound::Sound;
pub use wifi::Wifi;
