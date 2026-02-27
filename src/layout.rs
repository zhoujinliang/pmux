// layout.rs - Layout configuration and responsive layout calculation
use serde::{Deserialize, Serialize};

/// Layout configuration for the application
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Sidebar width in pixels
    pub sidebar_width: u32,
    /// Window size (width, height) in pixels
    pub window_size: (u32, u32),
    /// Window position (x, y) in screen coordinates
    pub window_position: (i32, i32),
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            sidebar_width: 250,
            window_size: (1200, 800),
            window_position: (100, 100),
        }
    }
}

impl LayoutConfig {
    /// Minimum sidebar width in pixels
    pub const MIN_SIDEBAR_WIDTH: u32 = 200;
    /// Maximum sidebar width in pixels
    pub const MAX_SIDEBAR_WIDTH: u32 = 400;

    /// Create a new layout config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new layout config with specific values
    pub fn with_values(
        sidebar_width: u32,
        window_width: u32,
        window_height: u32,
        pos_x: i32,
        pos_y: i32,
    ) -> Self {
        let mut config = Self {
            sidebar_width,
            window_size: (window_width, window_height),
            window_position: (pos_x, pos_y),
        };
        config.normalize();
        config
    }

    /// Normalize sidebar width to be within bounds
    pub fn normalize(&mut self) {
        self.sidebar_width = self.sidebar_width.clamp(Self::MIN_SIDEBAR_WIDTH, Self::MAX_SIDEBAR_WIDTH);
    }

    /// Calculate terminal view width based on current layout
    pub fn terminal_view_width(&self) -> u32 {
        let window_width = self.window_size.0;
        if window_width > self.sidebar_width {
            window_width - self.sidebar_width
        } else {
            0
        }
    }

    /// Calculate terminal view height (same as window height)
    pub fn terminal_view_height(&self) -> u32 {
        self.window_size.1
    }

    /// Update window size and recalculate layout
    pub fn update_window_size(&mut self, width: u32, height: u32) {
        self.window_size = (width, height);
        // Ensure sidebar doesn't exceed window width
        if self.sidebar_width >= width {
            self.sidebar_width = width.saturating_sub(50).max(Self::MIN_SIDEBAR_WIDTH);
        }
    }

    /// Update window position
    pub fn update_window_position(&mut self, x: i32, y: i32) {
        self.window_position = (x, y);
    }

    /// Set sidebar width with constraints
    pub fn set_sidebar_width(&mut self, width: u32) {
        self.sidebar_width = width.clamp(Self::MIN_SIDEBAR_WIDTH, Self::MAX_SIDEBAR_WIDTH);
    }

    /// Check if sidebar is visible (width > 0)
    pub fn is_sidebar_visible(&self) -> bool {
        self.sidebar_width > 0
    }

    /// Toggle sidebar visibility
    pub fn toggle_sidebar(&mut self) {
        if self.is_sidebar_visible() {
            self.sidebar_width = 0;
        } else {
            self.sidebar_width = 250; // Default width when showing
        }
    }
}

/// Split-screen layout management (reserved for future use)
#[derive(Clone, Debug, PartialEq, Default)]
pub enum SplitLayout {
    #[default]
    Single,
    Vertical { ratio: f32 },
    Horizontal { ratio: f32 },
    Grid { rows: u32, cols: u32 },
}

impl SplitLayout {
    /// Get the number of panes in this layout
    pub fn pane_count(&self) -> usize {
        match self {
            SplitLayout::Single => 1,
            SplitLayout::Vertical { .. } | SplitLayout::Horizontal { .. } => 2,
            SplitLayout::Grid { rows, cols } => (*rows * *cols) as usize,
        }
    }

    /// Calculate pane dimensions for given container size
    pub fn calculate_pane_sizes(&self, container_width: u32, container_height: u32) -> Vec<(u32, u32)> {
        match self {
            SplitLayout::Single => vec![(container_width, container_height)],
            SplitLayout::Vertical { ratio } => {
                let left_width = (container_width as f32 * ratio) as u32;
                let right_width = container_width - left_width;
                vec![(left_width, container_height), (right_width, container_height)]
            }
            SplitLayout::Horizontal { ratio } => {
                let top_height = (container_height as f32 * ratio) as u32;
                let bottom_height = container_height - top_height;
                vec![(container_width, top_height), (container_width, bottom_height)]
            }
            SplitLayout::Grid { rows, cols } => {
                let pane_width = container_width / cols;
                let pane_height = container_height / rows;
                vec![(pane_width, pane_height); (*rows * *cols) as usize]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_config_default() {
        let config = LayoutConfig::default();
        assert_eq!(config.sidebar_width, 250);
        assert_eq!(config.window_size, (1200, 800));
        assert_eq!(config.window_position, (100, 100));
    }

    #[test]
    fn test_layout_config_new() {
        let config = LayoutConfig::new();
        assert_eq!(config.sidebar_width, 250);
    }

    #[test]
    fn test_sidebar_width_constraints() {
        let mut config = LayoutConfig::default();
        
        // Test minimum constraint
        config.set_sidebar_width(100);
        assert_eq!(config.sidebar_width, LayoutConfig::MIN_SIDEBAR_WIDTH);
        
        // Test maximum constraint
        config.set_sidebar_width(500);
        assert_eq!(config.sidebar_width, LayoutConfig::MAX_SIDEBAR_WIDTH);
        
        // Test valid range
        config.set_sidebar_width(300);
        assert_eq!(config.sidebar_width, 300);
    }

    #[test]
    fn test_normalize_enforces_bounds() {
        let mut config = LayoutConfig::with_values(150, 1200, 800, 100, 100);
        assert_eq!(config.sidebar_width, LayoutConfig::MIN_SIDEBAR_WIDTH);
        
        let mut config = LayoutConfig::with_values(500, 1200, 800, 100, 100);
        assert_eq!(config.sidebar_width, LayoutConfig::MAX_SIDEBAR_WIDTH);
    }

    #[test]
    fn test_terminal_view_width() {
        let config = LayoutConfig::with_values(250, 1200, 800, 100, 100);
        assert_eq!(config.terminal_view_width(), 950);
    }

    #[test]
    fn test_terminal_view_width_with_small_window() {
        let config = LayoutConfig::with_values(300, 200, 800, 100, 100);
        assert_eq!(config.terminal_view_width(), 0);
    }

    #[test]
    fn test_update_window_size_adjusts_sidebar() {
        let mut config = LayoutConfig::with_values(300, 1200, 800, 100, 100);
        config.update_window_size(250, 600);
        // Sidebar should be adjusted to not exceed window width
        assert!(config.sidebar_width <= config.window_size.0);
    }

    #[test]
    fn test_toggle_sidebar() {
        let mut config = LayoutConfig::default();
        assert!(config.is_sidebar_visible());
        
        config.toggle_sidebar();
        assert!(!config.is_sidebar_visible());
        assert_eq!(config.sidebar_width, 0);
        
        config.toggle_sidebar();
        assert!(config.is_sidebar_visible());
        assert_eq!(config.sidebar_width, 250);
    }

    #[test]
    fn test_split_layout_single() {
        let layout = SplitLayout::Single;
        assert_eq!(layout.pane_count(), 1);
        let sizes = layout.calculate_pane_sizes(800, 600);
        assert_eq!(sizes, vec![(800, 600)]);
    }

    #[test]
    fn test_split_layout_vertical() {
        let layout = SplitLayout::Vertical { ratio: 0.5 };
        assert_eq!(layout.pane_count(), 2);
        let sizes = layout.calculate_pane_sizes(800, 600);
        assert_eq!(sizes[0].1, 600);
        assert_eq!(sizes[1].1, 600);
        assert_eq!(sizes[0].0 + sizes[1].0, 800);
    }
}
