//! Terminal cursor tests: DECSCUSR shape, DECTCEM visibility.
//! Uses try_renderable_content to read cursor from RenderableContent.

use alacritty_terminal::vte::ansi::CursorShape;
use pmux::terminal::TerminalEngine;
use std::sync::Arc;

fn make_engine_with_bytes(bytes: &[u8]) -> Arc<TerminalEngine> {
    let (tx, rx) = flume::unbounded();
    let engine = Arc::new(TerminalEngine::new(80, 24, rx));
    tx.send(bytes.to_vec()).unwrap();
    drop(tx);
    engine.advance_bytes();
    engine
}

/// Read cursor shape from engine via try_renderable_content.
fn get_cursor_shape(engine: &TerminalEngine) -> Option<CursorShape> {
    engine.try_renderable_content(|content, _display_iter, _screen_lines, _cols, _display_offset| {
        content.cursor.shape
    })
}

/// Read cursor visibility from engine (shape == Hidden means DECTCEM hid it).
fn is_cursor_visible(engine: &TerminalEngine) -> bool {
    get_cursor_shape(engine).map(|s| s != CursorShape::Hidden).unwrap_or(false)
}

#[test]
fn test_cursor_shape_block_parsed_from_decscusr() {
    // DECSCUSR \x1b[1 q = Block shape (blinking)
    let engine = make_engine_with_bytes(b"\x1b[1 q");
    let shape = get_cursor_shape(&engine);
    assert!(shape.is_some(), "try_renderable_content should return Some");
    assert_eq!(
        shape.unwrap(),
        CursorShape::Block,
        "\\x1b[1 q should set cursor shape to Block"
    );
}

#[test]
fn test_cursor_shape_bar_from_decscusr() {
    // DECSCUSR \x1b[5 q = Beam/Bar shape (blinking)
    let engine = make_engine_with_bytes(b"\x1b[5 q");
    let shape = get_cursor_shape(&engine);
    assert!(shape.is_some(), "try_renderable_content should return Some");
    assert_eq!(
        shape.unwrap(),
        CursorShape::Beam,
        "\\x1b[5 q should set cursor shape to Beam (bar)"
    );
}

#[test]
fn test_cursor_hidden_when_dectcem_l() {
    // DECTCEM \x1b[?25l = hide cursor (ShowCursor mode off)
    let engine = make_engine_with_bytes(b"\x1b[?25l");
    assert!(
        !is_cursor_visible(&engine),
        "\\x1b[?25l should hide cursor (shape == Hidden)"
    );
}

#[test]
fn test_cursor_shape_underline_from_decscusr() {
    // DECSCUSR \x1b[3 q = Underline shape (blinking)
    let engine = make_engine_with_bytes(b"\x1b[3 q");
    let shape = get_cursor_shape(&engine);
    assert!(shape.is_some(), "try_renderable_content should return Some");
    assert_eq!(
        shape.unwrap(),
        CursorShape::Underline,
        "\\x1b[3 q should set cursor shape to Underline"
    );
}

#[test]
fn test_cursor_visible_when_dectcem_h() {
    // DECTCEM: first hide with 25l, then show with 25h
    let engine = make_engine_with_bytes(b"\x1b[?25l\x1b[?25h");
    assert!(
        is_cursor_visible(&engine),
        "\\x1b[?25h after \\x1b[?25l should make cursor visible again"
    );
}
