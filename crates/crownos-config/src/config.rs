use std::path::PathBuf;

/// Environment variable key that overrides the config directory.
pub const CONFIG_DIR_ENV: &str = "CROWN_CONFIG_DIR";

pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(CONFIG_DIR_ENV)
        && !dir.is_empty()
    {
        return PathBuf::from(dir);
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("crownos")
}
