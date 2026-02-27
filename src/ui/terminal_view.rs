// ui/terminal_view.rs - Terminal view component with GPUI render
use gpui::prelude::*;
use gpui::*;
use std::sync::{Arc, Mutex};

/// Terminal content representation
#[derive(Clone)]
pub struct TerminalContent {
    pub lines: Vec<TerminalLine>,
    pub cursor_position: Option<(usize, usize)>,
}

impl TerminalContent {
    pub fn new() -> Self {
        Self { lines: Vec::new(), cursor_position: None }
    }

    pub fn from_string(content: &str) -> Self {
        Self {
            lines: content.lines().map(TerminalLine::new).collect(),
            cursor_position: None,
        }
    }

    pub fn line_count(&self) -> usize { self.lines.len() }

    pub fn to_string(&self) -> String {
        self.lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n")
    }

    pub fn update(&mut self, content: &str) {
        self.lines = content.lines().map(TerminalLine::new).collect();
    }
}

impl Default for TerminalContent {
    fn default() -> Self { Self::new() }
}

/// Single line in terminal
#[derive(Debug, Clone)]
pub struct TerminalLine {
    pub text: String,
    pub styles: Vec<StyleRange>,
}

impl TerminalLine {
    pub fn new(text: &str) -> Self {
        Self { text: text.to_string(), styles: Vec::new() }
    }
    pub fn len(&self) -> usize { self.text.len() }
    pub fn is_empty(&self) -> bool { self.text.is_empty() }
}

/// Style range for a portion of text
#[derive(Debug, Clone)]
pub struct StyleRange {
    pub start: usize,
    pub end: usize,
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
    pub bold: bool,
    pub italic: bool,
}

/// Color representation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self { Self { r, g, b } }

    pub fn to_hsla(&self) -> Hsla {
        let hex = ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32);
        rgb(hex).into()
    }

    pub fn black() -> Self { Self::new(0, 0, 0) }
    pub fn white() -> Self { Self::new(255, 255, 255) }
    pub fn red() -> Self { Self::new(255, 0, 0) }
    pub fn green() -> Self { Self::new(0, 255, 0) }
    pub fn blue() -> Self { Self::new(0, 0, 255) }
    pub fn gray() -> Self { Self::new(128, 128, 128) }
    pub fn dark_gray() -> Self { Self::new(64, 64, 64) }
}

/// Terminal view component - renders tmux pane content
pub struct TerminalView {
    pane_id: String,
    title: String,
    content: Arc<Mutex<TerminalContent>>,
    scroll_offset: usize,
}

impl TerminalView {
    pub fn new(pane_id: &str, title: &str) -> Self {
        Self {
            pane_id: pane_id.to_string(),
            title: title.to_string(),
            content: Arc::new(Mutex::new(TerminalContent::new())),
            scroll_offset: 0,
        }
    }

    /// Create with a shared content buffer (for live tmux polling)
    pub fn with_content(pane_id: &str, title: &str, content: Arc<Mutex<TerminalContent>>) -> Self {
        Self {
            pane_id: pane_id.to_string(),
            title: title.to_string(),
            content,
            scroll_offset: 0,
        }
    }

    pub fn update_content(&mut self, content: &str) {
        if let Ok(mut guard) = self.content.lock() { guard.update(content); }
    }

    pub fn pane_id(&self) -> &str { &self.pane_id }
    pub fn title(&self) -> &str { &self.title }
    pub fn set_title(&mut self, title: &str) { self.title = title.to_string(); }
    pub fn scroll_up(&mut self, lines: usize) { self.scroll_offset = self.scroll_offset.saturating_add(lines); }
    pub fn scroll_down(&mut self, lines: usize) { self.scroll_offset = self.scroll_offset.saturating_sub(lines); }
    pub fn reset_scroll(&mut self) { self.scroll_offset = 0; }
}

impl IntoElement for TerminalView {
    type Element = Component<Self>;
    fn into_element(self) -> Self::Element { Component::new(self) }
}

impl RenderOnce for TerminalView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let content = self.content.lock().unwrap().clone();
        let lines_to_show = 50;
        let start_idx = content.line_count().saturating_sub(lines_to_show + self.scroll_offset);
        let end_idx = content.line_count().saturating_sub(self.scroll_offset);

        div()
            .id("terminal-view")
            .size_full().flex().flex_col()
            .bg(rgb(0x1a1a1a)).text_color(rgb(0xcccccc))
            .font_family("Menlo").text_size(px(12.))
            .child(
                div()
                    .flex().flex_row().items_center()
                    .px(px(8.)).py(px(4.))
                    .bg(rgb(0x2d2d2d)).border_b_1().border_color(rgb(0x3d3d3d))
                    .child(
                        div().text_size(px(11.)).text_color(rgb(0x999999))
                            .child(format!("🖥 {}", self.title))
                    )
            )
            .child(
                div()
                    .id("terminal-content")
                    .flex_1().p(px(4.))
                    .overflow_y_scroll()
                    .children(
                        content.lines[start_idx..end_idx]
                            .iter()
                            .map(|line| {
                                div()
                                    .h(px(14.))
                                    .child(if line.text.is_empty() {
                                        SharedString::from(" ")
                                    } else {
                                        SharedString::from(line.text.clone())
                                    })
                                    .into_any_element()
                            })
                            .collect::<Vec<_>>()
                    )
            )
    }
}

impl Default for TerminalView {
    fn default() -> Self { Self::new("default", "Terminal") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_content_creation() {
        let content = TerminalContent::new();
        assert_eq!(content.line_count(), 0);
        assert!(content.cursor_position.is_none());
    }

    #[test]
    fn test_terminal_content_from_string() {
        let content = TerminalContent::from_string("Line 1\nLine 2\nLine 3");
        assert_eq!(content.line_count(), 3);
        assert_eq!(content.lines[0].text, "Line 1");
    }

    #[test]
    fn test_terminal_content_to_string() {
        let content = TerminalContent::from_string("Hello\nWorld");
        assert_eq!(content.to_string(), "Hello\nWorld");
    }

    #[test]
    fn test_terminal_content_update() {
        let mut content = TerminalContent::from_string("Old");
        content.update("New\nLines");
        assert_eq!(content.line_count(), 2);
        assert_eq!(content.lines[0].text, "New");
    }

    #[test]
    fn test_terminal_line() {
        let line = TerminalLine::new("Hello World");
        assert_eq!(line.text, "Hello World");
        assert_eq!(line.len(), 11);
        assert!(!line.is_empty());
    }

    #[test]
    fn test_empty_terminal_line() {
        let line = TerminalLine::new("");
        assert!(line.is_empty());
    }

    #[test]
    fn test_color_creation() {
        let color = Color::new(100, 150, 200);
        assert_eq!(color.r, 100);
        assert_eq!(color.g, 150);
        assert_eq!(color.b, 200);
    }

    #[test]
    fn test_common_colors() {
        assert_eq!(Color::black().r, 0);
        assert_eq!(Color::white().r, 255);
        assert_eq!(Color::red().r, 255);
        assert_eq!(Color::green().g, 255);
        assert_eq!(Color::blue().b, 255);
    }

    #[test]
    fn test_terminal_view_creation() {
        let view = TerminalView::new("session:0.0", "main");
        assert_eq!(view.pane_id(), "session:0.0");
        assert_eq!(view.title(), "main");
    }

    #[test]
    fn test_terminal_view_update_content() {
        let mut view = TerminalView::new("pane-1", "zsh");
        view.update_content("Test content\nSecond line");
        let content = view.content.lock().unwrap();
        assert_eq!(content.line_count(), 2);
        assert_eq!(content.lines[0].text, "Test content");
    }

    #[test]
    fn test_terminal_view_scroll() {
        let mut view = TerminalView::new("pane-1", "zsh");
        assert_eq!(view.scroll_offset, 0);
        view.scroll_up(5);
        assert_eq!(view.scroll_offset, 5);
        view.scroll_down(2);
        assert_eq!(view.scroll_offset, 3);
        view.reset_scroll();
        assert_eq!(view.scroll_offset, 0);
    }

    #[test]
    fn test_terminal_view_set_title() {
        let mut view = TerminalView::new("pane-1", "old");
        view.set_title("new");
        assert_eq!(view.title(), "new");
    }

    #[test]
    fn test_color_to_hsla() {
        let color = Color::new(255, 0, 0);
        let _ = color.to_hsla();
    }
}
