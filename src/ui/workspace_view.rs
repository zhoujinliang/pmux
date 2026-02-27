// ui/workspace_view.rs - Workspace view component
use crate::ui::AppState;
use std::path::PathBuf;

/// Workspace view component shown when a workspace is selected
pub struct WorkspaceView {
    state: AppState,
}

impl WorkspaceView {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Get the workspace path if available
    pub fn workspace_path(&self) -> Option<&PathBuf> {
        self.state.workspace_path.as_ref()
    }

    /// Get the title text
    pub fn title(&self) -> &'static str {
        "Workspace Selected"
    }

    /// Get the description text
    pub fn description(&self) -> &'static str {
        "Your Git repository has been loaded successfully"
    }

    /// Get the change workspace button label
    pub fn change_button_label(&self) -> &'static str {
        "🔄 Change Workspace"
    }

    /// Format the workspace path for display
    pub fn formatted_path(&self) -> String {
        match &self.state.workspace_path {
            Some(path) => format!("Current workspace: {}", path.display()),
            None => "No workspace selected".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_view_title() {
        let state = AppState::default();
        let view = WorkspaceView::new(state);
        assert_eq!(view.title(), "Workspace Selected");
    }

    #[test]
    fn test_workspace_view_description() {
        let state = AppState::default();
        let view = WorkspaceView::new(state);
        assert_eq!(
            view.description(),
            "Your Git repository has been loaded successfully"
        );
    }

    #[test]
    fn test_workspace_view_change_button_label() {
        let state = AppState::default();
        let view = WorkspaceView::new(state);
        assert_eq!(view.change_button_label(), "🔄 Change Workspace");
    }

    #[test]
    fn test_workspace_view_with_path() {
        let path = PathBuf::from("/test/workspace");
        let state = AppState::with_workspace(path.clone());
        let view = WorkspaceView::new(state);

        assert_eq!(view.workspace_path(), Some(&path));
        assert!(view.formatted_path().contains("/test/workspace"));
    }

    #[test]
    fn test_workspace_view_without_path() {
        let state = AppState::default();
        let view = WorkspaceView::new(state);

        assert_eq!(view.workspace_path(), None);
        assert_eq!(view.formatted_path(), "No workspace selected");
    }
}
