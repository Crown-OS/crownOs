use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sound {
    pub output_volume: f64,
    pub input_volume: f64,
    pub muted: bool,
    pub output_device: Option<String>,
}

impl Sound {
    pub const SECTION: &'static str = "sound";
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            output_volume: 50.0,
            input_volume: 50.0,
            muted: false,
            output_device: None,
        }
    }
}
