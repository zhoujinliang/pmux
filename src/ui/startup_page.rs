// ui/startup_page.rs - Startup page component
use crate::ui::AppState;

/// Startup page component
pub struct StartupPage {
    state: AppState,
}

impl StartupPage {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Get the title text
    pub fn title(&self) -> &'static str {
        "Welcome to pmux"
    }

    /// Get the description text
    pub fn description(&self) -> &'static str {
        "Select a Git repository to manage your AI agents"
    }

    /// Get the button label
    pub fn button_label(&self) -> &'static str {
        "📁 Select Workspace"
    }

    /// Check if there's an error to display
    pub fn has_error(&self) -> bool {
        self.state.error_message.is_some()
    }

    /// Get error message if any
    pub fn error_message(&self) -> Option<&String> {
        self.state.error_message.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startup_page_title() {
        let state = AppState::default();
        let page = StartupPage::new(state);
        assert_eq!(page.title(), "Welcome to pmux");
    }

    #[test]
    fn test_startup_page_description() {
        let state = AppState::default();
        let page = StartupPage::new(state);
        assert_eq!(
            page.description(),
            "Select a Git repository to manage your AI agents"
        );
    }

    #[test]
    fn test_startup_page_button_label() {
        let state = AppState::default();
        let page = StartupPage::new(state);
        assert_eq!(page.button_label(), "📁 Select Workspace");
    }

    #[test]
    fn test_startup_page_no_error_by_default() {
        let state = AppState::default();
        let page = StartupPage::new(state);
        assert!(!page.has_error());
        assert!(page.error_message().is_none());
    }

    #[test]
    fn test_startup_page_shows_error() {
        let state = AppState::with_error("Test error".to_string());
        let page = StartupPage::new(state);
        assert!(page.has_error());
        assert_eq!(page.error_message(), Some(&"Test error".to_string()));
    }
}
