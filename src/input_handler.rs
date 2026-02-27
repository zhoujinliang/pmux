/// Input handler for forwarding keyboard events to tmux sessions
pub struct InputHandler {
    /// The tmux session name (e.g., "sdlc-myproject")
    session_name: String,
}

impl InputHandler {
    /// Create a new InputHandler for the given session
    pub fn new(session_name: String) -> Self {
        Self { session_name }
    }

    /// Get the session name
    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    /// Send a key to the active pane in the tmux session
    /// Returns Ok(()) if the key was sent, or an error message if it failed
    /// Errors are logged but do not crash the application
    pub fn send_key(&self, key: &str) -> Result<(), String> {
        use std::process::Command;

        // Build the target: send to the active pane in the session
        // Format: tmux send-keys -t <session_name> <key>
        let output = Command::new("tmux")
            .args(["send-keys", "-t", &self.session_name, key])
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    eprintln!("InputHandler: tmux send-keys failed: {}", stderr);
                    Err(stderr.to_string())
                }
            }
            Err(e) => {
                eprintln!("InputHandler: Failed to execute tmux send-keys: {}", e);
                Err(format!("Failed to execute tmux send-keys: {}", e))
            }
        }
    }
}

/// Convert a GPUI key name to a tmux send-keys string.
/// Returns None if the key should be handled by pmux (app shortcut).
pub fn key_to_tmux(key: &str, modifiers_cmd: bool) -> Option<String> {
    // pmux shortcuts: Cmd+B, Cmd+N, Cmd+W — intercept
    if modifiers_cmd {
        return None;
    }
    let tmux_key = match key {
        "enter" | "return" => "Enter",
        "backspace" => "BSpace",
        "escape" => "Escape",
        "tab" => "Tab",
        "up" => "Up",
        "down" => "Down",
        "left" => "Left",
        "right" => "Right",
        "home" => "Home",
        "end" => "End",
        "pageup" => "PPage",
        "pagedown" => "NPage",
        other => return Some(other.to_string()),
    };
    Some(tmux_key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_handler_creation() {
        let handler = InputHandler::new("sdlc-myproject".to_string());
        assert_eq!(handler.session_name(), "sdlc-myproject");
    }

    #[test]
    fn test_send_key_api_exists() {
        // Verify the send_key method has the correct signature
        let handler = InputHandler::new("test-session".to_string());
        let _fn_ptr: fn(&InputHandler, &str) -> Result<(), String> = InputHandler::send_key;
        // We can't test actual tmux execution without tmux running,
        // but we verify the API exists by checking the method signature
        let _ = handler.send_key("test-key");
    }

    #[test]
    fn test_enter_key() {
        assert_eq!(key_to_tmux("enter", false), Some("Enter".to_string()));
    }

    #[test]
    fn test_backspace_key() {
        assert_eq!(key_to_tmux("backspace", false), Some("BSpace".to_string()));
    }

    #[test]
    fn test_arrow_keys() {
        assert_eq!(key_to_tmux("up", false), Some("Up".to_string()));
        assert_eq!(key_to_tmux("down", false), Some("Down".to_string()));
        assert_eq!(key_to_tmux("left", false), Some("Left".to_string()));
        assert_eq!(key_to_tmux("right", false), Some("Right".to_string()));
    }

    #[test]
    fn test_escape_tab() {
        assert_eq!(key_to_tmux("escape", false), Some("Escape".to_string()));
        assert_eq!(key_to_tmux("tab", false), Some("Tab".to_string()));
    }

    #[test]
    fn test_cmd_key_intercepted() {
        assert_eq!(key_to_tmux("b", true), None);
        assert_eq!(key_to_tmux("n", true), None);
    }

    #[test]
    fn test_regular_char_passthrough() {
        assert_eq!(key_to_tmux("a", false), Some("a".to_string()));
        assert_eq!(key_to_tmux("z", false), Some("z".to_string()));
    }
}
