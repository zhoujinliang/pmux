// app.rs - Main application logic for pmux
use crate::config::Config;
use crate::file_selector::show_folder_picker;
use crate::git_utils::{get_git_error_message, is_git_repository, GitError};
use std::path::PathBuf;

/// Main application struct
pub struct App {
    config: Config,
    workspace_path: Option<PathBuf>,
    error_message: Option<String>,
}

impl App {
    /// Create a new application instance
    pub fn new() -> Self {
        // Try to load existing config
        let config = Config::load().unwrap_or_default();
        let workspace_path = config.get_recent_workspace();

        Self {
            config,
            workspace_path,
            error_message: None,
        }
    }

    /// Check if we have a workspace to open
    pub fn has_workspace(&self) -> bool {
        self.workspace_path.is_some()
    }

    /// Get the current workspace path
    pub fn get_workspace(&self) -> Option<&PathBuf> {
        self.workspace_path.as_ref()
    }

    /// Get any error message to display
    pub fn get_error(&self) -> Option<&String> {
        self.error_message.as_ref()
    }

    /// Clear the current error
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Handle "Select Workspace" button click
    /// Returns true if a valid workspace was selected
    pub fn select_workspace(&mut self) -> bool {
        // Show file picker
        let Some(selected_path) = show_folder_picker() else {
            // User cancelled
            return false;
        };

        // Validate it's a git repository
        if !is_git_repository(&selected_path) {
            let error = GitError::NotARepository;
            self.error_message = Some(get_git_error_message(&selected_path, &error));
            return false;
        }

        // Save the workspace
        self.workspace_path = Some(selected_path.clone());
        self.config.save_workspace(selected_path.to_str().unwrap());

        // Persist config
        if let Err(e) = self.config.save() {
            eprintln!("Warning: Failed to save config: {}", e);
        }

        self.error_message = None;
        true
    }

    /// Reset to startup page (for testing or "change workspace" feature)
    pub fn reset_workspace(&mut self) {
        self.workspace_path = None;
        self.config.save_workspace("");
        let _ = self.config.save();
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Test: App initializes without crashing
    #[test]
    fn test_app_initializes() {
        let app = App::new();
        // Should not panic
        let _ = app.has_workspace();
    }

    /// Test: has_workspace returns correct state
    #[test]
    fn test_has_workspace_reflects_state() {
        let mut app = App::new();

        // After reset, should not have workspace
        app.reset_workspace();
        assert!(!app.has_workspace());
    }

    /// Test: Error handling flow
    #[test]
    fn test_error_handling() {
        let mut app = App::new();

        // Initially no error
        assert!(app.get_error().is_none());

        // Simulate setting an error
        app.error_message = Some("Test error".to_string());
        assert!(app.get_error().is_some());

        // Clear error
        app.clear_error();
        assert!(app.get_error().is_none());
    }

    /// Test: validate_git_repository through app context
    #[test]
    fn test_app_validates_git_repo() {
        // Create temp git repo
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();

        // Verify validation works at git_utils level
        assert!(is_git_repository(temp_dir.path()));
    }
}
