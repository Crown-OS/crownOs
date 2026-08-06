use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PowerProfile {
    PowerSaver,
    #[default]
    Balanced,
    Performance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Power {
    pub screen_off_minutes: u32,
    pub sleep_minutes: u32,
    pub power_profile: PowerProfile,
}

impl Power {
    pub const SECTION: &'static str = "power";
}

impl Default for Power {
    fn default() -> Self {
        Self {
            screen_off_minutes: 10,
            sleep_minutes: 30,
            power_profile: PowerProfile::Balanced,
        }
    }
}
