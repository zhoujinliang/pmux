//! Cursor coordinate types for Task 2.5.
//!
//! Three coordinate systems (tmux backend must distinguish):
//! - **LogicalCursor**: engine internal, includes scrollback (alacritty `renderable_content().cursor`)
//! - **VisualCursor**: viewport-adjusted, for paint (LogicalCursor + display_offset → pixel coords)
//! - **TmuxCursor**: pane-relative, for tmux commands (capture-pane, send-keys click)
//!
//! Local PTY: Logical = Tmux. Tmux backend: when display_offset ≠ tmux scroll, conversion needed.

/// Logical cursor from engine (grid line can be negative for scrollback).
#[derive(Debug, Clone, Copy)]
pub struct LogicalCursor {
    pub line: i32,
    pub col: usize,
}

/// Visual cursor (viewport-relative), used for paint.
#[derive(Debug, Clone, Copy)]
pub struct VisualCursor {
    pub line: i32,
    pub col: usize,
}

/// Tmux pane-relative cursor for tmux commands.
#[derive(Debug, Clone, Copy)]
pub struct TmuxCursor {
    pub row: usize,
    pub col: usize,
}

impl LogicalCursor {
    /// Convert to viewport-relative visual cursor.
    /// visible_line = grid_line - display_offset
    pub fn to_visual(self, display_offset: usize) -> VisualCursor {
        VisualCursor {
            line: self.line - display_offset as i32,
            col: self.col,
        }
    }

    /// Convert to Tmux cursor. When display_offset matches tmux scroll, row = logical line.
    /// TODO: When tmux scroll ≠ display_offset, sync or map. See Task 2.5.
    pub fn to_tmux(self, display_offset: usize) -> TmuxCursor {
        let row = (self.line + display_offset as i32).max(0) as usize;
        TmuxCursor {
            row,
            col: self.col,
        }
    }
}
