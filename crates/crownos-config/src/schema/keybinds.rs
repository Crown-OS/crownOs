//! Desktop-wide keyboard shortcuts.
//!
//! Shortcuts that belong to the desktop itself rather than to one feature —
//! the ones a compositor grabs globally and acts on no matter what has focus.
//! A shortcut that only makes sense while a particular thing is running stays
//! with that thing instead: dictation's push-to-talk chord lives in
//! [`input`](crate::schema::input), because it is meaningless with dictation
//! switched off.
//!
//! The consumer is the compositor. It is the only process that can honour a
//! global chord — every other CrownOS app is a Wayland client and only ever
//! sees keys while it is focused, which is exactly the state a launcher
//! shortcut has to work from *outside*. It reads this file live, so rebinding
//! takes effect without a restart or a re-login.

use serde::{Deserialize, Serialize};

use crate::keybind::Keybind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    /// `"Super+Q"`, `"Super+Shift+1"`, `"Super"` for a modifier-only chord.
    pub keys: String,
    /// `"close-window"`, `"spawn foot"`, `"workspace +1"`.
    pub action: String,
}

crate::section! {
    pub struct Keybinds in "keybinds", keys KeybindsKey {
        pub launcher as Launcher: Keybind = Keybind::SUPER_CTRL,
        pub custom_keybinds as CustomKeybinds: Vec<Binding> = Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybind::Mods;

    #[test]
    fn the_launchpad_opens_on_super_ctrl_by_default() {
        let keybinds = Keybinds::default();

        assert_eq!(
            keybinds.launcher.mods,
            Mods {
                meta: true,
                ctrl: true,
                alt: false,
                shift: false
            }
        );
        assert_eq!(
            keybinds.launcher.key, None,
            "a modifier-only chord cannot collide with an application's own bindings"
        );
    }

    /// The written form is what lands in the file and what the settings panel
    /// shows, and modifiers print in one fixed order however they were typed.
    #[test]
    fn the_default_is_written_super_ctrl_whichever_way_it_is_spelled() {
        assert_eq!(Keybinds::default().launcher.to_string(), "Super+Ctrl");
        assert_eq!(
            "Ctrl+Super".parse::<Keybind>().unwrap(),
            Keybinds::default().launcher
        );
    }

    #[test]
    fn the_shortcut_is_written_as_a_string() {
        let text = ron::ser::to_string_pretty(&Keybinds::default(), Default::default())
            .expect("serialise keybinds section");

        assert!(
            text.contains("launcher: \"Super+Ctrl\""),
            "the shortcut should be hand-editable, got:\n{text}"
        );
        assert_eq!(
            ron::from_str::<Keybinds>(&text).expect("parse back"),
            Keybinds::default()
        );
    }

    /// Unbinding is a value the file can hold, not a parse failure — a user who
    /// clears the field in the panel gets `"None"` written out and read back.
    #[test]
    fn an_unbound_launcher_survives_the_file() {
        let cleared = Keybinds {
            custom_keybinds: Vec::new(),
            launcher: Keybind::NONE,
        };
        let text = ron::ser::to_string_pretty(&cleared, Default::default()).expect("serialise");

        assert!(text.contains("launcher: \"None\""), "got:\n{text}");
        assert_eq!(
            ron::from_str::<Keybinds>(&text).expect("parse back"),
            cleared
        );
    }
}
