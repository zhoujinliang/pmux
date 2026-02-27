// app_state.rs - Application state management for GPUI
use crate::window_state::WindowState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Complete application state for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppState {
    /// Window state (size, position, maximized)
    pub window_state: WindowState,
    /// Sidebar width in pixels
    pub sidebar_width: u32,
    /// Currently active workspace index
    pub active_workspace_index: usize,
    /// List of recently opened workspaces
    pub recent_workspaces: Vec<PathBuf>,
    /// Current workspace path (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<PathBuf>,
    /// Error message (transient, not persisted)
    #[serde(skip)]
    pub error_message: Option<String>,
    /// Last saved timestamp
    #[serde(skip)]
    pub last_saved: Option<std::time::Instant>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            window_state: WindowState::default(),
            sidebar_width: 250,
            active_workspace_index: 0,
            recent_workspaces: Vec::new(),
            workspace_path: None,
            error_message: None,
            last_saved: None,
        }
    }
}

impl AppState {
    /// Create a new default app state
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific values
    pub fn with_workspace(path: PathBuf) -> Self {
        let mut state = Self::default();
        state.workspace_path = Some(path.clone());
        state.add_recent_workspace(path);
        state
    }

    /// Create with error
    pub fn with_error(message: String) -> Self {
        Self {
            error_message: Some(message),
            ..Default::default()
        }
    }

    /// Check if has a workspace loaded
    pub fn has_workspace(&self) -> bool {
        self.workspace_path.is_some()
    }

    /// Add a workspace to recent list
    pub fn add_recent_workspace(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_workspaces.retain(|p| p != &path);
        // Add to front
        self.recent_workspaces.insert(0, path);
        // Keep only last 10
        self.recent_workspaces.truncate(10);
    }

    /// Update window size
    pub fn update_window_size(&mut self, width: u32, height: u32) {
        self.window_state.size = (width, height);
    }

    /// Update window position
    pub fn update_window_position(&mut self, x: i32, y: i32) {
        self.window_state.position = (x, y);
    }

    /// Set maximized state
    pub fn set_maximized(&mut self, maximized: bool) {
        self.window_state.maximized = maximized;
    }

    /// Set sidebar width
    pub fn set_sidebar_width(&mut self, width: u32) {
        self.sidebar_width = width.clamp(200, 400);
    }

    /// Set active workspace index
    pub fn set_active_workspace(&mut self, index: usize) {
        self.active_workspace_index = index;
    }

    /// Mark as saved
    pub fn mark_saved(&mut self) {
        self.last_saved = Some(std::time::Instant::now());
    }

    /// Check if should auto-save (every 30 seconds)
    pub fn should_auto_save(&self) -> bool {
        match self.last_saved {
            None => true,
            Some(last) => last.elapsed().as_secs() > 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert_eq!(state.sidebar_width, 250);
        assert_eq!(state.active_workspace_index, 0);
        assert!(state.recent_workspaces.is_empty());
        assert!(state.workspace_path.is_none());
        assert!(state.error_message.is_none());
    }

    #[test]
    fn test_app_state_with_workspace() {
        let path = PathBuf::from("/test/path");
        let state = AppState::with_workspace(path.clone());
        assert_eq!(state.workspace_path, Some(path));
        assert_eq!(state.recent_workspaces.len(), 1);
    }

    #[test]
    fn test_add_recent_workspace() {
        let mut state = AppState::new();
        
        state.add_recent_workspace(PathBuf::from("/path1"));
        state.add_recent_workspace(PathBuf::from("/path2"));
        state.add_recent_workspace(PathBuf::from("/path3"));
        
        assert_eq!(state.recent_workspaces.len(), 3);
        assert_eq!(state.recent_workspaces[0], PathBuf::from("/path3"));
        
        // Adding duplicate moves to front
        state.add_recent_workspace(PathBuf::from("/path1"));
        assert_eq!(state.recent_workspaces.len(), 3);
        assert_eq!(state.recent_workspaces[0], PathBuf::from("/path1"));
    }

    #[test]
    fn test_recent_workspaces_limit() {
        let mut state = AppState::new();
        
        for i in 0..15 {
            state.add_recent_workspace(PathBuf::from(format!("/path{}", i)));
        }
        
        assert_eq!(state.recent_workspaces.len(), 10);
    }

    #[test]
    fn test_update_window_state() {
        let mut state = AppState::new();
        
        state.update_window_size(1920, 1080);
        assert_eq!(state.window_state.size, (1920, 1080));
        
        state.update_window_position(100, 200);
        assert_eq!(state.window_state.position, (100, 200));
        
        state.set_maximized(true);
        assert!(state.window_state.maximized);
    }

    #[test]
    fn test_set_sidebar_width_clamped() {
        let mut state = AppState::new();
        
        state.set_sidebar_width(150);
        assert_eq!(state.sidebar_width, 200);
        
        state.set_sidebar_width(500);
        assert_eq!(state.sidebar_width, 400);
        
        state.set_sidebar_width(300);
        assert_eq!(state.sidebar_width, 300);
    }

    #[test]
    fn test_should_auto_save() {
        let state = AppState::new();
        assert!(state.should_auto_save());
    }
}
