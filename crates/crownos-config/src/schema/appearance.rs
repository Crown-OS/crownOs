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

crate::section! {
    pub struct Appearance in "appearance", keys AppearanceKey {
        // System
        pub dark_mode as DarkMode: bool = true,
        pub accent as Accent: AccentColor = AccentColor::Purple,
        pub transparency as Transparency: f64 = 0.0,
        pub wallpaper as Wallpaper: String = String::new(),

        // Bar
        pub bar_height as BarHeight: u32 = 32,
    }
}
