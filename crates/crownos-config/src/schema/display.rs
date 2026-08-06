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

crate::section! {
    pub struct Display in "display", keys DisplayKey {
        pub brightness as Brightness: f64 = 80.0,
        pub night_light as NightLight: bool = false,
        pub night_light_warmth as NightLightWarmth: f64 = 50.0,
        pub scale as Scale: DisplayScale = DisplayScale::S100,
    }
}
