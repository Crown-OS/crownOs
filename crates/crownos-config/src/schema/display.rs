use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DisplayScale {
    #[default]
    S100,
    S125,
    S150,
    S200,
}

impl DisplayScale {
    pub const fn factor(self) -> f64 {
        match self {
            Self::S100 => 1.0,
            Self::S125 => 1.25,
            Self::S150 => 1.5,
            Self::S200 => 2.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Display {
    pub brightness: f64,
    pub night_light: bool,
    pub night_light_warmth: f64,
    pub scale: DisplayScale,
}

impl Display {
    pub const SECTION: &'static str = "display";
}

impl Default for Display {
    fn default() -> Self {
        Self {
            brightness: 80.0,
            night_light: false,
            night_light_warmth: 50.0,
            scale: DisplayScale::S100,
        }
    }
}
