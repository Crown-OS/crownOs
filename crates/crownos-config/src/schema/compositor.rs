//! Window management: tiling, keybindings, window rules, outputs.
//!
//! How windows are *arranged*, not how they look — gaps, borders and animation
//! are in [`appearance`](crate::schema::appearance), because a user changing them
//! is changing the theme rather than the tiling.
//!
//! The consumer is [crownpositor], which reads this file live — a rebind takes
//! effect without a restart. It keeps its own compiled form of these values
//! (regexes, chords, geometry in signed pixels); this is only the on-disk
//! vocabulary, so the file stays a stable contract while the compositor's
//! internal types move around.
//!
//! [crownpositor]: https://github.com/crown-os/crownpositor

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LayoutMode {
    #[default]
    MasterStack,
    ScrollingColumns,
    Floating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OutputTransform {
    #[default]
    Normal,
    R90,
    R180,
    R270,
    Flipped,
    Flipped90,
    Flipped180,
    Flipped270,
}

/// One row of the keybinding table. Both fields are strings so the file stays
/// hand-editable; a bad row is logged and skipped rather than failing the load.
///
/// Deliberately not a [`Keybind`](crate::Keybind): the compositor's actions are
/// its own vocabulary, and the chord spellings it accepts (`"Super+Shift+Q"`)
/// are a superset of what the settings panel's single-shortcut fields need.
/// Matched at a window's first buffer commit, the earliest point `app_id`,
/// `title` and the size hints exist.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowRule {
    /// Regex, unanchored: `"blender"` matches `"org.blender.Blender"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    /// Regex.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floating: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fullscreen: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximized: Option<bool>,
    /// Zero-based workspace index on the target output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<u16>,
    /// Connector name or `"MAKE MODEL SERIAL"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// `false` opens the window without stealing focus.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<u16>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputSetting {
    /// Connector name (`"eDP-1"`) or `"MAKE MODEL SERIAL"`.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// `"2560x1440@144.000"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<OutputTransform>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<(i32, i32)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vrr: Option<bool>,
    /// Overrides the global default for workspaces created on this output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<LayoutMode>,
}

crate::section! {
    pub struct Compositor in "compositor", keys CompositorKey {
        pub layout as Layout: LayoutMode = LayoutMode::MasterStack,
        pub focus_follows_mouse as FocusFollowsMouse: bool = false,

        pub window_rules as WindowRules: Vec<WindowRule> = Vec::new(),
        pub outputs as Outputs: Vec<OutputSetting> = Vec::new(),
        pub startup as Startup: Vec<String> = Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documented_file_shape_round_trips() {
        let sample = r#"(
            layout: ScrollingColumns,
            keybinds: [
                (keys: "Super+Q", action: "close-window"),
            ],
            window_rules: [
                (app_id: "Nautilus", floating: true),
                (title: "^(Open|Save)", floating: true, focus: false),
            ],
            outputs: [
                (name: "eDP-1", scale: 2.0, position: (0, 0)),
            ],
            startup: [
                "crownbar",
                "swaybg -i /usr/share/backgrounds/crown.png",
            ],
        )"#;

        // Parsed the way `load` does, so what this asserts is what a hand-edited
        // file actually gets.
        let parsed: Compositor = crate::parser::options()
            .from_str(sample)
            .expect("the documented shape must parse");

        assert_eq!(parsed.layout, LayoutMode::ScrollingColumns);
        // Omitted fields fall back rather than failing the whole section.
        assert_eq!(
            parsed.focus_follows_mouse,
            Compositor::default().focus_follows_mouse
        );
        assert_eq!(parsed.window_rules.len(), 2);
        assert_eq!(parsed.window_rules[0].app_id.as_deref(), Some("Nautilus"));
        assert_eq!(parsed.window_rules[0].floating, Some(true));
        assert_eq!(parsed.window_rules[0].focus, None, "omitted stays unset");
        assert_eq!(parsed.window_rules[1].focus, Some(false));
        assert_eq!(parsed.outputs[0].scale, Some(2.0));
    }

    /// The compositor never writes this file, but [`load`](crate::load) does
    /// when it is missing — so what it writes has to be readable again.
    #[test]
    fn round_trips_through_save_and_load() {
        let config = Compositor {
            window_rules: vec![WindowRule {
                app_id: Some("blender".into()),
                floating: Some(true),
                ..Default::default()
            }],
            outputs: vec![OutputSetting {
                name: "eDP-1".into(),
                scale: Some(2.0),
                ..Default::default()
            }],
            ..Default::default()
        };

        let text = ron::ser::to_string(&config).expect("serialises");
        let parsed: Compositor = ron::from_str(&text).expect("deserialises");
        assert_eq!(parsed, config);
    }

    #[test]
    fn defaults_round_trip_through_ron() {
        let default = Compositor::default();
        let text = ron::ser::to_string(&default).expect("serialises");
        let parsed: Compositor = ron::from_str(&text).expect("deserialises");
        assert_eq!(parsed, default);
    }
}
