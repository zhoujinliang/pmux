//! BatchedTextRun: combines adjacent cells with the same style for efficient shaping/paint.
//! Reference: Zed terminal_element.rs BatchedTextRun.

#[cfg(test)]
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::index::Point as AlacPoint;
use gpui::{AbsoluteLength, TextRun};

/// A batched text run that combines multiple adjacent cells with the same style.
#[derive(Debug, Clone)]
pub struct BatchedTextRun {
    pub start_point: AlacPoint,
    pub text: String,
    pub cell_count: usize,
    pub style: TextRun,
    pub font_size: AbsoluteLength,
}

impl BatchedTextRun {
    /// Create a new run from a single character.
    pub fn new_from_char(
        start_point: AlacPoint,
        c: char,
        mut style: TextRun,
        font_size: AbsoluteLength,
    ) -> Self {
        style.len = c.len_utf8();
        let mut text = String::with_capacity(100);
        text.push(c);
        BatchedTextRun {
            start_point,
            text,
            cell_count: 1,
            style,
            font_size,
        }
    }

    /// Check if another style can be appended (same font, color, background, underline, strikethrough).
    pub fn can_append(&self, other_style: &TextRun) -> bool {
        self.style.font == other_style.font
            && self.style.color == other_style.color
            && self.style.background_color == other_style.background_color
            && self.style.underline == other_style.underline
            && self.style.strikethrough == other_style.strikethrough
    }

    /// Append a character that occupies a cell. Increments cell_count and updates style.len.
    pub fn append_char(&mut self, c: char) {
        self.append_char_internal(c, true);
    }

    /// Append zero-width characters (e.g. emoji modifiers). Does not increment cell_count.
    pub fn append_zero_width_chars(&mut self, chars: &[char]) {
        for &c in chars {
            self.append_char_internal(c, false);
        }
    }

    fn append_char_internal(&mut self, c: char, counts_cell: bool) {
        self.text.push(c);
        if counts_cell {
            self.cell_count += 1;
        }
        self.style.len += c.len_utf8();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{font, hsla, px};

    fn point(line: i32, col: usize) -> AlacPoint {
        AlacPoint {
            line: Line(line),
            column: Column(col),
        }
    }

    fn style_a() -> TextRun {
        TextRun {
            len: 0,
            font: font("Menlo"),
            color: gpui::Hsla::default(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }

    fn style_b_different_color() -> TextRun {
        TextRun {
            len: 0,
            font: font("Menlo"),
            color: hsla(1.0, 0.0, 0.0, 1.0),
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }

    #[test]
    fn test_batched_text_run_can_append_same_style() {
        let style = style_a();
        let run = BatchedTextRun::new_from_char(
            point(0, 0),
            'a',
            style.clone(),
            AbsoluteLength::from(px(14.)),
        );
        assert!(run.can_append(&style));
    }

    #[test]
    fn test_batched_text_run_cannot_append_different_font() {
        let style = style_a();
        let run = BatchedTextRun::new_from_char(
            point(0, 0),
            'a',
            style,
            AbsoluteLength::from(px(14.)),
        );
        let mut other_style = style_a();
        other_style.font = font("Courier");
        assert!(!run.can_append(&other_style));
    }

    #[test]
    fn test_batched_text_run_cannot_append_different_style() {
        let style = style_a();
        let run = BatchedTextRun::new_from_char(
            point(0, 0),
            'a',
            style,
            AbsoluteLength::from(px(14.)),
        );
        assert!(!run.can_append(&style_b_different_color()));
    }

    #[test]
    fn test_batched_text_run_append_char_increments_cell_count() {
        let mut run = BatchedTextRun::new_from_char(
            point(0, 0),
            'a',
            style_a(),
            AbsoluteLength::from(px(14.)),
        );
        run.append_char('b');
        assert_eq!(run.cell_count, 2);
        assert_eq!(run.text, "ab");
    }

    #[test]
    fn test_batched_text_run_append_zero_width_chars_does_not_increment_cell_count() {
        let mut run = BatchedTextRun::new_from_char(
            point(0, 0),
            'a',
            style_a(),
            AbsoluteLength::from(px(14.)),
        );
        run.append_zero_width_chars(&['\u{fe0f}']);
        assert_eq!(run.cell_count, 1);
        assert_eq!(run.text, "a\u{fe0f}");
    }
}
