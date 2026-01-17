//! Configuration for the Synheart Sensor Agent.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration for the sensor agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Duration of each collection window
    #[serde(with = "duration_serde")]
    pub window_duration: Duration,

    /// Which input sources to capture
    pub sources: SourceConfig,

    /// Path for exporting HSI snapshots
    pub export_path: PathBuf,

    /// Path for storing state and transparency logs
    pub data_path: PathBuf,

    /// Whether collection is currently paused
    pub paused: bool,

    /// Gap threshold for session boundaries (in seconds)
    pub session_gap_threshold_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("synheart-sensor-agent");

        Self {
            window_duration: Duration::from_secs(10),
            sources: SourceConfig::default(),
            export_path: data_dir.join("exports"),
            data_path: data_dir,
            paused: false,
            session_gap_threshold_secs: 300, // 5 minutes
        }
    }
}

impl Config {
    /// Load configuration from the default location.
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| ConfigError::IoError(e.to_string()))?;
            let config: Config = serde_json::from_str(&content)
                .map_err(|e| ConfigError::ParseError(e.to_string()))?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to the default location.
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::config_path();

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::IoError(e.to_string()))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializeError(e.to_string()))?;

        std::fs::write(&config_path, content).map_err(|e| ConfigError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Get the path to the configuration file.
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("synheart-sensor-agent")
            .join("config.json")
    }

    /// Ensure all required directories exist.
    pub fn ensure_directories(&self) -> Result<(), ConfigError> {
        std::fs::create_dir_all(&self.export_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
        std::fs::create_dir_all(&self.data_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
        Ok(())
    }
}

/// Configuration for which input sources to capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    pub keyboard: bool,
    pub mouse: bool,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            keyboard: true,
            mouse: true,
        }
    }
}

impl SourceConfig {
    /// Parse source configuration from a comma-separated string.
    pub fn from_csv(s: &str) -> Self {
        let sources: Vec<String> = s.split(',').map(|s| s.trim().to_lowercase()).collect();

        Self {
            keyboard: sources.iter().any(|s| s == "keyboard" || s == "all"),
            mouse: sources.iter().any(|s| s == "mouse" || s == "all"),
        }
    }

    /// Check if at least one source is enabled.
    pub fn any_enabled(&self) -> bool {
        self.keyboard || self.mouse
    }
}

/// Configuration errors.
#[derive(Debug)]
pub enum ConfigError {
    IoError(String),
    ParseError(String),
    SerializeError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {e}"),
            ConfigError::ParseError(e) => write!(f, "Parse error: {e}"),
            ConfigError::SerializeError(e) => write!(f, "Serialize error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Serde support for Duration.
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_config_parsing() {
        let config = SourceConfig::from_csv("keyboard,mouse");
        assert!(config.keyboard);
        assert!(config.mouse);

        let config = SourceConfig::from_csv("keyboard");
        assert!(config.keyboard);
        assert!(!config.mouse);

        let config = SourceConfig::from_csv("all");
        assert!(config.keyboard);
        assert!(config.mouse);
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.window_duration, Duration::from_secs(10));
        assert!(config.sources.keyboard);
        assert!(config.sources.mouse);
        assert!(!config.paused);
    }
}
