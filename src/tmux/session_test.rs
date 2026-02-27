// session_test.rs - TDD tests for tmux session management
#[cfg(test)]
mod tests {
    use super::super::*;

    /// Test: Session name should be formatted correctly
    #[test]
    fn test_session_name_formatting() {
        let session = Session::new("myproject");
        assert_eq!(session.name(), "sdlc-myproject");
    }

    /// Test: Session exists check (mock)
    #[test]
    fn test_session_exists_mock() {
        // This would need actual tmux to test properly
        // For now, just verify the API exists
        let _ = Session::exists as fn(&str) -> bool;
    }

    /// Test: Session creation returns correct struct
    #[test]
    fn test_session_creation() {
        let session = Session::new("test-repo");
        assert_eq!(session.name(), "sdlc-test-repo");
        assert_eq!(session.window_name(), "control-tower");
    }

    /// Test: Invalid repo names are sanitized
    #[test]
    fn test_session_name_sanitization() {
        let session = Session::new("my/repo with spaces");
        let name = session.name();
        assert!(!name.contains('/'));
        assert!(!name.contains(' '));
    }
}
