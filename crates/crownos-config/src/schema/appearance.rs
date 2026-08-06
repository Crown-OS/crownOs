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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Appearance {
    // System
    pub dark_mode: bool,
    pub accent: AccentColor,
    pub transparency: f64,
    pub wallpaper: String,

    // Bar
    pub bar_height: u32,
}

impl Appearance {
    pub const SECTION: &'static str = "appearance";
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            dark_mode: true,
            accent: AccentColor::Purple,
            transparency: 0.0,
            wallpaper: String::new(),
            bar_height: 32,
        }
    }
}
