//! Keyboard shortcuts, as a value a settings file can hold.
//!
//! A [`Keybind`] is a set of held modifiers plus at most one ordinary key —
//! `Super+Space`, `Ctrl+Alt+D`, or just `Super` on its own. It exists here, in
//! the config crate, because a shortcut has to survive a round trip between two
//! programs that have no types in common: the settings window records it from a
//! `winit` keyboard event, and the daemon that acts on it reads raw evdev
//! codes. Neither vocabulary can be the one on disk, so this is a third:
//! [`KeyCode`], a closed list of the keys a shortcut is allowed to name.
//!
//! ```ignore
//! use crownos_config::{Keybind, KeyCode, Mods};
//!
//! let bind: Keybind = "Super+Space".parse().unwrap();
//! assert_eq!(bind.key, Some(KeyCode::Space));
//! assert!(bind.mods.meta);
//! assert_eq!(bind.to_string(), "Super+Space");
//! ```
//!
//! # Why it is a string on disk
//!
//! `Keybind` serialises as its [`Display`] form rather than as a struct, so
//! `input.ron` says `dictation_hotkey: "Super+Space"` and not a nested record of
//! four booleans and an enum. Every other value in this crate is already
//! something a person can edit in place, and a shortcut written the way it is
//! spoken keeps that true. The cost is that a typo — `"Supper+Space"` — is a
//! parse error rather than a silently ignored field, which [`load`](crate::load)
//! turns into "this section falls back to its defaults".
//!
//! # Translating in and out
//!
//! Neither direction lives here, because neither crate's vocabulary belongs in
//! this one:
//!
//! * *In*, from a UI toolkit: [`KeyCode::from_code`] takes the W3C
//!   `KeyboardEvent.code` name (`"KeyA"`, `"Space"`, `"ArrowLeft"`), which is
//!   what `winit`, the web and every toolkit built on either already speak.
//! * *Out*, to whatever performs the grab: match on the [`KeyCode`]. It is a
//!   closed enum, so a consumer that forgets a key does not compile.

use core::fmt;
use core::str::FromStr;

use serde::de::{Error as _, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The modifier keys a chord holds down.
///
/// Four booleans rather than a bitflag set: this type is part of a config
/// schema, and `mods.meta` reads better at a call site than a `contains` call
/// against a constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods {
    pub meta: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Mods {
    pub const NONE: Self = Self {
        meta: false,
        ctrl: false,
        alt: false,
        shift: false,
    };

    pub const META: Self = Self {
        meta: true,
        ..Self::NONE
    };

    pub const fn is_empty(self) -> bool {
        !self.meta && !self.ctrl && !self.alt && !self.shift
    }

    fn names(self) -> impl Iterator<Item = &'static str> {
        [
            self.meta.then_some("Super"),
            self.ctrl.then_some("Ctrl"),
            self.alt.then_some("Alt"),
            self.shift.then_some("Shift"),
        ]
        .into_iter()
        .flatten()
    }

    fn set_named(&mut self, name: &str) -> Option<()> {
        let slot = match name {
            n if eq(n, "Super") || eq(n, "Meta") || eq(n, "Cmd") || eq(n, "Win") => &mut self.meta,
            n if eq(n, "Ctrl") || eq(n, "Control") => &mut self.ctrl,
            n if eq(n, "Alt") || eq(n, "Option") => &mut self.alt,
            n if eq(n, "Shift") => &mut self.shift,
            _ => return None,
        };
        *slot = true;
        Some(())
    }
}

fn eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

// --- MARK: Keys ---

/// Declare the closed list of keys a shortcut may name.
///
/// Each row is `Variant => "<W3C code>", "<written form>"`. The code is what a
/// toolkit reports and is only ever matched, never shown; the written form is
/// what appears in `input.ron` and on screen, and is what [`FromStr`] parses.
macro_rules! keys {
    ($($variant:ident => $code:literal, $label:literal;)*) => {
        /// One ordinary (non-modifier) key.
        ///
        /// Deliberately not every key a keyboard has. A shortcut that named
        /// `LaunchMail` or a dead key would be one no consumer could reasonably
        /// translate, and every key here is one that exists on the keyboards
        /// CrownOS runs on, in the same place, under the same name.
        ///
        /// Closed on purpose — no `#[non_exhaustive]`. A consumer translating
        /// these into its own vocabulary (evdev codes, say) writes one match
        /// arm per key, and adding a key here should stop that consumer from
        /// compiling until it has said what the new key means.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum KeyCode {
            $(
                #[doc = concat!("The `", $label, "` key.")]
                $variant,
            )*
        }

        impl KeyCode {
            /// Every key a shortcut may name, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)*];

            /// How this key is written — in `input.ron`, and on screen.
            pub const fn label(self) -> &'static str {
                match self {
                    $(Self::$variant => $label,)*
                }
            }

            /// The W3C `KeyboardEvent.code` name for this key.
            pub const fn code(self) -> &'static str {
                match self {
                    $(Self::$variant => $code,)*
                }
            }

            /// The key a W3C `code` names, or `None` for one no shortcut may
            /// use.
            ///
            /// This is the door a UI toolkit comes in through: `winit`, the web
            /// and everything built on either report exactly these names, so a
            /// recorder has nothing to translate.
            pub fn from_code(code: &str) -> Option<Self> {
                match code {
                    $($code => Some(Self::$variant),)*
                    _ => None,
                }
            }

            /// The key written as `label`, case-insensitively.
            pub fn from_label(label: &str) -> Option<Self> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|key| eq(key.label(), label))
            }
        }
    };
}

keys! {
    A => "KeyA", "A";
    B => "KeyB", "B";
    C => "KeyC", "C";
    D => "KeyD", "D";
    E => "KeyE", "E";
    F => "KeyF", "F";
    G => "KeyG", "G";
    H => "KeyH", "H";
    I => "KeyI", "I";
    J => "KeyJ", "J";
    K => "KeyK", "K";
    L => "KeyL", "L";
    M => "KeyM", "M";
    N => "KeyN", "N";
    O => "KeyO", "O";
    P => "KeyP", "P";
    Q => "KeyQ", "Q";
    R => "KeyR", "R";
    S => "KeyS", "S";
    T => "KeyT", "T";
    U => "KeyU", "U";
    V => "KeyV", "V";
    W => "KeyW", "W";
    X => "KeyX", "X";
    Y => "KeyY", "Y";
    Z => "KeyZ", "Z";

    Digit0 => "Digit0", "0";
    Digit1 => "Digit1", "1";
    Digit2 => "Digit2", "2";
    Digit3 => "Digit3", "3";
    Digit4 => "Digit4", "4";
    Digit5 => "Digit5", "5";
    Digit6 => "Digit6", "6";
    Digit7 => "Digit7", "7";
    Digit8 => "Digit8", "8";
    Digit9 => "Digit9", "9";

    F1 => "F1", "F1";
    F2 => "F2", "F2";
    F3 => "F3", "F3";
    F4 => "F4", "F4";
    F5 => "F5", "F5";
    F6 => "F6", "F6";
    F7 => "F7", "F7";
    F8 => "F8", "F8";
    F9 => "F9", "F9";
    F10 => "F10", "F10";
    F11 => "F11", "F11";
    F12 => "F12", "F12";

    Space => "Space", "Space";
    Enter => "Enter", "Enter";
    Tab => "Tab", "Tab";
    Escape => "Escape", "Escape";
    Backspace => "Backspace", "Backspace";
    Delete => "Delete", "Delete";
    Insert => "Insert", "Insert";
    Home => "Home", "Home";
    End => "End", "End";
    PageUp => "PageUp", "PageUp";
    PageDown => "PageDown", "PageDown";
    CapsLock => "CapsLock", "CapsLock";

    ArrowUp => "ArrowUp", "Up";
    ArrowDown => "ArrowDown", "Down";
    ArrowLeft => "ArrowLeft", "Left";
    ArrowRight => "ArrowRight", "Right";

    Minus => "Minus", "Minus";
    Equal => "Equal", "Equal";
    LeftBracket => "BracketLeft", "LeftBracket";
    RightBracket => "BracketRight", "RightBracket";
    Backslash => "Backslash", "Backslash";
    Semicolon => "Semicolon", "Semicolon";
    Quote => "Quote", "Quote";
    Backquote => "Backquote", "Backquote";
    Comma => "Comma", "Comma";
    Period => "Period", "Period";
    Slash => "Slash", "Slash";
}

impl fmt::Display for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// --- MARK: Keybind ---

/// How a shortcut with nothing bound to it is written.
const UNBOUND: &str = "None";

/// A keyboard shortcut: the modifiers held, and at most one ordinary key.
///
/// A bind with a [`key`](Self::key) is the ordinary kind — `Super+Space`. A
/// bind with modifiers and no key is a *modifier-only* chord: `Super` on its
/// own, which is a real thing to want for push-to-talk, where the shortcut is
/// held rather than struck. A bind with neither is [`NONE`](Self::NONE), the
/// "not bound to anything" state; consumers should treat it as the feature
/// being unreachable rather than as an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Keybind {
    pub mods: Mods,
    pub key: Option<KeyCode>,
}

impl Keybind {
    /// Bound to nothing.
    pub const NONE: Self = Self {
        mods: Mods::NONE,
        key: None,
    };

    /// `Super+Space`, the CrownOS push-to-talk default.
    pub const SUPER_SPACE: Self = Self {
        mods: Mods::META,
        key: Some(KeyCode::Space),
    };

    /// `Super+Ctrl`, the CrownOS launchpad default.
    ///
    /// Modifier-only, like a push-to-talk chord: the launchpad is opened and
    /// closed by the same shortcut, and a chord with no ordinary key in it
    /// cannot collide with anything an application has bound. Written
    /// `Super+Ctrl` rather than `Ctrl+Super` because [`Display`] prints
    /// modifiers in the freedesktop order — the two parse identically.
    pub const SUPER_CTRL: Self = Self {
        mods: Mods {
            meta: true,
            ctrl: true,
            alt: false,
            shift: false,
        },
        key: None,
    };

    /// Build a chord.
    pub const fn new(mods: Mods, key: Option<KeyCode>) -> Self {
        Self { mods, key }
    }

    /// Whether this binds nothing at all.
    pub const fn is_empty(self) -> bool {
        self.mods.is_empty() && self.key.is_none()
    }
}

impl fmt::Display for Keybind {
    /// `Super+Space`, `Ctrl+Alt+Delete`, `Super`, or `None`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str(UNBOUND);
        }
        let mut first = true;
        for part in self.mods.names().chain(self.key.map(KeyCode::label)) {
            if !first {
                f.write_str("+")?;
            }
            f.write_str(part)?;
            first = false;
        }
        Ok(())
    }
}

/// What went wrong reading a shortcut out of a config file.
///
/// One variant, because there is exactly one thing a caller can do about any of
/// it — the message names the offending part so the user can fix the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseKeybindError {
    part: String,
    reason: &'static str,
}

impl fmt::Display for ParseKeybindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.reason, self.part)
    }
}

impl std::error::Error for ParseKeybindError {}

impl FromStr for Keybind {
    type Err = ParseKeybindError;

    /// Parse `Super+Space`, `ctrl+alt+delete`, `Super`, or `None`.
    ///
    /// Case-insensitive throughout, and tolerant of spaces around the `+`, so a
    /// hand-edited `Ctrl + Alt + D` is read the way it was obviously meant.
    /// Order does not matter on the way in; [`Display`] always prints the
    /// canonical one.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        if text.is_empty() || eq(text, UNBOUND) {
            return Ok(Self::NONE);
        }

        let mut bind = Self::NONE;
        for part in text.split('+') {
            let part = part.trim();
            if part.is_empty() {
                return Err(ParseKeybindError {
                    part: text.to_owned(),
                    reason: "empty part in shortcut",
                });
            }
            if bind.mods.set_named(part).is_some() {
                continue;
            }
            let Some(key) = KeyCode::from_label(part) else {
                return Err(ParseKeybindError {
                    part: part.to_owned(),
                    reason: "not a modifier or a key a shortcut may use",
                });
            };
            // A chord holds one ordinary key; a second is a typo, not an
            // intention, and silently keeping either one would bind something
            // the user did not write.
            if bind.key.is_some() {
                return Err(ParseKeybindError {
                    part: text.to_owned(),
                    reason: "more than one key in shortcut",
                });
            }
            bind.key = Some(key);
        }
        Ok(bind)
    }
}

impl Serialize for Keybind {
    /// As the written form — see the module docs for why this is a string and
    /// not a struct.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Keybind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Owned rather than borrowed: RON hands back a `Cow` for any string
        // carrying an escape, and a borrowed-only impl would reject those.
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(|_| {
            D::Error::invalid_value(
                Unexpected::Str(&text),
                &"a shortcut such as \"Super+Space\"",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one thing this type exists to do: survive the trip to disk and back
    /// as the string it is written as.
    #[test]
    fn a_shortcut_round_trips_through_its_written_form() {
        for (bind, written) in [
            (Keybind::SUPER_SPACE, "Super+Space"),
            (Keybind::NONE, "None"),
            (Keybind::new(Mods::META, None), "Super"),
            (
                Keybind::new(
                    Mods {
                        ctrl: true,
                        alt: true,
                        ..Mods::NONE
                    },
                    Some(KeyCode::Delete),
                ),
                "Ctrl+Alt+Delete",
            ),
            (
                Keybind::new(
                    Mods {
                        meta: true,
                        shift: true,
                        ..Mods::NONE
                    },
                    Some(KeyCode::S),
                ),
                "Super+Shift+S",
            ),
        ] {
            assert_eq!(bind.to_string(), written);
            assert_eq!(written.parse::<Keybind>().unwrap(), bind);
        }
    }

    /// Order and case are the writer's business; the canonical form is this
    /// type's.
    #[test]
    fn parsing_is_forgiving_and_printing_is_not() {
        for written in ["shift+super+s", "SUPER + SHIFT + S", "Super+Shift+S"] {
            assert_eq!(
                written.parse::<Keybind>().unwrap().to_string(),
                "Super+Shift+S"
            );
        }
        // The aliases a person is likely to reach for.
        assert_eq!(
            "cmd+space".parse::<Keybind>().unwrap(),
            Keybind::SUPER_SPACE
        );
        assert_eq!(
            "win+space".parse::<Keybind>().unwrap(),
            Keybind::SUPER_SPACE
        );
        assert_eq!(
            "control+option+d".parse::<Keybind>().unwrap(),
            Keybind::new(
                Mods {
                    ctrl: true,
                    alt: true,
                    ..Mods::NONE
                },
                Some(KeyCode::D),
            )
        );
    }

    /// An unbound shortcut has one spelling on the way out and several on the
    /// way in — an empty string is what a half-finished hand edit leaves.
    #[test]
    fn nothing_bound_is_a_value_and_not_an_error() {
        assert!(Keybind::NONE.is_empty());
        assert_eq!("".parse::<Keybind>().unwrap(), Keybind::NONE);
        assert_eq!("  ".parse::<Keybind>().unwrap(), Keybind::NONE);
        assert_eq!("none".parse::<Keybind>().unwrap(), Keybind::NONE);
        assert!(!Keybind::SUPER_SPACE.is_empty());
    }

    #[test]
    fn a_shortcut_that_cannot_be_honoured_is_rejected() {
        for written in ["Supper+Space", "Super+", "Super+Space+Enter", "Super+Fn"] {
            assert!(
                written.parse::<Keybind>().is_err(),
                "{written} should not parse"
            );
        }
    }

    /// The door a UI toolkit comes in through, and the closed list behind it.
    #[test]
    fn keys_translate_from_the_w3c_code_names() {
        assert_eq!(KeyCode::from_code("KeyA"), Some(KeyCode::A));
        assert_eq!(KeyCode::from_code("Space"), Some(KeyCode::Space));
        assert_eq!(KeyCode::from_code("ArrowLeft"), Some(KeyCode::ArrowLeft));
        assert_eq!(
            KeyCode::from_code("BracketLeft"),
            Some(KeyCode::LeftBracket)
        );
        // Modifiers are held, not struck: they are `Mods`, never a `KeyCode`.
        assert_eq!(KeyCode::from_code("MetaLeft"), None);
        assert_eq!(KeyCode::from_code("ShiftRight"), None);
        // And a key no shortcut may use stays unusable rather than becoming a
        // near-miss.
        assert_eq!(KeyCode::from_code("LaunchMail"), None);
    }

    /// Every key is reachable by both of its names, and no two share either.
    #[test]
    fn every_key_has_one_code_and_one_written_form() {
        for &key in KeyCode::ALL {
            assert_eq!(KeyCode::from_code(key.code()), Some(key));
            assert_eq!(KeyCode::from_label(key.label()), Some(key));
        }
        for &key in KeyCode::ALL {
            let codes = KeyCode::ALL.iter().filter(|k| k.code() == key.code());
            let labels = KeyCode::ALL.iter().filter(|k| eq(k.label(), key.label()));
            assert_eq!(codes.count(), 1, "duplicate code for {key:?}");
            assert_eq!(labels.count(), 1, "duplicate written form for {key:?}");
        }
        // And no key is written the way a modifier is, which would make a
        // chord like `Super+Super` parse as something.
        for &key in KeyCode::ALL {
            let mut mods = Mods::NONE;
            assert!(
                mods.set_named(key.label()).is_none(),
                "{key:?} is written like a modifier"
            );
        }
    }
}
