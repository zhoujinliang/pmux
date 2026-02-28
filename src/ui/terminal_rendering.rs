//! Terminal rendering optimization: style-run batching
//!
//! ## Style-run batching concept
//!
//! Instead of creating one GPUI element per terminal cell (O(cells)), we group consecutive cells
//! with identical styling (fg, bg, flags) into **StyledSegments**. Each segment becomes one element.
//! Typical terminal content has long runs of uniform styling (e.g. a prompt line, plain output),
//! so we achieve O(style-runs) instead of O(cells).
//!
//! **Example**: 80 uniform cells in a row → 1 segment → 1 element (vs 80 elements with per-cell).
//!
//! ## Why it's more efficient than per-cell
//!
//! - **Fewer elements**: GPUI must allocate, layout, and render each element. Reducing from ~1,944
//!   to ~60 elements per frame (80×24 terminal) cuts allocation and layout work by ~97%.
//! - **Fewer draw calls**: Each element typically becomes a draw primitive; batching reduces GPU work.
//! - **Better cache locality**: Fewer allocations improve memory access patterns.
//!
//! ## Rendering pipeline
//!
//! ```text
//! Grid (alacritty_terminal)  →  display_iter()  →  Iterator<Indexed<Cell>>
//!        ↓
//! Viewport culling (visible_start..visible_end)  →  only visible rows
//!        ↓
//! group_cells_into_segments()  →  Vec<StyledSegment> per row
//!        ↓
//! Row cache (hash → Vec<StyledSegment>)  →  skip rebuild for unchanged rows
//!        ↓
//! render_batch_row()  →  one flex row with styled spans per row
//!        ↓
//! GPUI  →  div().children(row_elements)
//! ```
use crate::terminal::TermBridge;
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::vte::ansi::Rgb;
use gpui::prelude::*;
use gpui::*;
use std::hash::{Hash, Hasher};

/// A run of consecutive cells with identical styling. Used for batched rendering
/// to reduce element count from O(cells) to O(style-runs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSegment {
    pub text: String,
    pub fg: Rgb,
    pub bg: Rgb,
    pub flags: Flags,
}

/// Fast content hash for a row's segments. Used as cache key for row-level caching.
/// Includes text content and style info (fg, bg, flags). Uses DefaultHasher (non-cryptographic).
pub fn hash_row_content(segments: &[StyledSegment]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for seg in segments {
        seg.text.hash(&mut hasher);
        seg.fg.r.hash(&mut hasher);
        seg.fg.g.hash(&mut hasher);
        seg.fg.b.hash(&mut hasher);
        seg.bg.r.hash(&mut hasher);
        seg.bg.g.hash(&mut hasher);
        seg.bg.b.hash(&mut hasher);
        seg.flags.bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// Groups consecutive cells with the same (fg, bg, flags) into StyledSegments.
///
/// **Algorithm**: Single pass over cells. When (fg, bg, flags) matches the current run, append
/// the character to `current_text`. When it changes, push the current segment and start a new one.
/// Skips WIDE_CHAR_SPACER cells (same as per-cell rendering). Uses TermBridge::resolve_color
/// for Color → Rgb resolution.
///
/// **Example**: 80 uniform cells → 1 segment instead of 80 elements. A row with "hello" (normal),
/// "world" (bold), "!" (normal) → 3 segments.
pub fn group_cells_into_segments<'a, I>(
    cells: I,
    colors: &alacritty_terminal::term::color::Colors,
) -> Vec<StyledSegment>
where
    I: Iterator<Item = alacritty_terminal::grid::Indexed<&'a Cell>>,
{
    let mut segments = Vec::new();
    let mut current_text = String::new();
    let mut current_fg: Option<Rgb> = None;
    let mut current_bg: Option<Rgb> = None;
    let mut current_flags: Option<Flags> = None;
    let mut cell_idx: usize = 0;
    let mut skipped_wide_spacer: usize = 0;

    for indexed in cells {
        let is_wide_spacer = indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER);
        if is_wide_spacer {
            skipped_wide_spacer += 1;
            // #region agent log
            let _ = std::fs::OpenOptions::new().create(true).append(true).open("/Users/matt.chow/workspace/pmux/.cursor/debug-12d1c8.log").and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "{{\"sessionId\":\"12d1c8\",\"id\":\"log_{}_spacer{}\",\"timestamp\":{},\"location\":\"terminal_rendering.rs:93\",\"message\":\"WIDE_CHAR_SPACER skipped\",\"data\":{{\"cell_idx\":{},\"line\":{},\"column\":{},\"char\":\"{}\",\"hypothesis\":\"D\"}},\"runId\":\"debug1\",\"hypothesisId\":\"D\"}}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), cell_idx, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), cell_idx, indexed.point.line.0, indexed.point.column.0, indexed.cell.c)
            });
            // #endregion
            continue;
        }

        let fg = TermBridge::resolve_color(indexed.cell.fg, colors);
        let bg = TermBridge::resolve_color(indexed.cell.bg, colors);
        let flags = indexed.cell.flags;

        let style_matches = current_fg.as_ref() == Some(&fg)
            && current_bg.as_ref() == Some(&bg)
            && current_flags == Some(flags);

        cell_idx += 1;
        if style_matches {
            current_text.push(indexed.cell.c);
        } else {
            if !current_text.is_empty() {
                segments.push(StyledSegment {
                    text: std::mem::take(&mut current_text),
                    fg: current_fg.unwrap(),
                    bg: current_bg.unwrap(),
                    flags: current_flags.unwrap(),
                });
            }
            current_text.push(indexed.cell.c);
            current_fg = Some(fg);
            current_bg = Some(bg);
            current_flags = Some(flags);
        }
    }

    if !current_text.is_empty() {
        segments.push(StyledSegment {
            text: current_text,
            fg: current_fg.unwrap(),
            bg: current_bg.unwrap(),
            flags: current_flags.unwrap(),
        });
    }

    segments
}

/// Count total segments across all rows for benchmarking. Used by terminal_bench.
/// Returns (segment_count, row_count) - element count = segment_count + row_count.
pub fn count_segments_for_benchmark<'a, I>(
    display_iter: I,
    colors: &alacritty_terminal::term::color::Colors,
) -> (usize, usize)
where
    I: Iterator<Item = alacritty_terminal::grid::Indexed<&'a Cell>>,
{
    count_segments_for_benchmark_with_viewport(display_iter, colors, 0, usize::MAX)
}

/// Count segments for visible rows only (viewport culling). Used by terminal_bench Phase 3.
/// visible_start..visible_end is the viewport range (scroll_offset..scroll_offset+screen_lines).
/// Returns (segment_count, row_count) for visible rows only.
pub fn count_segments_for_benchmark_with_viewport<'a, I>(
    display_iter: I,
    colors: &alacritty_terminal::term::color::Colors,
    visible_start: usize,
    visible_end: usize,
) -> (usize, usize)
where
    I: Iterator<Item = alacritty_terminal::grid::Indexed<&'a Cell>>,
{
    let mut row_cells: Vec<Vec<alacritty_terminal::grid::Indexed<&'a Cell>>> = Vec::new();
    let mut current_row: Vec<alacritty_terminal::grid::Indexed<&'a Cell>> = Vec::new();
    let mut current_line: i32 = i32::MIN;
    let mut viewport_line: usize = 0;

    for indexed in display_iter {
        if indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }
        let row = indexed.point.line.0;
        if row != current_line {
            if !current_row.is_empty() {
                if visible_start <= viewport_line && viewport_line < visible_end {
                    row_cells.push(std::mem::take(&mut current_row));
                } else {
                    current_row.clear();
                }
            }
            if current_line != i32::MIN {
                viewport_line = viewport_line.saturating_add(1);
            }
            current_line = row;
        }
        if visible_start <= viewport_line && viewport_line < visible_end {
            current_row.push(indexed);
        }
    }
    if !current_row.is_empty() && visible_start <= viewport_line && viewport_line < visible_end {
        row_cells.push(current_row);
    }

    let row_count = row_cells.len();
    let segment_count: usize = row_cells
        .into_iter()
        .map(|cells| group_cells_into_segments(cells.into_iter(), colors).len())
        .sum();
    (segment_count, row_count)
}

/// Returns segments for each visible row. Used by terminal_bench Phase 4 (row cache).
pub fn segments_per_visible_row_for_benchmark<'a, I>(
    display_iter: I,
    colors: &alacritty_terminal::term::color::Colors,
    visible_start: usize,
    visible_end: usize,
) -> Vec<Vec<StyledSegment>>
where
    I: Iterator<Item = alacritty_terminal::grid::Indexed<&'a Cell>>,
{
    let mut row_cells: Vec<Vec<alacritty_terminal::grid::Indexed<&'a Cell>>> = Vec::new();
    let mut current_row: Vec<alacritty_terminal::grid::Indexed<&'a Cell>> = Vec::new();
    let mut current_line: i32 = i32::MIN;
    let mut viewport_line: usize = 0;

    for indexed in display_iter {
        if indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }
        let row = indexed.point.line.0;
        if row != current_line {
            if !current_row.is_empty() {
                if visible_start <= viewport_line && viewport_line < visible_end {
                    row_cells.push(std::mem::take(&mut current_row));
                } else {
                    current_row.clear();
                }
            }
            if current_line != i32::MIN {
                viewport_line = viewport_line.saturating_add(1);
            }
            current_line = row;
        }
        if visible_start <= viewport_line && viewport_line < visible_end {
            current_row.push(indexed);
        }
    }
    if !current_row.is_empty() && visible_start <= viewport_line && viewport_line < visible_end {
        row_cells.push(current_row);
    }

    row_cells
        .into_iter()
        .map(|cells| group_cells_into_segments(cells.into_iter(), colors))
        .collect()
}

/// Line height in pixels - matches terminal_view for consistent layout.
const LINE_HEIGHT: f32 = 20.0;

#[inline]
fn rgb_u8(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Renders a row of styled segments as a single flex row with styled text spans.
///
/// **Segment rendering**: Each StyledSegment becomes one div with text_color, bg, font_weight,
/// underline applied. All segments are children of one flex row—one GPUI element per row plus
/// one per segment (plus cursor when shown). This is the core of style-run batching: many cells
/// become few segments, few elements.
///
/// **Cursor handling**: Cursor is a separate element inserted at cursor_col (character index).
/// When the cursor falls within a segment, we split that segment into before/after and insert
/// the cursor element between. When show_cursor is false or cursor_col is None, no cursor is rendered.
pub fn render_batch_row(
    segments: Vec<StyledSegment>,
    cursor_col: Option<usize>,
    show_cursor: bool,
) -> gpui::AnyElement {
    let base_row = || {
        div()
            .h(px(LINE_HEIGHT))
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .overflow_x_hidden()
            .whitespace_nowrap()
    };

    let segment_to_element = |s: &StyledSegment| {
        let fg_rgb = rgb(rgb_u8(s.fg.r, s.fg.g, s.fg.b));
        let bg_rgb = rgb(rgb_u8(s.bg.r, s.bg.g, s.bg.b));
        let mut el = div()
            .text_color(fg_rgb)
            .bg(bg_rgb)
            .font_family("Menlo")
            .text_size(px(12.));
        if s.flags.contains(Flags::BOLD) {
            el = el.font_weight(FontWeight::BOLD);
        }
        if s.flags.contains(Flags::UNDERLINE) {
            el = el.underline();
        }
        el.child(SharedString::from(s.text.clone()))
            .into_any_element()
    };

    let cursor_el = || {
        div()
            .h(px(LINE_HEIGHT))
            .w(px(8.))
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0x74ade8))
            .text_color(rgb(0x282c34))
            .child(SharedString::from(" "))
            .into_any_element()
    };

    if !show_cursor || cursor_col.is_none() {
        let children: Vec<AnyElement> = segments.iter().map(segment_to_element).collect();
        return base_row().children(children).into_any_element();
    }

    let cursor_col = cursor_col.unwrap();
    let mut elements: Vec<AnyElement> = Vec::new();
    let mut acc: usize = 0;
    let mut cursor_inserted = false;
    // #region agent log
    let _ = std::fs::OpenOptions::new().create(true).append(true).open("/Users/matt.chow/workspace/pmux/.cursor/debug-12d1c8.log").and_then(|mut f| {
        use std::io::Write;
        writeln!(f, "{{\"sessionId\":\"12d1c8\",\"id\":\"log_{}_start\",\"timestamp\":{},\"location\":\"terminal_rendering.rs:313\",\"message\":\"render_batch_row start\",\"data\":{{\"cursor_col\":{},\"segment_count\":{},\"hypothesis\":\"A\"}},\"runId\":\"debug1\",\"hypothesisId\":\"A\"}}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), cursor_col, segments.len())
    });
    // #endregion

    for (seg_idx, seg) in segments.iter().enumerate() {
        let len = seg.text.chars().count();
        let text_preview: String = seg.text.chars().take(20).collect();
        // #region agent log
        let _ = std::fs::OpenOptions::new().create(true).append(true).open("/Users/matt.chow/workspace/pmux/.cursor/debug-12d1c8.log").and_then(|mut f| {
            use std::io::Write;
            writeln!(f, "{{\"sessionId\":\"12d1c8\",\"id\":\"log_{}_seg{}\",\"timestamp\":{},\"location\":\"terminal_rendering.rs:325\",\"message\":\"segment info\",\"data\":{{\"seg_idx\":{},\"text\":\"{}\",\"len_chars\":{},\"acc_before\":{},\"cursor_col\":{},\"hypothesis\":\"A\"}},\"runId\":\"debug1\",\"hypothesisId\":\"A\"}}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), seg_idx, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), seg_idx, text_preview.replace("\\", "\\\\").replace("\"", "\\\""), len, acc, cursor_col)
        });
        // #endregion
        if !cursor_inserted && cursor_col <= acc {
            // #region agent log
            let _ = std::fs::OpenOptions::new().create(true).append(true).open("/Users/matt.chow/workspace/pmux/.cursor/debug-12d1c8.log").and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "{{\"sessionId\":\"12d1c8\",\"id\":\"log_{}_insert1\",\"timestamp\":{},\"location\":\"terminal_rendering.rs:320\",\"message\":\"cursor inserted at boundary\",\"data\":{{\"cursor_col\":{},\"acc\":{},\"seg_idx\":{},\"hypothesis\":\"B\"}},\"runId\":\"debug1\",\"hypothesisId\":\"B\"}}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), cursor_col, acc, seg_idx)
            });
            // #endregion
            elements.push(cursor_el());
            cursor_inserted = true;
        }
        if !cursor_inserted && cursor_col < acc + len {
            let offset = cursor_col - acc;
            let char_offset = seg.text.char_indices().nth(offset).map(|(i, _)| i).unwrap_or(seg.text.len());
            let (before, after) = seg.text.split_at(char_offset);
            // #region agent log
            let _ = std::fs::OpenOptions::new().create(true).append(true).open("/Users/matt.chow/workspace/pmux/.cursor/debug-12d1c8.log").and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "{{\"sessionId\":\"12d1c8\",\"id\":\"log_{}_insert2\",\"timestamp\":{},\"location\":\"terminal_rendering.rs:335\",\"message\":\"cursor inserted inside segment\",\"data\":{{\"cursor_col\":{},\"acc\":{},\"len\":{},\"offset\":{},\"char_offset\":{},\"seg_idx\":{},\"before\":\"{}\",\"after\":\"{}\",\"hypothesis\":\"A\"}},\"runId\":\"debug1\",\"hypothesisId\":\"A\"}}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), cursor_col, acc, len, offset, char_offset, seg_idx, before.replace("\\", "\\\\").replace("\"", "\\\""), after.replace("\\", "\\\\").replace("\"", "\\\""))
            });
            // #endregion
            if !before.is_empty() {
                elements.push(segment_to_element(&StyledSegment {
                    text: before.to_string(),
                    fg: seg.fg,
                    bg: seg.bg,
                    flags: seg.flags,
                }));
            }
            elements.push(cursor_el());
            cursor_inserted = true;
            if !after.is_empty() {
                elements.push(segment_to_element(&StyledSegment {
                    text: after.to_string(),
                    fg: seg.fg,
                    bg: seg.bg,
                    flags: seg.flags,
                }));
            }
        } else {
            elements.push(segment_to_element(seg));
        }
        acc += len;
    }

    if !cursor_inserted && cursor_col >= acc {
        // #region agent log
        let _ = std::fs::OpenOptions::new().create(true).append(true).open("/Users/matt.chow/workspace/pmux/.cursor/debug-12d1c8.log").and_then(|mut f| {
            use std::io::Write;
            writeln!(f, "{{\"sessionId\":\"12d1c8\",\"id\":\"log_{}_insert3\",\"timestamp\":{},\"location\":\"terminal_rendering.rs:360\",\"message\":\"cursor inserted at end\",\"data\":{{\"cursor_col\":{},\"acc\":{},\"hypothesis\":\"B\"}},\"runId\":\"debug1\",\"hypothesisId\":\"B\"}}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), cursor_col, acc)
        });
        // #endregion
        elements.push(cursor_el());
    }

    base_row().children(elements).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::grid::Indexed;
    use alacritty_terminal::term::cell::Flags;
    use alacritty_terminal::vte::ansi::Color;

    fn default_colors() -> alacritty_terminal::term::color::Colors {
        alacritty_terminal::term::color::Colors::default()
    }

    fn make_cell(c: char, fg: Color, bg: Color, flags: Flags) -> Cell {
        Cell {
            c,
            fg,
            bg,
            flags,
            ..Default::default()
        }
    }

    fn indexed(row: i32, col: i32, cell: &Cell) -> Indexed<&Cell> {
        Indexed {
            point: alacritty_terminal::grid::Point {
                line: alacritty_terminal::grid::Line(row),
                column: alacritty_terminal::grid::Column(col),
            },
            cell,
        }
    }

    #[test]
    fn test_empty_iterator_returns_empty_segments() {
        let cells: Vec<Indexed<&Cell>> = vec![];
        let colors = default_colors();
        let segments = group_cells_into_segments(cells.into_iter(), &colors);
        assert!(segments.is_empty());
    }

    #[test]
    fn test_single_cell_single_segment() {
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        let cell = make_cell('x', default_fg, default_bg, Flags::empty());
        let cells = vec![indexed(0, 0, &cell)];
        let colors = default_colors();
        let segments = group_cells_into_segments(cells.into_iter(), &colors);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "x");
        assert_eq!(segments[0].flags, Flags::empty());
    }

    #[test]
    fn test_same_style_cells_merged_into_one_segment() {
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        let cells_vec = [
            make_cell('h', default_fg, default_bg, Flags::empty()),
            make_cell('e', default_fg, default_bg, Flags::empty()),
            make_cell('l', default_fg, default_bg, Flags::empty()),
            make_cell('l', default_fg, default_bg, Flags::empty()),
            make_cell('o', default_fg, default_bg, Flags::empty()),
        ];
        let cells: Vec<Indexed<&Cell>> = cells_vec
            .iter()
            .enumerate()
            .map(|(i, c)| indexed(0, i as i32, c))
            .collect();
        let colors = default_colors();
        let segments = group_cells_into_segments(cells.into_iter(), &colors);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "hello");
    }

    #[test]
    fn test_different_styles_create_separate_segments() {
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        let red = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Red);
        let cells_vec = [
            make_cell('a', default_fg, default_bg, Flags::empty()),
            make_cell('b', red, default_bg, Flags::empty()),
            make_cell('c', default_fg, default_bg, Flags::empty()),
        ];
        let cells: Vec<Indexed<&Cell>> = cells_vec
            .iter()
            .enumerate()
            .map(|(i, c)| indexed(0, i as i32, c))
            .collect();
        let colors = default_colors();
        let segments = group_cells_into_segments(cells.into_iter(), &colors);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].text, "a");
        assert_eq!(segments[1].text, "b");
        assert_eq!(segments[2].text, "c");
    }

    #[test]
    fn test_different_flags_create_separate_segments() {
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        let cells_vec = [
            make_cell('n', default_fg, default_bg, Flags::empty()),
            make_cell('b', default_fg, default_bg, Flags::BOLD),
            make_cell('u', default_fg, default_bg, Flags::UNDERLINE),
            make_cell('n', default_fg, default_bg, Flags::empty()),
        ];
        let cells: Vec<Indexed<&Cell>> = cells_vec
            .iter()
            .enumerate()
            .map(|(i, c)| indexed(0, i as i32, c))
            .collect();
        let colors = default_colors();
        let segments = group_cells_into_segments(cells.into_iter(), &colors);
        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0].text, "n");
        assert_eq!(segments[0].flags, Flags::empty());
        assert_eq!(segments[1].text, "b");
        assert_eq!(segments[1].flags, Flags::BOLD);
        assert_eq!(segments[2].text, "u");
        assert_eq!(segments[2].flags, Flags::UNDERLINE);
        assert_eq!(segments[3].text, "n");
        assert_eq!(segments[3].flags, Flags::empty());
    }

    #[test]
    fn test_wide_char_spacer_skipped() {
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        let cells_vec = [
            make_cell('a', default_fg, default_bg, Flags::empty()),
            make_cell(' ', default_fg, default_bg, Flags::WIDE_CHAR_SPACER),
            make_cell('b', default_fg, default_bg, Flags::empty()),
        ];
        let cells: Vec<Indexed<&Cell>> = cells_vec
            .iter()
            .enumerate()
            .map(|(i, c)| indexed(0, i as i32, c))
            .collect();
        let colors = default_colors();
        let segments = group_cells_into_segments(cells.into_iter(), &colors);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "ab");
    }

    #[test]
    fn test_count_segments_with_viewport_culling() {
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        let colors = default_colors();
        // 3 rows: row0="a", row1="bb", row2="ccc"
        let cells_vec = [
            make_cell('a', default_fg, default_bg, Flags::empty()),
            make_cell('b', default_fg, default_bg, Flags::empty()),
            make_cell('b', default_fg, default_bg, Flags::empty()),
            make_cell('c', default_fg, default_bg, Flags::empty()),
            make_cell('c', default_fg, default_bg, Flags::empty()),
            make_cell('c', default_fg, default_bg, Flags::empty()),
        ];
        let cells: Vec<Indexed<&Cell>> = vec![
            indexed(0, 0, &cells_vec[0]),
            indexed(1, 0, &cells_vec[1]),
            indexed(1, 1, &cells_vec[2]),
            indexed(2, 0, &cells_vec[3]),
            indexed(2, 1, &cells_vec[4]),
            indexed(2, 2, &cells_vec[5]),
        ];
        let (seg_all, row_all) =
            count_segments_for_benchmark_with_viewport(cells.iter().cloned(), &colors, 0, usize::MAX);
        assert_eq!(seg_all, 3, "all rows: 3 segments");
        assert_eq!(row_all, 3, "all rows: 3 rows");
        let (seg_mid, row_mid) =
            count_segments_for_benchmark_with_viewport(cells.iter().cloned(), &colors, 1, 2);
        assert_eq!(seg_mid, 1, "viewport 1..2: 1 segment (row1)");
        assert_eq!(row_mid, 1, "viewport 1..2: 1 row");
        let (seg_none, row_none) =
            count_segments_for_benchmark_with_viewport(cells.iter().cloned(), &colors, 10, 20);
        assert_eq!(seg_none, 0, "viewport 10..20: 0 segments");
        assert_eq!(row_none, 0, "viewport 10..20: 0 rows");
    }

    #[test]
    fn test_bold_then_normal_merged_correctly() {
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        let cells_vec = [
            make_cell('*', default_fg, default_bg, Flags::BOLD),
            make_cell('*', default_fg, default_bg, Flags::BOLD),
            make_cell(' ', default_fg, default_bg, Flags::empty()),
            make_cell('t', default_fg, default_bg, Flags::empty()),
            make_cell('e', default_fg, default_bg, Flags::empty()),
            make_cell('x', default_fg, default_bg, Flags::empty()),
            make_cell('t', default_fg, default_bg, Flags::empty()),
        ];
        let cells: Vec<Indexed<&Cell>> = cells_vec
            .iter()
            .enumerate()
            .map(|(i, c)| indexed(0, i as i32, c))
            .collect();
        let colors = default_colors();
        let segments = group_cells_into_segments(cells.into_iter(), &colors);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "**");
        assert_eq!(segments[0].flags, Flags::BOLD);
        assert_eq!(segments[1].text, " text");
        assert_eq!(segments[1].flags, Flags::empty());
    }

    #[test]
    fn test_hash_row_content_same_segments_same_hash() {
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        let rgb_fg = crate::terminal::TermBridge::resolve_color(default_fg, &default_colors());
        let rgb_bg = crate::terminal::TermBridge::resolve_color(default_bg, &default_colors());
        let segs = vec![
            StyledSegment {
                text: "hello".to_string(),
                fg: rgb_fg,
                bg: rgb_bg,
                flags: Flags::empty(),
            },
        ];
        let h1 = hash_row_content(&segs);
        let h2 = hash_row_content(&segs);
        assert_eq!(h1, h2, "same segments => same hash");
    }

    #[test]
    fn test_hash_row_content_different_segments_different_hash() {
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        let rgb_fg = crate::terminal::TermBridge::resolve_color(default_fg, &default_colors());
        let rgb_bg = crate::terminal::TermBridge::resolve_color(default_bg, &default_colors());
        let segs1 = vec![StyledSegment {
            text: "a".to_string(),
            fg: rgb_fg,
            bg: rgb_bg,
            flags: Flags::empty(),
        }];
        let segs2 = vec![StyledSegment {
            text: "b".to_string(),
            fg: rgb_fg,
            bg: rgb_bg,
            flags: Flags::empty(),
        }];
        assert_ne!(hash_row_content(&segs1), hash_row_content(&segs2));
    }
}
