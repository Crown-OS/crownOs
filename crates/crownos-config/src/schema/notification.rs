use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notifications {
    pub enabled: bool,
    pub do_not_disturb: bool,
    pub show_previews: bool,
}

impl Notifications {
    pub const SECTION: &'static str = "notifications";
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            enabled: true,
            do_not_disturb: false,
            show_previews: true,
        }
    }
}
