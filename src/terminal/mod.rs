// terminal/mod.rs - Terminal emulation bridge for alacritty_terminal
pub mod engine;
pub mod pty_reader;
pub mod pty_writer;
pub mod term_bridge;

pub use engine::TerminalEngine;
pub use pty_reader::{spawn_pty_reader, spawn_pty_reader_with_handle, PtyReaderHandle};
pub use pty_writer::PtyWriter;
pub use term_bridge::{StyledCell, TermBridge, RenderableContent, RenderableCursor};
