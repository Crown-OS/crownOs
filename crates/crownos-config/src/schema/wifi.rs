crate::section! {
    pub struct Wifi in "wifi", keys WifiKey {
        pub enabled as Enabled: bool = true,
        pub network as Network: Option<String> = None,
    }
}
