// config_test.rs - TDD tests for configuration management
// RED phase: Write failing test first

use std::path::PathBuf;
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
    assert_eq!(loaded.recent_workspace, Some("/new/workspace/path".to_string()));
}

// Placeholder struct - this will fail to compile until we implement it
struct Config {
    recent_workspace: Option<String>,
}

impl Config {
    fn load_from_path(_path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: Implement this
        unimplemented!("Config::load_from_path not yet implemented")
    }
    
    fn save_to_path(&self, _path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement this
        unimplemented!("Config::save_to_path not yet implemented")
    }
    
    fn get_recent_workspace(&self) -> Option<PathBuf> {
        // TODO: Implement this
        unimplemented!("Config::get_recent_workspace not yet implemented")
    }
    
    fn save_workspace(&mut self, _path: &str) {
        // TODO: Implement this
        unimplemented!("Config::save_workspace not yet implemented")
    }
}
