//! build_frame: snapshot + RowCache + ShapedLineCache -> RenderableGrid.
//! Viewport culling: filters to visible rows (visible_line = grid_line - display_offset).

use alacritty_terminal::index::{Line, Point};
use crate::terminal::RenderableSnapshot;
use crate::ui::terminal_renderer::layout_grid;
use crate::ui::terminal_renderer::renderable_grid::{CursorLayout, LayoutRect, LayoutTextRun, RenderableGrid};
use crate::ui::terminal_renderer::row_cache::{hash_row_cells, RowCache};
use crate::ui::terminal_renderer::shaped_line_cache::ShapedLineCache;

/// Optional column range for wide-terminal culling. (start, end) exclusive end.
/// When None, all columns are included.
pub type VisibleColRange = Option<(usize, usize)>;

/// Build RenderableGrid from snapshot using row cache.
/// Filters to visible rows (visible_line = grid_line - display_offset in [0, screen_lines)).
/// When visible_col_range is Some((start, end)), filters to columns in [start, end) for wide terminals.
/// Output uses viewport-relative coordinates for paint.
pub fn build_frame<V>(
    snapshot: &RenderableSnapshot,
    row_cache: &mut RowCache,
    _shaped_line_cache: &mut ShapedLineCache<V>,
    cursor_visible: bool,
    visible_col_range: VisibleColRange,
) -> RenderableGrid {
    let display_offset = snapshot.display_offset;
    let screen_lines = snapshot.rows as usize;
    let mut all_rects: Vec<LayoutRect> = Vec::new();
    let mut all_runs: Vec<LayoutTextRun> = Vec::new();

    for row_data in &snapshot.row_data {
        let grid_line = row_data
            .cells
            .first()
            .map(|(pt, _)| pt.line.0)
            .unwrap_or(0);
        let visible_line = grid_line - display_offset as i32;
        if visible_line < 0 || (visible_line as usize) >= screen_lines {
            continue;
        }

        // Column culling for wide terminals (Task 4.1)
        let cells_iter: Box<dyn Iterator<Item = _>> = if let Some((col_start, col_end)) = visible_col_range {
            Box::new(
                row_data
                    .cells
                    .iter()
                    .filter(move |(pt, _)| {
                        let c = pt.column.0;
                        c >= col_start && c < col_end
                    })
                    .map(|(pt, cell)| (pt, cell)),
            )
        } else {
            Box::new(row_data.cells.iter().map(|(pt, cell)| (pt, cell)))
        };
        let filtered_row = crate::terminal::RowData {
            cells: cells_iter.map(|(pt, cell)| (*pt, cell.clone())).collect(),
        };
        if filtered_row.cells.is_empty() {
            continue;
        }

        let hash = hash_row_cells(filtered_row.iter(), &snapshot.colors);
        let (rects, runs) = row_cache.get_or_build(hash, || {
            layout_grid(filtered_row.iter(), &snapshot.colors)
        });
        let col_offset = visible_col_range.map(|(s, _)| s).unwrap_or(0);
        let visible_line_i32 = visible_line;
        for mut r in rects {
            let vis_col = r.point.column.0.saturating_sub(col_offset);
            r.point = Point::new(Line(visible_line_i32), alacritty_terminal::index::Column(vis_col));
            all_rects.push(r);
        }
        for mut run in runs {
            let vis_col = run.start_point.column.0.saturating_sub(col_offset);
            run.start_point = Point::new(Line(visible_line_i32), alacritty_terminal::index::Column(vis_col));
            all_runs.push(run);
        }
    }

    let visible_cursor_line = snapshot.cursor_line - display_offset as i32;
    let col_offset = visible_col_range.map(|(s, _)| s).unwrap_or(0);
    let visible_col_width = visible_col_range.map(|(s, e)| e.saturating_sub(s));

    let (visible_cursor_col, cursor_in_col_range) = if let Some(w) = visible_col_width {
        let rel = snapshot.cursor_col.saturating_sub(col_offset);
        (rel, rel < w)
    } else {
        (snapshot.cursor_col, true)
    };

    RenderableGrid {
        background_regions: all_rects,
        text_runs: all_runs,
        cursor_layout: CursorLayout {
            line: visible_cursor_line,
            col: visible_cursor_col,
            visible: cursor_visible && cursor_in_col_range,
            shape: snapshot.cursor_shape,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{RenderableSnapshot, RowData};
    use alacritty_terminal::index::{Column, Line, Point};
    use alacritty_terminal::term::cell::Cell;
    use alacritty_terminal::term::color::Colors;
    use alacritty_terminal::vte::ansi::{Color, CursorShape};

    fn make_cell(c: char) -> Cell {
        let default_fg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground);
        let default_bg = Color::Named(alacritty_terminal::vte::ansi::NamedColor::Background);
        Cell {
            c,
            fg: default_fg,
            bg: default_bg,
            ..Default::default()
        }
    }

    #[test]
    fn test_visible_bounds_partial_intersection_filters_rows() {
        // display_offset=10, screen_lines=24.
        // Rows at grid_line 5 (visible_line -5) and 35 (visible_line 25) should be filtered out.
        // Rows at grid_line 10, 15, 20, 25 should be kept.
        let display_offset = 10;
        let screen_lines = 24;
        let colors = Colors::default();

        let row_at = |grid_line: i32| RowData {
            cells: vec![(
                Point::new(Line(grid_line), Column(0)),
                make_cell('x'),
            )],
        };

        let snapshot = RenderableSnapshot {
            cols: 80,
            rows: screen_lines as u16,
            cursor_line: 15,
            cursor_col: 0,
            cursor_shape: CursorShape::Block,
            colors: colors.clone(),
            row_data: vec![
                row_at(5),   // filtered: visible_line -5
                row_at(10),  // kept: visible_line 0
                row_at(15),  // kept: visible_line 5
                row_at(20),  // kept: visible_line 10
                row_at(25),  // kept: visible_line 15
                row_at(35),  // filtered: visible_line 25 >= 24
            ],
            display_offset,
        };

        let mut row_cache = RowCache::new(100);
        let mut shaped_line_cache = ShapedLineCache::new(100);
        let grid = build_frame(
            &snapshot,
            &mut row_cache,
            &mut shaped_line_cache,
            true,
            None,
        );

        // Only 4 rows should be in output (grid_lines 10, 15, 20, 25)
        let visible_lines: Vec<i32> = grid
            .background_regions
            .iter()
            .map(|r| r.point.line.0)
            .collect();
        assert_eq!(
            visible_lines,
            vec![0, 5, 10, 15],
            "visible_line = grid_line - display_offset; filtered rows 5 and 35"
        );

        // Cursor at grid_line 15 -> visible_line 5
        assert_eq!(grid.cursor_layout.line, 5);
    }
}
