// empty_state.rs - Empty state components for pmux
use serde::{Deserialize, Serialize};

/// Visual style for empty states
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EmptyStateStyle {
    Minimal,
    Illustrated,
    Card,
}

impl Default for EmptyStateStyle {
    fn default() -> Self {
        EmptyStateStyle::Illustrated
    }
}

/// Empty state configuration
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmptyStateConfig {
    pub style: EmptyStateStyle,
    pub show_icon: bool,
    pub show_title: bool,
    pub show_description: bool,
    pub show_cta: bool,
}

impl Default for EmptyStateConfig {
    fn default() -> Self {
        Self {
            style: EmptyStateStyle::default(),
            show_icon: true,
            show_title: true,
            show_description: true,
            show_cta: true,
        }
    }
}

/// Empty state types
#[derive(Clone, Debug, PartialEq)]
pub enum EmptyStateType {
    NoWorkspaceSelected,
    NoNotifications,
    EmptyWorktreeList,
    NoResults,
    Loading,
    Error,
}

/// Empty state data
#[derive(Clone, Debug)]
pub struct EmptyState {
    pub state_type: EmptyStateType,
    pub icon: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub cta_text: Option<&'static str>,
    pub config: EmptyStateConfig,
}

impl EmptyState {
    /// Create "No workspace selected" empty state
    pub fn no_workspace_selected() -> Self {
        Self {
            state_type: EmptyStateType::NoWorkspaceSelected,
            icon: "📂",
            title: "No Workspace Selected",
            description: "Select a git repository to get started with pmux.",
            cta_text: Some("Open Workspace"),
            config: EmptyStateConfig::default(),
        }
    }

    /// Create "No notifications" empty state
    pub fn no_notifications() -> Self {
        Self {
            state_type: EmptyStateType::NoNotifications,
            icon: "🔕",
            title: "No Notifications",
            description: "You're all caught up! Desktop notifications will appear here when agents need attention.",
            cta_text: None,
            config: EmptyStateConfig {
                show_cta: false,
                ..Default::default()
            },
        }
    }

    /// Create "Empty worktree list" empty state
    pub fn empty_worktree_list() -> Self {
        Self {
            state_type: EmptyStateType::EmptyWorktreeList,
            icon: "🌳",
            title: "No Worktrees Yet",
            description: "Create your first branch to start working with AI agents in parallel.",
            cta_text: Some("New Branch (⌘⇧N)"),
            config: EmptyStateConfig::default(),
        }
    }

    /// Create "No results" empty state (for search/filter)
    pub fn no_results() -> Self {
        Self {
            state_type: EmptyStateType::NoResults,
            icon: "🔍",
            title: "No Results Found",
            description: "Try adjusting your search or filters.",
            cta_text: Some("Clear Filters"),
            config: EmptyStateConfig::default(),
        }
    }

    /// Create loading empty state
    pub fn loading() -> Self {
        Self {
            state_type: EmptyStateType::Loading,
            icon: "⏳",
            title: "Loading...",
            description: "Please wait while we set things up.",
            cta_text: None,
            config: EmptyStateConfig {
                show_cta: false,
                ..Default::default()
            },
        }
    }

    /// Create error empty state
    pub fn error(message: &'static str) -> Self {
        Self {
            state_type: EmptyStateType::Error,
            icon: "⚠️",
            title: "Something Went Wrong",
            description: message,
            cta_text: Some("Retry"),
            config: EmptyStateConfig::default(),
        }
    }

    /// Get CSS class for the empty state style
    pub fn style_class(&self) -> &'static str {
        match self.config.style {
            EmptyStateStyle::Minimal => "empty-state-minimal",
            EmptyStateStyle::Illustrated => "empty-state-illustrated",
            EmptyStateStyle::Card => "empty-state-card",
        }
    }

    /// Check if CTA should be shown
    pub fn has_cta(&self) -> bool {
        self.config.show_cta && self.cta_text.is_some()
    }
}

/// Empty state renderer (for GPUI integration)
pub struct EmptyStateRenderer;

impl EmptyStateRenderer {
    /// Render empty state as text representation
    pub fn render(state: &EmptyState) -> String {
        let mut output = String::new();

        if state.config.show_icon {
            output.push_str(&format!("{}\n\n", state.icon));
        }

        if state.config.show_title {
            output.push_str(&format!("{}\n", state.title));
        }

        if state.config.show_description {
            output.push_str(&format!("{}\n", state.description));
        }

        if state.has_cta() {
            output.push_str(&format!("\n[ {} ]", state.cta_text.unwrap()));
        }

        output
    }

    /// Get compact representation for sidebar/tooltip
    pub fn render_compact(state: &EmptyState) -> String {
        format!("{} {}", state.icon, state.title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_workspace_selected() {
        let state = EmptyState::no_workspace_selected();
        assert_eq!(state.state_type, EmptyStateType::NoWorkspaceSelected);
        assert_eq!(state.icon, "📂");
        assert_eq!(state.title, "No Workspace Selected");
        assert_eq!(state.cta_text, Some("Open Workspace"));
        assert!(state.has_cta());
    }

    #[test]
    fn test_no_notifications() {
        let state = EmptyState::no_notifications();
        assert_eq!(state.state_type, EmptyStateType::NoNotifications);
        assert_eq!(state.icon, "🔕");
        assert_eq!(state.title, "No Notifications");
        assert!(!state.has_cta());
    }

    #[test]
    fn test_empty_worktree_list() {
        let state = EmptyState::empty_worktree_list();
        assert_eq!(state.state_type, EmptyStateType::EmptyWorktreeList);
        assert_eq!(state.icon, "🌳");
        assert_eq!(state.title, "No Worktrees Yet");
        assert_eq!(state.cta_text, Some("New Branch (⌘⇧N)"));
        assert!(state.has_cta());
    }

    #[test]
    fn test_no_results() {
        let state = EmptyState::no_results();
        assert_eq!(state.state_type, EmptyStateType::NoResults);
        assert_eq!(state.icon, "🔍");
        assert_eq!(state.title, "No Results Found");
        assert!(state.has_cta());
    }

    #[test]
    fn test_loading() {
        let state = EmptyState::loading();
        assert_eq!(state.state_type, EmptyStateType::Loading);
        assert_eq!(state.icon, "⏳");
        assert_eq!(state.title, "Loading...");
        assert!(!state.has_cta());
    }

    #[test]
    fn test_error() {
        let state = EmptyState::error("Connection failed");
        assert_eq!(state.state_type, EmptyStateType::Error);
        assert_eq!(state.icon, "⚠️");
        assert_eq!(state.title, "Something Went Wrong");
        assert_eq!(state.description, "Connection failed");
        assert!(state.has_cta());
    }

    #[test]
    fn test_render() {
        let state = EmptyState::no_workspace_selected();
        let rendered = EmptyStateRenderer::render(&state);
        assert!(rendered.contains("📂"));
        assert!(rendered.contains("No Workspace Selected"));
        assert!(rendered.contains("Open Workspace"));
    }

    #[test]
    fn test_render_compact() {
        let state = EmptyState::no_notifications();
        let rendered = EmptyStateRenderer::render_compact(&state);
        assert_eq!(rendered, "🔕 No Notifications");
    }

    #[test]
    fn test_style_class() {
        let state = EmptyState::no_workspace_selected();
        assert_eq!(state.style_class(), "empty-state-illustrated");
    }

    #[test]
    fn test_config_default() {
        let config = EmptyStateConfig::default();
        assert!(config.show_icon);
        assert!(config.show_title);
        assert!(config.show_description);
        assert!(config.show_cta);
    }
}
