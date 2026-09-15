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
        pub dark_mode as DarkMode: bool = true,
        pub accent as Accent: AccentColor = AccentColor::Purple,
        pub transparency as Transparency: f64 = 0.0,
        pub wallpaper as Wallpaper: String = String::new(),

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

        pub titlebar_height as TitlebarHeight: u16 = 36,
        pub snap as Snap: bool = true,
        pub appmenu as Appmenu: bool = true,

        pub blur as Blur: bool = true,
        pub blur_passes as BlurPasses: u16 = 3,
        pub blur_size as BlurSize: f64 = 1.5,
        pub blur_noise as BlurNoise: f64 = 0.01,
    }
}
