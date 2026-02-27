// config.rs - Configuration management for pmux
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub recent_workspace: Option<String>,
}

impl Config {
    /// Load configuration from a specific path
    /// Returns default config if file doesn't exist
    pub fn load_from_path(path: &PathBuf) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Config::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a specific path
    pub fn save_to_path(&self, path: &PathBuf) -> Result<(), ConfigError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the recent workspace as a PathBuf
    pub fn get_recent_workspace(&self) -> Option<PathBuf> {
        self.recent_workspace.as_ref().map(PathBuf::from)
    }

    /// Save a new workspace path
    pub fn save_workspace(&mut self, path: &str) {
        self.recent_workspace = Some(path.to_string());
    }

    /// Get the default config path (~/.config/pmux/config.json)
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pmux")
            .join("config.json")
    }

    /// Load from default path
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from_path(&Self::default_path())
    }

    /// Save to default path
    pub fn save(&self) -> Result<(), ConfigError> {
        self.save_to_path(&Self::default_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Test: Config should be readable when file exists
    #[test]
    fn test_config_read_existing_file() {
        // Arrange: Create a temp directory with config file
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Write test config
        let test_config = r#"{"recent_workspace": "/path/to/repo"}"#;
        std::fs::write(&config_path, test_config).unwrap();

        // Act: Try to read config
        let config = Config::load_from_path(&config_path);

        // Assert: Should successfully load with correct path
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.recent_workspace, Some("/path/to/repo".to_string()));
    }

    /// Test: Config should return default when file doesn't exist
    #[test]
    fn test_config_load_nonexistent_file() {
        // Arrange: Use a path that doesn't exist
        let nonexistent_path = PathBuf::from("/tmp/nonexistent/config.json");

        // Act: Try to load from non-existent path
        let config = Config::load_from_path(&nonexistent_path);

        // Assert: Should return default config (None for recent_workspace)
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.recent_workspace, None);
    }

    /// Test: Config should save correctly
    #[test]
    fn test_config_save() {
        // Arrange: Create temp directory
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let config = Config {
            recent_workspace: Some("/home/user/project".to_string()),
        };

        // Act: Save config
        let result = config.save_to_path(&config_path);

        // Assert: Should save successfully
        assert!(result.is_ok());

        // Verify: Read back and check
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("/home/user/project"));
    }

    /// Test: Config should handle invalid JSON gracefully
    #[test]
    fn test_config_load_invalid_json() {
        // Arrange: Create temp file with invalid JSON
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        std::fs::write(&config_path, "not valid json").unwrap();

        // Act: Try to load invalid config
        let result = Config::load_from_path(&config_path);

        // Assert: Should return error
        assert!(result.is_err());
    }

    /// Test: get_recent_workspace should return the saved path
    #[test]
    fn test_get_recent_workspace_returns_saved_path() {
        // Arrange
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let config = Config {
            recent_workspace: Some("/workspace/myrepo".to_string()),
        };
        config.save_to_path(&config_path).unwrap();

        // Act
        let loaded = Config::load_from_path(&config_path).unwrap();
        let workspace = loaded.get_recent_workspace();

        // Assert
        assert_eq!(workspace, Some(PathBuf::from("/workspace/myrepo")));
    }

    /// Test: save_workspace should update and persist the path
    #[test]
    fn test_save_workspace_updates_config() {
        // Arrange
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut config = Config {
            recent_workspace: None,
        };

        // Act
        config.save_workspace("/new/workspace/path");
        config.save_to_path(&config_path).unwrap();

        // Assert: Load and verify
        let loaded = Config::load_from_path(&config_path).unwrap();
        assert_eq!(
            loaded.recent_workspace,
            Some("/new/workspace/path".to_string())
        );
    }
}
