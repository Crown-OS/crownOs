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

        // Background blur behind windows that ask for it (via the
        // ext-background-effect-v1 protocol). Applied by the compositor.
        pub blur as Blur: bool = true,
        /// Downsample depth of the blur pyramid; each pass roughly doubles the
        /// perceived radius. Clamped by the compositor to 1..=8.
        pub blur_passes as BlurPasses: u16 = 3,
        /// Kawase tap spread. Fractional values are fine — the taps land
        /// between texels on purpose.
        pub blur_size as BlurSize: f64 = 1.5,
        /// Dither strength that hides gradient banding in the blur (0 to 1).
        pub blur_noise as BlurNoise: f64 = 0.01,
    }
}
