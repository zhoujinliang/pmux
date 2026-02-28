//! layout_grid: converts cell iterator to (Vec<LayoutRect>, Vec<BatchedTextRun>).
//! Reuses group_cells_into_segments logic. Merges background regions (same line only).
//! Handles zerowidth: append to prev run, inherit cluster index.

use crate::terminal::TermBridge;
use crate::ui::terminal_renderer::renderable_grid::{LayoutRect, LayoutTextRun};
use alacritty_terminal::grid::Indexed;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::Rgb;
use gpui::hsla;

/// Convert Rgb to Hsla for LayoutRect. Uses standard RGB to HSL conversion.
pub fn rgb_to_hsla(rgb: &Rgb) -> gpui::Hsla {
    let r = rgb.r as f32 / 255.0;
    let g = rgb.g as f32 / 255.0;
    let b = rgb.b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let (h, s) = if (max - min).abs() < 1e-9 {
        (0.0, 0.0)
    } else {
        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        let h = if (max - r).abs() < 1e-9 {
            (g - b) / d + (if g < b { 6.0 } else { 0.0 })
        } else if (max - g).abs() < 1e-9 {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        (h / 6.0, s)
    };
    hsla(h, s, l, 1.0)
}

/// Merge adjacent LayoutRects on the same line with the same color.
/// No cross-wrap: only merges within a single row.
pub fn merge_background_regions(mut rects: Vec<LayoutRect>) -> Vec<LayoutRect> {
    if rects.is_empty() {
        return rects;
    }
    rects.sort_by_key(|r| (r.point.line, r.point.column));
    let mut merged = Vec::with_capacity(rects.len());
    let mut cur = rects[0].clone();
    for r in rects.into_iter().skip(1) {
        let same_line = r.point.line == cur.point.line;
        let adjacent = r.point.column.0 == cur.point.column.0 + cur.num_of_cells;
        let same_color = r.color == cur.color;
        if same_line && adjacent && same_color {
            cur.num_of_cells += r.num_of_cells;
        } else {
            merged.push(std::mem::replace(&mut cur, r));
        }
    }
    merged.push(cur);
    merged
}

/// Check if a cell is zerowidth: has zerowidth chars in extra, or char has unicode width 0.
fn is_zerowidth_cell(cell: &Cell) -> bool {
    if cell.zerowidth().is_some() {
        return true;
    }
    unicode_width::UnicodeWidthChar::width(cell.c).unwrap_or(0) == 0 && cell.c != '\0'
}

/// layout_grid: takes cells iterator, outputs (Vec<LayoutRect>, Vec<BatchedTextRun>).
/// Reuses group_cells_into_segments logic. Zerowidth chars append to prev run and inherit cluster index.
pub fn layout_grid<'a, I>(
    cells: I,
    colors: &Colors,
) -> (Vec<LayoutRect>, Vec<LayoutTextRun>)
where
    I: Iterator<Item = Indexed<&'a Cell>>,
{
    let mut rects: Vec<LayoutRect> = Vec::new();
    let mut runs: Vec<LayoutTextRun> = Vec::new();
    let mut current_text = String::new();
    let mut current_fg: Option<Rgb> = None;
    let mut current_bg: Option<Rgb> = None;
    let mut current_flags: Option<Flags> = None;
    let mut start_point: Option<Point> = None;
    let mut cell_count: usize = 0;
    let mut cluster_index: usize = 0;

    for indexed in cells {
        let is_wide_spacer = indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER);
        if is_wide_spacer {
            continue;
        }

        let is_zerowidth = is_zerowidth_cell(indexed.cell);
        let fg = TermBridge::resolve_color(indexed.cell.fg, colors);
        let bg = TermBridge::resolve_color(indexed.cell.bg, colors);
        let flags = indexed.cell.flags;

        let style_matches = current_fg.as_ref() == Some(&fg)
            && current_bg.as_ref() == Some(&bg)
            && current_flags == Some(flags);

        if is_zerowidth {
            // Append to previous run, inherit cluster index (don't advance cell_count for cursor)
            if !runs.is_empty() {
                let last = runs.last_mut().unwrap();
                last.text.push(indexed.cell.c);
                // Zerowidth chars: also append any from cell.zerowidth()
                if let Some(zw) = indexed.cell.zerowidth() {
                    last.text.extend(zw.iter().copied());
                }
            } else {
                // No previous run - start one (edge case: leading zerowidth)
                current_text.push(indexed.cell.c);
                if let Some(zw) = indexed.cell.zerowidth() {
                    current_text.extend(zw.iter().copied());
                }
                start_point = Some(indexed.point);
                current_fg = Some(fg);
                current_bg = Some(bg);
                current_flags = Some(flags);
                cell_count = 0; // zerowidth doesn't occupy a cell
                cluster_index = 0;
            }
            continue;
        }

        if style_matches {
            current_text.push(indexed.cell.c);
            if let Some(zw) = indexed.cell.zerowidth() {
                current_text.extend(zw.iter().copied());
            }
            cell_count += 1;
        } else {
            if !current_text.is_empty() {
                let pt = start_point.unwrap_or(indexed.point);
                let bg_rgb = current_bg.unwrap();
                rects.push(LayoutRect::new(
                    pt,
                    cell_count,
                    rgb_to_hsla(&bg_rgb),
                ));
                runs.push(LayoutTextRun {
                    text: std::mem::take(&mut current_text),
                    start_point: pt,
                    cell_count,
                    cluster_index,
                    fg: current_fg.unwrap(),
                    bg: current_bg.unwrap(),
                    flags: current_flags.unwrap(),
                });
            }
            current_text.push(indexed.cell.c);
            if let Some(zw) = indexed.cell.zerowidth() {
                current_text.extend(zw.iter().copied());
            }
            start_point = Some(indexed.point);
            current_fg = Some(fg);
            current_bg = Some(bg);
            current_flags = Some(flags);
            cell_count = 1;
            cluster_index = runs.len();
        }
    }

    if !current_text.is_empty() {
        let pt = start_point.unwrap_or(Point::new(Line(0), Column(0)));
        let bg_rgb = current_bg.unwrap();
        rects.push(LayoutRect::new(pt, cell_count, rgb_to_hsla(&bg_rgb)));
        runs.push(LayoutTextRun {
            text: current_text,
            start_point: pt,
            cell_count,
            cluster_index,
            fg: current_fg.unwrap(),
            bg: current_bg.unwrap(),
            flags: current_flags.unwrap(),
        });
    }

    let rects = merge_background_regions(rects);
    (rects, runs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::vte::ansi::Color;
    use gpui::Hsla;

    fn default_colors() -> Colors {
        Colors::default()
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
            point: Point::new(Line(row), Column(col)),
            cell,
        }
    }

    #[test]
    fn test_layout_grid_empty_cells_returns_empty() {
        let cells: Vec<Indexed<&Cell>> = vec![];
        let colors = default_colors();
        let (rects, runs) = layout_grid(cells.into_iter(), &colors);
        assert!(rects.is_empty());
        assert!(runs.is_empty());
    }

    #[test]
    fn test_layout_grid_same_style_merge() {
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
        let (rects, runs) = layout_grid(cells.into_iter(), &colors);
        assert_eq!(runs.len(), 1, "same style => 1 run");
        assert_eq!(runs[0].text, "hello");
        assert_eq!(runs[0].cell_count, 5);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].num_of_cells, 5);
    }

    #[test]
    fn test_layout_grid_different_backgrounds_create_rects() {
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        let red_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Red);
        // Row 0: "aa" default bg, row 1: "bb" red bg
        let cells_vec = [
            make_cell('a', default_fg, default_bg, Flags::empty()),
            make_cell('a', default_fg, default_bg, Flags::empty()),
            make_cell('b', default_fg, red_bg, Flags::empty()),
            make_cell('b', default_fg, red_bg, Flags::empty()),
        ];
        let cells: Vec<Indexed<&Cell>> = vec![
            indexed(0, 0, &cells_vec[0]),
            indexed(0, 1, &cells_vec[1]),
            indexed(1, 0, &cells_vec[2]),
            indexed(1, 1, &cells_vec[3]),
        ];
        let colors = default_colors();
        let (rects, runs) = layout_grid(cells.into_iter(), &colors);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "aa");
        assert_eq!(runs[1].text, "bb");
        assert_eq!(rects.len(), 2);
        assert_ne!(rects[0].color, rects[1].color, "different backgrounds");
    }

    #[test]
    fn test_zerowidth_cluster_index_inherited() {
        // Base char + zerowidth modifier (e.g. emoji + skin tone U+1F3FF)
        // Zerowidth should append to prev run and share cluster index
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        // U+1F44D = thumbs up, U+1F3FF = dark skin tone modifier (zerowidth)
        let base = make_cell('\u{1F44D}', default_fg, default_bg, Flags::empty());
        let modifier = make_cell('\u{1F3FF}', default_fg, default_bg, Flags::empty());
        let cells: Vec<Indexed<&Cell>> = vec![
            indexed(0, 0, &base),
            indexed(0, 1, &modifier), // zerowidth at col 1 - in terminal often same cell or next
        ];
        let colors = default_colors();
        let (_, runs) = layout_grid(cells.into_iter(), &colors);
        // Zerowidth modifier should be in same run as base
        assert_eq!(runs.len(), 1, "zerowidth should append to prev run");
        assert!(runs[0].text.contains('\u{1F44D}'));
        assert!(runs[0].text.contains('\u{1F3FF}'));
        assert_eq!(runs[0].cell_count, 1, "zerowidth doesn't add to cell_count");
    }

    #[test]
    fn test_layout_rect_creates_valid_bounds() {
        let rect = LayoutRect::new(
            Point::new(Line(0), Column(0)),
            5,
            gpui::Hsla::default(),
        );
        assert_eq!(rect.num_of_cells, 5);
    }

    #[test]
    fn test_merge_background_regions_same_line() {
        let hsla = Hsla::default();
        let rects = vec![
            LayoutRect::new(Point::new(Line(0), Column(0)), 3, hsla),
            LayoutRect::new(Point::new(Line(0), Column(3)), 2, hsla),
        ];
        let merged = merge_background_regions(rects);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].num_of_cells, 5);
    }

    #[test]
    fn test_merge_background_regions_different_lines_no_merge() {
        let hsla = Hsla::default();
        let rects = vec![
            LayoutRect::new(Point::new(Line(0), Column(0)), 3, hsla),
            LayoutRect::new(Point::new(Line(1), Column(0)), 2, hsla),
        ];
        let merged = merge_background_regions(rects);
        assert_eq!(merged.len(), 2, "different lines must not merge");
    }
}
