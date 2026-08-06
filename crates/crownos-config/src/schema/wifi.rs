use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wifi {
    pub enabled: bool,
    pub network: Option<String>,
}

impl Wifi {
    pub const SECTION: &'static str = "wifi";
}

impl Default for Wifi {
    fn default() -> Self {
        Self {
            enabled: true,
            network: None,
        }
    }
}
