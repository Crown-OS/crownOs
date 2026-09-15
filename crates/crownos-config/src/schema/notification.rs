crate::section! {
    pub struct Notifications in "notifications", keys NotificationsKey {
        pub enabled as Enabled: bool = true,
        pub do_not_disturb as DoNotDisturb: bool = false,
        pub show_previews as ShowPreviews: bool = true,
    }
}
