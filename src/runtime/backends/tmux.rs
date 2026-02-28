//! Tmux backend - uses tmux commands for session persistence.
//!
//! Implements AgentRuntime via tmux send-keys, pipe-pane, split-window, etc.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::runtime::agent_runtime::{AgentId, AgentRuntime, PaneId, RuntimeError};

/// Tmux runtime - delegates to tmux commands for session persistence.
pub struct TmuxRuntime {
    session_name: String,
    window_name: String,
}

impl TmuxRuntime {
    /// Create a new TmuxRuntime for the given session and window.
    pub fn new(session_name: impl Into<String>, window_name: impl Into<String>) -> Self {
        Self {
            session_name: session_name.into(),
            window_name: window_name.into(),
        }
    }

    /// Attach to an existing tmux session/window. Verifies the session exists before returning.
    /// Returns Err if the session or window does not exist (e.g. tmux was killed).
    pub fn attach(
        session_id: &str,
        window_id: &str,
    ) -> Result<Self, crate::runtime::agent_runtime::RuntimeError> {
        let rt = Self::new(session_id, window_id);
        // Verify session and window exist
        let target = rt.window_target();
        let output = std::process::Command::new("tmux")
            .args(["list-windows", "-t", &target])
            .output()
            .map_err(|e| crate::runtime::agent_runtime::RuntimeError::Backend(format!("tmux exec failed: {}", e)))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::runtime::agent_runtime::RuntimeError::Backend(format!(
                "tmux session/window not found: {}",
                stderr.trim()
            )));
        }
        Ok(rt)
    }

    /// Build full pane target: session:window.pane_id
    fn pane_target(&self, pane_id: &PaneId) -> String {
        format!("{}:{}.{}", self.session_name, self.window_name, pane_id)
    }

    /// Build window target: session:window
    fn window_target(&self) -> String {
        format!("{}:{}", self.session_name, self.window_name)
    }

    /// Run tmux command, return error on non-zero exit.
    fn tmux_cmd(&self, args: &[&str]) -> Result<(), RuntimeError> {
        let output = Command::new("tmux")
            .args(args)
            .output()
            .map_err(|e| RuntimeError::Backend(format!("tmux exec failed: {}", e)))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RuntimeError::Backend(format!(
                "tmux failed: {}",
                stderr.trim()
            )));
        }
        Ok(())
    }

    /// Run tmux command and capture stdout.
    fn tmux_cmd_output(&self, args: &[&str]) -> Result<String, RuntimeError> {
        let output = Command::new("tmux")
            .args(args)
            .output()
            .map_err(|e| RuntimeError::Backend(format!("tmux exec failed: {}", e)))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RuntimeError::Backend(format!(
                "tmux failed: {}",
                stderr.trim()
            )));
        }
        String::from_utf8(output.stdout)
            .map_err(|e| RuntimeError::Backend(format!("tmux output invalid utf8: {}", e)))
    }

    /// Get current branch for worktree (for review window name).
    fn get_branch(&self, worktree: &Path) -> Result<String, RuntimeError> {
        let output = Command::new("git")
            .args(["-C", worktree.to_str().unwrap_or("."), "rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .map_err(|e| RuntimeError::Backend(format!("git exec failed: {}", e)))?;
        if !output.status.success() {
            return Err(RuntimeError::Backend("git rev-parse failed".to_string()));
        }
        let branch = String::from_utf8(output.stdout)
            .map_err(|e| RuntimeError::Backend(format!("git output invalid utf8: {}", e)))?;
        Ok(branch.trim().to_string())
    }
}

impl AgentRuntime for TmuxRuntime {
    fn send_input(&self, pane_id: &PaneId, bytes: &[u8]) -> Result<(), RuntimeError> {
        let target = self.pane_target(pane_id);
        // tmux send-keys -t {target} -l {literal} for raw bytes
        // We need to escape/encode bytes for tmux. For UTF-8 text, pass as -l literal.
        let s = String::from_utf8_lossy(bytes);
        let escaped = escape_for_tmux_send_keys(&s);
        self.tmux_cmd(&["send-keys", "-t", &target, "-l", &escaped])
    }

    fn send_key(&self, pane_id: &PaneId, key: &str, use_literal: bool) -> Result<(), RuntimeError> {
        let target = self.pane_target(pane_id);
        if use_literal {
            let escaped = escape_for_tmux_send_keys(key);
            self.tmux_cmd(&["send-keys", "-t", &target, "-l", &escaped])
        } else {
            self.tmux_cmd(&["send-keys", "-t", &target, key])
        }
    }

    fn resize(&self, pane_id: &PaneId, cols: u16, rows: u16) -> Result<(), RuntimeError> {
        let target = self.pane_target(pane_id);
        self.tmux_cmd(&[
            "resize-pane",
            "-t",
            &target,
            "-x",
            &cols.to_string(),
            "-y",
            &rows.to_string(),
        ])
    }

    fn subscribe_output(&self, pane_id: &PaneId) -> Option<flume::Receiver<Vec<u8>>> {
        #[cfg(unix)]
        {
            let (tx, rx) = flume::unbounded();
            let fifo_path = std::env::temp_dir().join(format!("pmux-pipe-{}", uuid::Uuid::new_v4()));
            if nix::unistd::mkfifo(&fifo_path, nix::sys::stat::Mode::S_IRWXU).is_err() {
                return None;
            }
            let fifo_path = fifo_path.to_path_buf();
            let stop = std::sync::Arc::new(AtomicBool::new(false));

            // Reader thread: open fifo (blocks until writer opens), read and send to channel
            let fifo_read = fifo_path.clone();
            let tx_clone = tx.clone();
            let stop_clone = stop.clone();
            thread::spawn(move || {
                let file = match std::fs::File::open(&fifo_read) {
                    Ok(f) => f,
                    Err(_) => return,
                };
                let mut reader = std::io::BufReader::new(file);
                let mut buf = [0u8; 4096];
                while !stop_clone.load(Ordering::SeqCst) {
                    match std::io::Read::read(&mut reader, &mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if tx_clone.send(buf[..n].to_vec()).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let _ = std::fs::remove_file(&fifo_read);
            });

            // Spawn pipe-pane: tmux pipe-pane -o -t {target} 'cat >> {fifo}'
            let fifo_str = fifo_path.to_string_lossy().to_string();
            let session = self.session_name.clone();
            let window = self.window_name.clone();
            let pane_id = pane_id.to_string();
            thread::spawn(move || {
                let target = format!("{}:{}.{}", session, window, pane_id);
                let cmd = format!("cat >> {}", fifo_str);
                let _ = Command::new("tmux")
                    .args(["pipe-pane", "-o", "-t", &target, &cmd])
                    .status();
                stop.store(true, Ordering::SeqCst);
            });

            Some(rx)
        }
        #[cfg(not(unix))]
        {
            let _ = pane_id;
            None
        }
    }

    fn capture_initial_content(&self, pane_id: &PaneId) -> Option<Vec<u8>> {
        let target = self.pane_target(pane_id);
        let out = self
            .tmux_cmd_output(&["capture-pane", "-t", &target, "-p", "-e"])
            .ok()?;
        Some(out.into_bytes())
    }

    fn list_panes(&self, agent_id: &AgentId) -> Vec<PaneId> {
        let target = if agent_id.is_empty() {
            self.window_target()
        } else {
            agent_id.clone()
        };
        let out = match self.tmux_cmd_output(&["list-panes", "-t", &target, "-F", "#{pane_id}"]) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let panes: Vec<PaneId> = out
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        panes
    }

    fn focus_pane(&self, pane_id: &PaneId) -> Result<(), RuntimeError> {
        let target = self.pane_target(pane_id);
        self.tmux_cmd(&["select-pane", "-t", &target])
    }

    fn split_pane(&self, pane_id: &PaneId, vertical: bool) -> Result<PaneId, RuntimeError> {
        let target = self.pane_target(pane_id);
        let flag = if vertical { "-v" } else { "-h" };
        let out = self.tmux_cmd_output(&["split-window", flag, "-t", &target, "-P", "-F", "#{pane_id}"])?;
        let new_id = out.trim().to_string();
        if new_id.is_empty() {
            return Err(RuntimeError::Backend("split-window did not return pane id".to_string()));
        }
        Ok(new_id)
    }

    fn get_pane_dimensions(&self, pane_id: &PaneId) -> (u16, u16) {
        let target = self.pane_target(pane_id);
        let out = match self.tmux_cmd_output(&["display-message", "-t", &target, "-p", "-F", "#{pane_width} #{pane_height}"]) {
            Ok(s) => s,
            Err(_) => return (80, 24),
        };
        let parts: Vec<&str> = out.split_whitespace().collect();
        if parts.len() >= 2 {
            if let (Ok(c), Ok(r)) = (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
                return (c, r);
            }
        }
        (80, 24)
    }

    fn open_diff(&self, worktree: &Path, _pane_id: Option<&PaneId>) -> Result<String, RuntimeError> {
        let branch = self.get_branch(worktree)?;
        let safe_branch = branch.replace('/', "-").replace(' ', "-");
        let window_name = format!("review-{}", safe_branch);
        let worktree_str = worktree.to_string_lossy();
        self.tmux_cmd(&[
            "new-window",
            "-t",
            &self.session_name,
            "-n",
            &window_name,
            "-c",
            worktree_str.as_ref(),
            "nvim",
            "-c",
            "DiffviewOpen main...HEAD",
        ])?;
        Ok(window_name)
    }

    fn open_review(&self, worktree: &Path) -> Result<String, RuntimeError> {
        self.open_diff(worktree, None)
    }

    fn kill_window(&self, window_target: &str) -> Result<(), RuntimeError> {
        self.tmux_cmd(&["kill-window", "-t", window_target])
    }
}

/// Escape string for tmux send-keys -l (literal mode).
fn escape_for_tmux_send_keys(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmux_available() -> bool {
        Command::new("tmux").arg("-V").output().is_ok_and(|o| o.status.success())
    }

    #[test]
    fn test_tmux_runtime_new() {
        let rt = TmuxRuntime::new("sdlc-test", "main");
        assert_eq!(rt.pane_target(&"%0".to_string()), "sdlc-test:main.%0");
        assert_eq!(rt.window_target(), "sdlc-test:main");
    }

    #[test]
    fn test_tmux_runtime_pane_target() {
        let rt = TmuxRuntime::new("mysession", "mywindow");
        assert_eq!(rt.pane_target(&"%1".to_string()), "mysession:mywindow.%1");
    }

    #[test]
    fn test_escape_for_tmux_send_keys() {
        assert_eq!(escape_for_tmux_send_keys("hello"), "hello");
        assert_eq!(escape_for_tmux_send_keys("it's"), "it\\'s");
        assert_eq!(escape_for_tmux_send_keys("path\\to"), "path\\\\to");
    }

    #[test]
    fn test_tmux_runtime_list_panes_requires_tmux() {
        if !tmux_available() {
            return;
        }
        let rt = TmuxRuntime::new("nonexistent-session-xyz", "nonexistent-window");
        let panes = rt.list_panes(&String::new());
        assert!(panes.is_empty());
    }

    #[test]
    fn test_tmux_runtime_send_key_requires_tmux() {
        if !tmux_available() {
            return;
        }
        let rt = TmuxRuntime::new("nonexistent-session-xyz", "nonexistent-window");
        let err = rt.send_key(&"%0".to_string(), "Enter", false);
        assert!(err.is_err());
    }

    #[test]
    fn test_tmux_runtime_kill_window_invalid_target() {
        if !tmux_available() {
            return;
        }
        let rt = TmuxRuntime::new("sdlc-test", "main");
        let err = rt.kill_window("nonexistent:window");
        assert!(err.is_err());
    }

    #[test]
    fn test_tmux_runtime_get_pane_dimensions_invalid() {
        let rt = TmuxRuntime::new("nonexistent-session", "nonexistent-window");
        let (c, r) = rt.get_pane_dimensions(&"%0".to_string());
        assert_eq!(c, 80);
        assert_eq!(r, 24);
    }

    #[test]
    fn test_tmux_runtime_open_diff_no_git() {
        let rt = TmuxRuntime::new("sdlc-test", "main");
        let err = rt.open_diff(PathBuf::from("/nonexistent/path").as_path(), None);
        assert!(err.is_err());
    }

    #[test]
    fn test_tmux_runtime_attach_nonexistent_session() {
        let result = TmuxRuntime::attach("pmux-nonexistent-session-xyz", "nonexistent-window");
        assert!(result.is_err());
    }

    #[test]
    fn test_tmux_runtime_attach_requires_tmux() {
        if !tmux_available() {
            return;
        }
        let result = TmuxRuntime::attach("pmux-nonexistent-session-xyz", "main");
        assert!(result.is_err());
    }
}
