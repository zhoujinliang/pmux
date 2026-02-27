// tmux/mod.rs - Tmux integration module
pub mod session;
pub mod pane;
pub mod window;

pub use session::{Session, SessionError};
pub use pane::{PaneInfo, list_panes, create_pane, capture_pane, send_keys, PaneError};
pub use window::{WindowInfo, list_windows, create_window, rename_window, WindowError};

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Module exports are available
    #[test]
    fn test_module_exports() {
        // Verify all public items are accessible
        let _: fn(&str) -> Session = Session::new;
        let _: fn(&str) -> bool = Session::exists;
    }
}
