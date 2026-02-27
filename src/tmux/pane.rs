// tmux/pane.rs - Tmux pane management
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PaneError {
    #[error("tmux command failed: {0}")]
    CommandFailed(String),
    #[error("Pane not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Information about a tmux pane
#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub id: String,
    pub session_name: String,
    pub window_id: String,
    pub title: String,
    pub current_path: String,
}

impl PaneInfo {
    /// Create a new PaneInfo
    pub fn new(id: &str, session: &str, window: &str, title: &str, path: &str) -> Self {
        Self {
            id: id.to_string(),
            session_name: session.to_string(),
            window_id: window.to_string(),
            title: title.to_string(),
            current_path: path.to_string(),
        }
    }

    /// Get the full target identifier (e.g., "session:window.pane")
    pub fn target(&self) -> String {
        format!("{}:{}.{}", self.session_name, self.window_id, self.id)
    }
}

/// List all panes in a session
pub fn list_panes(session_name: &str) -> Result<Vec<PaneInfo>, PaneError> {
    let output = Command::new("tmux")
        .args([
            "list-panes",
            "-t", session_name,
            "-F", "#{pane_id}|#{window_id}|#{pane_title}|#{pane_current_path}",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PaneError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut panes = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            panes.push(PaneInfo::new(
                parts[0],
                session_name,
                parts[1],
                parts[2],
                parts[3],
            ));
        }
    }

    Ok(panes)
}

/// Create a new pane in a window
pub fn create_pane(session: &str, window: &str, path: &Path) -> Result<String, PaneError> {
    let path_str = path.to_str().unwrap_or(".");
    
    let output = Command::new("tmux")
        .args([
            "split-window",
            "-t", &format!("{}:{}", session, window),
            "-c", path_str,
            "-P", // print pane id
            "-F", "#{pane_id}",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PaneError::CommandFailed(stderr.to_string()));
    }

    let pane_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(pane_id)
}

/// Capture pane content
pub fn capture_pane(target: &str) -> Result<String, PaneError> {
    let output = Command::new("tmux")
        .args([
            "capture-pane",
            "-t", target,
            "-p", // print to stdout
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PaneError::CommandFailed(stderr.to_string()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Send keys to a pane
pub fn send_keys(target: &str, keys: &str) -> Result<(), PaneError> {
    let output = Command::new("tmux")
        .args([
            "send-keys",
            "-t", target,
            keys,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PaneError::CommandFailed(stderr.to_string()));
    }

    Ok(())
}

/// Kill (close) a pane by target
pub fn kill_pane(target: &str) -> Result<(), PaneError> {
    let output = Command::new("tmux")
        .args([
            "kill-pane",
            "-t",
            target,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PaneError::CommandFailed(stderr.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Test: PaneInfo creation
    #[test]
    fn test_pane_info_creation() {
        let pane = PaneInfo::new("%0", "sdlc-test", "@0", "zsh", "/home/user");
        assert_eq!(pane.id, "%0");
        assert_eq!(pane.session_name, "sdlc-test");
        assert_eq!(pane.target(), "sdlc-test:@0.%0");
    }

    /// Test: PaneInfo target formatting
    #[test]
    fn test_pane_target_formatting() {
        let pane = PaneInfo::new("%1", "my-session", "@1", "bash", "/tmp");
        assert_eq!(pane.target(), "my-session:@1.%1");
    }

    /// Test: API functions exist
    #[test]
    fn test_pane_api_exists() {
        let _list_fn: fn(&str) -> Result<Vec<PaneInfo>, PaneError> = list_panes;
        let _create_fn: fn(&str, &str, &Path) -> Result<String, PaneError> = create_pane;
        let _capture_fn: fn(&str) -> Result<String, PaneError> = capture_pane;
        let _send_fn: fn(&str, &str) -> Result<(), PaneError> = send_keys;
        let _kill_fn: fn(&str) -> Result<(), PaneError> = kill_pane;
    }

    /// Test: PaneError construction
    #[test]
    fn test_pane_error() {
        let err = PaneError::NotFound("%99".to_string());
        assert!(err.to_string().contains("%99"));
    }
}
