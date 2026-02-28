//! RenderableSnapshot: immutable snapshot of terminal state for layout/paint.
//! Built in single lock from engine.try_renderable_content; outlives the lock.

use alacritty_terminal::grid::Indexed;
use alacritty_terminal::index::Point;
use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::color::Colors;

/// Cells for one row: (Point, Cell) for layout_grid. Owns data to outlive the term lock.
#[derive(Debug, Clone)]
pub struct RowData {
    /// Indexed cells for this row: (point, cell). Point has correct line/column.
    pub cells: Vec<(Point, Cell)>,
}

impl RowData {
    /// Iterator of Indexed<&Cell> for layout_grid.
    pub fn iter(&self) -> impl Iterator<Item = Indexed<&Cell>> + '_ {
        self.cells.iter().map(|(point, cell)| Indexed {
            point: *point,
            cell,
        })
    }
}

/// Minimal snapshot: cols, rows, cursor, row cells. Built once per frame from display_iter.
/// display_offset: for viewport culling, visible_line = grid_line - display_offset.
#[derive(Clone)]
pub struct RenderableSnapshot {
    pub cols: u16,
    pub rows: u16,
    pub cursor_line: i32,
    pub cursor_col: usize,
    /// Cursor shape from DECSCUSR (Block, Beam, Underline, HollowBlock, Hidden).
    pub cursor_shape: alacritty_terminal::vte::ansi::CursorShape,
    pub colors: Colors,
    pub row_data: Vec<RowData>,
    pub display_offset: usize,
}

impl RenderableSnapshot {
    /// Build from renderable content. Consumes display_iter; must be called inside try_renderable_content.
    pub fn from<'a, I>(
        content: &alacritty_terminal::term::RenderableContent<'a>,
        display_iter: I,
        screen_lines: usize,
        cols: usize,
        display_offset: usize,
    ) -> Self
    where
        I: Iterator<Item = Indexed<&'a Cell>>,
    {
        let cursor_line = content.cursor.point.line.0;
        let cursor_col = content.cursor.point.column.0;
        let cursor_shape = content.cursor.shape;
        let colors = content.colors.clone();

        let mut row_data: Vec<RowData> = Vec::new();
        let mut current_row: Vec<(Point, Cell)> = Vec::new();
        let mut current_line: i32 = i32::MIN;

        for indexed in display_iter {
            if indexed.cell.flags.contains(alacritty_terminal::term::cell::Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            let line = indexed.point.line.0;
            if line != current_line {
                if !current_row.is_empty() {
                    row_data.push(RowData {
                        cells: std::mem::take(&mut current_row),
                    });
                }
                current_line = line;
            }
            current_row.push((indexed.point, indexed.cell.clone()));
        }
        if !current_row.is_empty() {
            row_data.push(RowData { cells: current_row });
        }

        Self {
            cols: cols as u16,
            rows: screen_lines as u16,
            cursor_line,
            cursor_col,
            cursor_shape,
            colors,
            row_data,
            display_offset,
        }
    }

    /// Empty snapshot for fallback when try_renderable_content returns None.
    pub fn empty(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            cursor_line: 0,
            cursor_col: 0,
            cursor_shape: alacritty_terminal::vte::ansi::CursorShape::Block,
            colors: Colors::default(),
            row_data: Vec::new(),
            display_offset: 0,
        }
    }
}
