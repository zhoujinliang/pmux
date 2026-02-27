// resizable_sidebar.rs - Resizable sidebar with drag detection and keyboard shortcuts
use crate::layout::LayoutConfig;
use serde::{Deserialize, Serialize};

/// Sidebar resize state for tracking drag operations
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum ResizeState {
    #[default]
    Idle,
    Hovering,           // Mouse is on the resize border
    Dragging {          // Currently dragging
        start_x: i32,
        start_width: u32,
    },
}

/// Resizable sidebar controller
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResizableSidebar {
    pub layout: LayoutConfig,
    pub resize_state: ResizeState,
    /// Width before collapse (for restore on double-click)
    pub width_before_collapse: Option<u32>,
    /// Whether sidebar is currently collapsed
    pub is_collapsed: bool,
    /// Resize handle width in pixels
    pub resize_handle_width: u32,
}

impl Default for ResizableSidebar {
    fn default() -> Self {
        Self {
            layout: LayoutConfig::default(),
            resize_state: ResizeState::Idle,
            width_before_collapse: None,
            is_collapsed: false,
            resize_handle_width: 4,
        }
    }
}

impl ResizableSidebar {
    /// Create a new resizable sidebar with default layout
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific layout config
    pub fn with_layout(layout: LayoutConfig) -> Self {
        Self {
            layout,
            ..Default::default()
        }
    }

    /// Check if mouse position is on the resize border
    /// Returns true if x is within resize_handle_width pixels of the right edge of sidebar
    pub fn is_on_resize_border(&self, mouse_x: i32) -> bool {
        if self.is_collapsed {
            return false;
        }
        let sidebar_right = self.layout.sidebar_width as i32;
        let tolerance = self.resize_handle_width as i32;
        mouse_x >= sidebar_right - tolerance && mouse_x <= sidebar_right + tolerance
    }

    /// Start dragging operation
    pub fn start_drag(&mut self, mouse_x: i32) {
        self.resize_state = ResizeState::Dragging {
            start_x: mouse_x,
            start_width: self.layout.sidebar_width,
        };
    }

    /// Update sidebar width during drag
    pub fn update_drag(&mut self, mouse_x: i32) {
        if let ResizeState::Dragging { start_x, start_width } = self.resize_state {
            let delta = mouse_x - start_x;
            let new_width = if delta >= 0 {
                start_width.saturating_add(delta as u32)
            } else {
                start_width.saturating_sub((-delta) as u32)
            };
            self.layout.set_sidebar_width(new_width);
            self.is_collapsed = false;
        }
    }

    /// End dragging operation
    pub fn end_drag(&mut self) {
        self.resize_state = ResizeState::Idle;
    }

    /// Set hover state when mouse is on resize border
    pub fn set_hovering(&mut self, hovering: bool) {
        match (&self.resize_state, hovering) {
            (ResizeState::Idle, true) => {
                self.resize_state = ResizeState::Hovering;
            }
            (ResizeState::Hovering, false) => {
                self.resize_state = ResizeState::Idle;
            }
            _ => {}
        }
    }

    /// Toggle sidebar visibility (⌘B shortcut)
    pub fn toggle_sidebar(&mut self) {
        if self.is_collapsed {
            // Restore to previous width or default
            let restored_width = self.width_before_collapse.unwrap_or(250);
            self.layout.sidebar_width = restored_width.clamp(
                LayoutConfig::MIN_SIDEBAR_WIDTH,
                LayoutConfig::MAX_SIDEBAR_WIDTH,
            );
            self.is_collapsed = false;
        } else {
            // Save current width and collapse
            self.width_before_collapse = Some(self.layout.sidebar_width);
            self.layout.sidebar_width = 0;
            self.is_collapsed = true;
        }
    }

    /// Double-click handler - auto expand/collapse
    pub fn on_double_click(&mut self) {
        self.toggle_sidebar();
    }

    /// Get current sidebar width (0 if collapsed)
    pub fn sidebar_width(&self) -> u32 {
        if self.is_collapsed {
            0
        } else {
            self.layout.sidebar_width
        }
    }

    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool {
        matches!(self.resize_state, ResizeState::Dragging { .. })
    }

    /// Check if hovering over resize border
    pub fn is_hovering(&self) -> bool {
        matches!(self.resize_state, ResizeState::Hovering)
    }

    /// Get the cursor style based on current state
    pub fn cursor_style(&self) -> &'static str {
        if self.is_dragging() || self.is_hovering() {
            "col-resize"
        } else {
            "default"
        }
    }

    /// Update window size
    pub fn update_window_size(&mut self, width: u32, height: u32) {
        self.layout.update_window_size(width, height);
    }

    /// Get terminal view width
    pub fn terminal_view_width(&self) -> u32 {
        self.layout.terminal_view_width()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resizable_sidebar_default() {
        let sidebar = ResizableSidebar::new();
        assert!(!sidebar.is_collapsed);
        assert_eq!(sidebar.sidebar_width(), 250);
        assert!(!sidebar.is_dragging());
        assert!(!sidebar.is_hovering());
    }

    #[test]
    fn test_is_on_resize_border() {
        let sidebar = ResizableSidebar::with_layout(LayoutConfig::with_values(250, 1200, 800, 0, 0));
        
        // Mouse at exactly sidebar width (right edge)
        assert!(sidebar.is_on_resize_border(250));
        
        // Mouse within tolerance (4px)
        assert!(sidebar.is_on_resize_border(248));
        assert!(sidebar.is_on_resize_border(252));
        
        // Mouse outside tolerance
        assert!(!sidebar.is_on_resize_border(240));
        assert!(!sidebar.is_on_resize_border(260));
    }

    #[test]
    fn test_is_on_resize_border_when_collapsed() {
        let mut sidebar = ResizableSidebar::new();
        sidebar.toggle_sidebar(); // Collapse
        
        // Should always return false when collapsed
        assert!(!sidebar.is_on_resize_border(0));
        assert!(!sidebar.is_on_resize_border(10));
    }

    #[test]
    fn test_drag_operations() {
        let mut sidebar = ResizableSidebar::with_layout(LayoutConfig::with_values(250, 1200, 800, 0, 0));
        
        // Start drag
        sidebar.start_drag(250);
        assert!(sidebar.is_dragging());
        
        // Update drag (move 50px right)
        sidebar.update_drag(300);
        assert_eq!(sidebar.sidebar_width(), 300);
        
        // End drag
        sidebar.end_drag();
        assert!(!sidebar.is_dragging());
    }

    #[test]
    fn test_drag_left_decreases_width() {
        let mut sidebar = ResizableSidebar::with_layout(LayoutConfig::with_values(300, 1200, 800, 0, 0));
        
        sidebar.start_drag(300);
        sidebar.update_drag(250); // Move 50px left
        
        assert_eq!(sidebar.sidebar_width(), 250);
    }

    #[test]
    fn test_drag_respects_min_max_constraints() {
        let mut sidebar = ResizableSidebar::with_layout(LayoutConfig::with_values(250, 1200, 800, 0, 0));
        
        sidebar.start_drag(250);
        
        // Try to drag way past max
        sidebar.update_drag(1000);
        assert_eq!(sidebar.sidebar_width(), LayoutConfig::MAX_SIDEBAR_WIDTH);
        
        // Reset and try to drag below min
        sidebar.layout.set_sidebar_width(250);
        sidebar.start_drag(250);
        sidebar.update_drag(0);
        assert_eq!(sidebar.sidebar_width(), LayoutConfig::MIN_SIDEBAR_WIDTH);
    }

    #[test]
    fn test_toggle_sidebar() {
        let mut sidebar = ResizableSidebar::new();
        
        // Initially visible
        assert!(!sidebar.is_collapsed);
        assert_eq!(sidebar.sidebar_width(), 250);
        
        // Toggle to collapse
        sidebar.toggle_sidebar();
        assert!(sidebar.is_collapsed);
        assert_eq!(sidebar.sidebar_width(), 0);
        assert_eq!(sidebar.width_before_collapse, Some(250));
        
        // Toggle to restore
        sidebar.toggle_sidebar();
        assert!(!sidebar.is_collapsed);
        assert_eq!(sidebar.sidebar_width(), 250);
    }

    #[test]
    fn test_toggle_sidebar_restores_different_widths() {
        let mut sidebar = ResizableSidebar::with_layout(LayoutConfig::with_values(350, 1200, 800, 0, 0));
        
        sidebar.toggle_sidebar(); // Collapse
        sidebar.toggle_sidebar(); // Restore
        
        assert_eq!(sidebar.sidebar_width(), 350);
    }

    #[test]
    fn test_double_click_toggles() {
        let mut sidebar = ResizableSidebar::new();
        
        sidebar.on_double_click();
        assert!(sidebar.is_collapsed);
        
        sidebar.on_double_click();
        assert!(!sidebar.is_collapsed);
    }

    #[test]
    fn test_hover_state() {
        let mut sidebar = ResizableSidebar::new();
        
        // Initially idle
        assert!(!sidebar.is_hovering());
        
        // Set hovering
        sidebar.set_hovering(true);
        assert!(sidebar.is_hovering());
        
        // Clear hovering
        sidebar.set_hovering(false);
        assert!(!sidebar.is_hovering());
    }

    #[test]
    fn test_cursor_style() {
        let mut sidebar = ResizableSidebar::new();
        
        // Default cursor
        assert_eq!(sidebar.cursor_style(), "default");
        
        // Hovering shows resize cursor
        sidebar.set_hovering(true);
        assert_eq!(sidebar.cursor_style(), "col-resize");
        
        // Back to default
        sidebar.set_hovering(false);
        assert_eq!(sidebar.cursor_style(), "default");
        
        // Dragging also shows resize cursor
        sidebar.start_drag(250);
        assert_eq!(sidebar.cursor_style(), "col-resize");
    }

    #[test]
    fn test_terminal_view_width() {
        let sidebar = ResizableSidebar::with_layout(LayoutConfig::with_values(250, 1200, 800, 0, 0));
        assert_eq!(sidebar.terminal_view_width(), 950);
    }

    #[test]
    fn test_terminal_view_width_when_collapsed() {
        let mut sidebar = ResizableSidebar::with_layout(LayoutConfig::with_values(250, 1200, 800, 0, 0));
        sidebar.toggle_sidebar();
        assert_eq!(sidebar.terminal_view_width(), 1200);
    }
}
