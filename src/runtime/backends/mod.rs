//! Backend adapters implementing AgentRuntime.
//!
//! Supports both Local PTY (direct shell spawn) and Tmux (session persistence).

mod local_pty;
#[cfg(unix)]
mod tmux;

pub use local_pty::LocalPtyRuntime;
#[cfg(unix)]
pub use tmux::TmuxRuntime;

use std::path::Path;
use std::sync::Arc;

use crate::config::Config;
use crate::runtime::agent_runtime::{AgentRuntime, RuntimeError};
use crate::runtime::WorktreeState;

/// Environment variable to select backend. Valid values: "local", "tmux".
pub const PMUX_BACKEND_ENV: &str = "PMUX_BACKEND";

/// Default backend when environment variable is not set.
pub const DEFAULT_BACKEND: &str = "local";

/// Resolve backend: PMUX_BACKEND env > config.backend > "local".
/// Invalid values (non-local/tmux) fall back to "local".
pub fn resolve_backend(config: Option<&Config>) -> String {
    const VALID: [&str; 2] = ["local", "tmux"];
    let from_env = std::env::var(PMUX_BACKEND_ENV).ok();
    let from_config = config.map(|c| c.backend.as_str());
    let raw = from_env.as_deref().or(from_config).unwrap_or(DEFAULT_BACKEND);
    if VALID.contains(&raw) {
        raw.to_string()
    } else {
        DEFAULT_BACKEND.to_string()
    }
}

/// Session naming for tmux backend. One workspace (repo) = one session.
/// Example: /foo/repo -> "pmux-repo"
pub fn session_name_for_workspace(workspace_path: &Path) -> String {
    format!(
        "pmux-{}",
        workspace_path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_else(|| "default".into())
    )
}

/// Window naming for tmux backend. One worktree/agent = one window.
/// Main worktree -> "main"; linked worktrees -> sanitized branch name.
pub fn window_name_for_worktree(_worktree_path: &Path, branch_name: &str) -> String {
    let name = if branch_name.is_empty() || branch_name == "main" {
        "main".to_string()
    } else {
        branch_name.to_string()
    };
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
}

/// Target for killing a worktree's window: session:window
pub fn window_target(workspace_path: &Path, window_name: &str) -> String {
    format!("{}:{}", session_name_for_workspace(workspace_path), window_name)
}

/// Create a runtime for the given worktree.
/// Backend resolution: PMUX_BACKEND env > config.backend > "local".
///
/// Tmux: one workspace = one session, one worktree = one window.
///
/// # Examples
/// ```bash
/// PMUX_BACKEND=tmux pmux
/// ```
pub fn create_runtime_from_env(
    workspace_path: &Path,
    worktree_path: &Path,
    branch_name: &str,
    cols: u16,
    rows: u16,
    config: Option<&Config>,
) -> Result<Arc<dyn AgentRuntime>, RuntimeError> {
    let backend = resolve_backend(config);

    match backend.as_str() {
        "tmux" => {
            #[cfg(unix)]
            {
                let session_name = session_name_for_workspace(workspace_path);
                let window_name = window_name_for_worktree(worktree_path, branch_name);
                Ok(create_tmux_runtime(session_name, window_name))
            }
            #[cfg(not(unix))]
            Err(RuntimeError::Backend(
                "tmux backend not supported on non-Unix platforms".into(),
            ))
        }
        "local" | _ => create_runtime(worktree_path, cols, rows),
    }
}

/// Create a LocalPtyRuntime for the given worktree path.
/// Returns an AgentRuntime that supports multiple panes.
pub fn create_runtime(
    worktree_path: &Path,
    cols: u16,
    rows: u16,
) -> Result<Arc<dyn AgentRuntime>, RuntimeError> {
    let rt = LocalPtyRuntime::new(worktree_path, cols, rows)?;
    Ok(Arc::new(rt))
}

/// Create a TmuxRuntime for the given session and window.
/// Session persistence allows agents to continue running after pmux closes.
#[cfg(unix)]
pub fn create_tmux_runtime(
    session_name: impl Into<String>,
    window_name: impl Into<String>,
) -> Arc<dyn AgentRuntime> {
    let rt = TmuxRuntime::new(session_name, window_name);
    Arc::new(rt)
}

/// Non-Unix fallback: create_local_runtime
#[cfg(not(unix))]
pub fn create_tmux_runtime(
    _session_name: impl Into<String>,
    _window_name: impl Into<String>,
) -> Arc<dyn AgentRuntime> {
    panic!("tmux backend not supported on non-Unix platforms")
}

/// Recover an AgentRuntime from persisted state.
/// Used when pmux restarts and needs to attach to existing sessions.
#[cfg(unix)]
pub fn recover_runtime(
    backend: &str,
    state: &WorktreeState,
    _event_bus: Option<Arc<crate::runtime::EventBus>>,
) -> Result<Arc<dyn AgentRuntime>, RuntimeError> {
    match backend {
        "local" | "local_pty" => Err(RuntimeError::Backend(
            "local_pty does not support session recovery".into(),
        )),
        "tmux" => {
            // Attach to existing tmux session/window
            let runtime = TmuxRuntime::attach(
                &state.backend_session_id,
                &state.backend_window_id,
            )?;
            Ok(Arc::new(runtime))
        }
        _ => Err(RuntimeError::Backend(format!(
            "unknown backend: {}",
            backend
        ))),
    }
}

/// Non-Unix fallback: tmux not supported
#[cfg(not(unix))]
pub fn recover_runtime(
    backend: &str,
    _state: &WorktreeState,
    _event_bus: Option<Arc<crate::runtime::EventBus>>,
) -> Result<Arc<dyn AgentRuntime>, RuntimeError> {
    match backend {
        "local" | "local_pty" => Err(RuntimeError::Backend(
            "local_pty does not support session recovery".into(),
        )),
        "tmux" => Err(RuntimeError::Backend(
            "tmux backend not supported on non-Unix platforms".into(),
        )),
        _ => Err(RuntimeError::Backend(format!(
            "unknown backend: {}",
            backend
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::runtime::WorktreeState;
    use std::path::PathBuf;

    #[test]
    fn test_resolve_backend_env_overrides_config() {
        std::env::set_var(PMUX_BACKEND_ENV, "tmux");
        let config = Config {
            backend: "local".into(),
            ..Config::default()
        };
        assert_eq!(resolve_backend(Some(&config)), "tmux");
        std::env::remove_var(PMUX_BACKEND_ENV);
    }

    #[test]
    fn test_resolve_backend_config_overrides_default() {
        std::env::remove_var(PMUX_BACKEND_ENV);
        let config = Config {
            backend: "tmux".into(),
            ..Config::default()
        };
        assert_eq!(resolve_backend(Some(&config)), "tmux");
    }

    #[test]
    fn test_resolve_backend_invalid_fallback() {
        std::env::remove_var(PMUX_BACKEND_ENV);
        let config = Config {
            backend: "docker".into(),
            ..Config::default()
        };
        assert_eq!(resolve_backend(Some(&config)), "local");
    }

    #[test]
    fn test_recover_runtime_unknown_backend() {
        let state = WorktreeState {
            path: PathBuf::from("/tmp/test"),
            branch: "main".to_string(),
            agent_id: "test".to_string(),
            pane_ids: vec![],
            backend: "unknown".to_string(),
            backend_session_id: String::new(),
            backend_window_id: String::new(),
            split_tree_json: None,
        };
        let result = recover_runtime("unknown_backend", &state, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_recover_runtime_local_pty_not_supported() {
        let state = WorktreeState {
            path: PathBuf::from("/tmp/test"),
            branch: "main".to_string(),
            agent_id: "test".to_string(),
            pane_ids: vec![],
            backend: "local".to_string(),
            backend_session_id: String::new(),
            backend_window_id: String::new(),
            split_tree_json: None,
        };
        let result = recover_runtime("local", &state, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not support"));
    }

    #[test]
    fn test_create_tmux_runtime_unix() {
        let rt = create_tmux_runtime("pmux-test-session", "test-window");
        // Just verify it creates without panicking
        // The actual tmux operations require tmux binary
        let _ = rt.primary_pane_id();
    }
}
