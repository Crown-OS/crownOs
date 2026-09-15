//! Text input: for now, dictation.
//!
//! The section is called `input` rather than `dictation` because it is the file
//! behind the settings panel's *Input* page, and that page is the home for
//! anything that turns a person into text — the same way macOS files Dictation
//! under Keyboard → Text Input. Dictation is simply the only thing in it today,
//! which is why every field is prefixed: a later `input.ron` gaining key-repeat
//! or an input-method setting should not have to rename what is already there.
//!
//! The consumer is [crowndictator], which reads this file at startup and
//! follows it live — turning the feature off releases its keyboard grab, and
//! changing the shortcut re-arms it without a restart.
//!
//! [crowndictator]: https://github.com/crown-os/crowndictator

use crate::keybind::Keybind;

crate::section! {
    pub struct Input in "input", keys InputKey {
        /// Whether push-to-talk dictation runs at all.
        ///
        /// Off means the daemon stays resident but the shortcut does nothing
        /// and no microphone is ever opened. The shortcut and the device below
        /// are remembered rather than forgotten, so turning it back on restores
        /// the setup that was already there.
        pub dictation_enabled as DictationEnabled: bool = true,

        /// Which microphone to record from, by the name the audio host gives
        /// it, or `None` for whatever the system's default input is.
        ///
        /// `None` is the honest default: the system default follows the user's
        /// headset in and out, and pinning a name is the exception rather than
        /// the rule. A name that no longer matches any device falls back to the
        /// default rather than failing to record.
        pub dictation_microphone as DictationMicrophone: Option<String> = None,

        /// The push-to-talk shortcut: held to record, released to transcribe.
        ///
        /// Held rather than struck, which is why a modifier-only chord is a
        /// reasonable thing to put here — see [`Keybind`].
        pub dictation_hotkey as DictationHotkey: Keybind = Keybind::SUPER_SPACE,

        /// Whether to run speech recognition on the GPU when one is available.
        ///
        /// The same switch as crowndictator's `--cpu` flag, from the other
        /// direction. Falling back to the CPU is automatic when there is no
        /// usable GPU, so this only matters on a machine that has one and wants
        /// it left alone.
        pub dictation_gpu as DictationGpu: bool = true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybind::KeyCode;

    /// The default this section ships is the shortcut crowndictator has always
    /// used, so an install that has never opened the settings panel behaves
    /// exactly as it did before the file existed.
    #[test]
    fn the_default_is_push_to_talk_on_super_space() {
        let input = Input::default();

        assert!(input.dictation_enabled);
        assert_eq!(input.dictation_microphone, None);
        assert_eq!(input.dictation_hotkey.key, Some(KeyCode::Space));
        assert!(input.dictation_hotkey.mods.meta);
        assert_eq!(input.dictation_hotkey.to_string(), "Super+Space");
    }

    /// The shortcut is the one field in the schema that is not a primitive, so
    /// it is the one worth proving survives the file.
    #[test]
    fn the_shortcut_is_written_as_a_string() {
        let text = ron::ser::to_string_pretty(&Input::default(), Default::default())
            .expect("serialise input section");

        assert!(
            text.contains("dictation_hotkey: \"Super+Space\""),
            "the shortcut should be hand-editable, got:\n{text}"
        );
        assert_eq!(
            ron::from_str::<Input>(&text).expect("parse back"),
            Input::default()
        );
    }
}
