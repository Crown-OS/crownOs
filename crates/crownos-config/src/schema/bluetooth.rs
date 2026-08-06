use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bluetooth {
    pub enabled: bool,
}

impl Bluetooth {
    pub const SECTION: &'static str = "bluetooth";
}

#[allow(clippy::derivable_impls, reason = "explicit for consistency")]
impl Default for Bluetooth {
    fn default() -> Self {
        Self { enabled: false }
    }
}
