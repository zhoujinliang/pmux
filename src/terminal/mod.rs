// terminal/mod.rs - Terminal emulation bridge for alacritty_terminal
pub mod bounds;
pub mod cursor_coords;
pub mod engine;
pub mod pty_reader;
pub mod pty_writer;
pub mod renderable_snapshot;
pub mod term_bridge;

pub use bounds::TerminalBounds;
pub use cursor_coords::{LogicalCursor, TmuxCursor, VisualCursor};
pub use engine::TerminalEngine;
pub use renderable_snapshot::{RenderableSnapshot, RowData};
pub use pty_reader::{spawn_pty_reader, spawn_pty_reader_with_handle, PtyReaderHandle};
pub use pty_writer::PtyWriter;
pub use term_bridge::{StyledCell, TermBridge, RenderableContent, RenderableCursor};
