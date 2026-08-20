use serde::{Deserialize, Serialize};

/// User settings, persisted as JSON in the OS config directory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// `id` of the currently selected pack. Must match a directory under assets/packs/.
    pub pack_id: String,
    /// Master output gain, clamped to 0.0..=1.0.
    pub volume: f32,
    /// When true, key events are tracked but no audio is played.
    pub muted: bool,
    /// When true, a LaunchAgent plist exists and is loaded.
    pub start_on_login: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pack_id: "cherry-mx-blue".to_string(),
            volume: 0.6,
            muted: false,
            start_on_login: false,
        }
    }
}

/// Errors that can occur while loading or saving the config file.
#[derive(Debug)]
pub enum ConfigError {
    /// The OS config directory could not be determined.
    NoConfigDir,
    /// Filesystem read/write failure.
    Io(std::io::Error),
    /// The file existed but was not valid JSON matching Config.
    Parse(serde_json::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NoConfigDir => write!(f, "could not determine OS config directory"),
            ConfigError::Io(e) => write!(f, "config file I/O error: {e}"),
            ConfigError::Parse(e) => write!(f, "config file parse error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Absolute path to the config file: ~/Library/Application Support/keyclack/config.json
pub fn config_path() -> Result<std::path::PathBuf, ConfigError> {
    let dirs = directories::ProjectDirs::from("", "", "keyclack").ok_or(ConfigError::NoConfigDir)?;
    Ok(dirs.config_dir().join("config.json"))
}

/// Load config from disk. Returns Config::default() if the file does not exist.
pub fn load() -> Result<Config, ConfigError> {
    let path = config_path()?;
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(e) => return Err(ConfigError::Io(e)),
    };
    serde_json::from_str(&contents).map_err(ConfigError::Parse)
}

/// Write config to disk as pretty-printed JSON, creating parent directories as needed.
pub fn save(config: &Config) -> Result<(), ConfigError> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(ConfigError::Io)?;
    }
    let json = serde_json::to_string_pretty(config).map_err(ConfigError::Parse)?;
    std::fs::write(&path, json).map_err(ConfigError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let c = Config::default();
        assert_eq!(c.pack_id, "cherry-mx-blue");
        assert_eq!(c.volume, 0.6);
        assert_eq!(c.muted, false);
        assert_eq!(c.start_on_login, false);
    }

    #[test]
    fn json_roundtrip() {
        let c = Config::default();
        let json = serde_json::to_string(&c).expect("serialize");
        let back: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c, back);
    }

    #[test]
    fn parse_rejects_garbage() {
        let result = serde_json::from_str::<Config>("{ not json }");
        assert!(result.is_err());
    }

    #[test]
    fn parse_rejects_missing_field() {
        let result = serde_json::from_str::<Config>(r#"{"pack_id":"x","volume":0.5}"#);
        assert!(result.is_err());
    }
}
