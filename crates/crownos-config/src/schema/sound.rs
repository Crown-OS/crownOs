crate::section! {
    pub struct Sound in "sound", keys SoundKey {
        pub output_volume as OutputVolume: f64 = 50.0,
        pub input_volume as InputVolume: f64 = 50.0,
        pub muted as Muted: bool = false,
        pub output_device as OutputDevice: Option<String> = None,
    }
}
