use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PowerProfile {
    PowerSaver,
    #[default]
    Balanced,
    Performance,
}

crate::section! {
    pub struct Power in "power", keys PowerKey {
        pub screen_off_minutes as ScreenOffMinutes: u32 = 10,
        pub sleep_minutes as SleepMinutes: u32 = 30,
        pub power_profile as Profile: PowerProfile = PowerProfile::Balanced,
    }
}
