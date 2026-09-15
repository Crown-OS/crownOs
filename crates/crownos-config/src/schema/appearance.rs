use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AccentColor {
    #[default]
    Purple,
    Blue,
    Green,
    Orange,
    Pink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AnimationProfile {
    /// Snap straight to the target; no springs stepped, no redraws scheduled.
    None,
    Snappy,
    #[default]
    Standard,
    Smooth,
}

crate::section! {
    pub struct Appearance in "appearance", keys AppearanceKey {
        // System
        pub dark_mode as DarkMode: bool = true,
        pub accent as Accent: AccentColor = AccentColor::Purple,
        pub transparency as Transparency: f64 = 0.0,
        pub wallpaper as Wallpaper: String = String::new(),

        // Bar
        pub bar_height as BarHeight: u32 = 32,

        // Windows. Applied by the compositor, but the same kind of setting as
        // the accent and the wallpaper: how the desktop looks rather than how it
        // behaves, and one page in the settings panel.
        /// Pixels between tiled windows.
        pub gaps_inner as GapsInner: u16 = 8,
        /// Pixels between the tiled area and the edge of the output.
        pub gaps_outer as GapsOuter: u16 = 8,
        pub border_width as BorderWidth: u16 = 2,
        pub border_radius as BorderRadius: u16 = 8,
        pub animations as Animations: AnimationProfile = AnimationProfile::Standard,
    }
}
