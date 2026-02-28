//! RenderableGrid: output of layout_grid + RowCache. TerminalElement consumes this.

use alacritty_terminal::index::Point;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::Rgb;
use gpui::{fill, px, Hsla, Pixels};

/// Alacritty Point (line, column) - alias for layout use.
pub type AlacPoint = Point;

/// Rectangular region for background paint_quad.
/// Uses cell-based coordinates (point, num_of_cells) for layout; paint() converts to pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutRect {
    pub point: AlacPoint,
    pub num_of_cells: usize,
    pub color: Hsla,
}

impl LayoutRect {
    pub fn new(point: AlacPoint, num_of_cells: usize, color: Hsla) -> Self {
        Self {
            point,
            num_of_cells,
            color,
        }
    }

    /// Paint this rect using paint_quad. origin is added to position (bounds.origin from Element::paint).
    pub fn paint(
        &self,
        origin: gpui::Point<Pixels>,
        cell_width: Pixels,
        cell_height: Pixels,
        window: &mut gpui::Window,
    ) {
        let cw = f32::from(cell_width);
        let ch = f32::from(cell_height);
        let x = px(self.point.column.0 as f32 * cw) + origin.x;
        let y = px(self.point.line.0 as f32 * ch) + origin.y;
        let width = px(self.num_of_cells as f32 * cw);
        let height = cell_height;
        let position = gpui::Point::new(x, y);
        let size = gpui::Size::new(width, height);
        let bounds = gpui::Bounds::new(position, size);
        window.paint_quad(fill(bounds, self.color));
    }
}

/// Cursor layout for paint: viewport-relative position and shape.
#[derive(Debug, Clone)]
pub struct CursorLayout {
    /// Line (viewport-relative)
    pub line: i32,
    /// Column
    pub col: usize,
    /// Whether cursor is visible (DECTCEM + focus)
    pub visible: bool,
    /// Shape from DECSCUSR: Block, Beam, Underline, HollowBlock, Hidden
    pub shape: alacritty_terminal::vte::ansi::CursorShape,
}

impl Default for CursorLayout {
    fn default() -> Self {
        Self {
            line: 0,
            col: 0,
            visible: false,
            shape: alacritty_terminal::vte::ansi::CursorShape::Block,
        }
    }
}

/// Layout-time text run from layout_grid. Contains fg, bg, flags for TextRun creation in paint.
#[derive(Debug, Clone)]
pub struct LayoutTextRun {
    pub text: String,
    pub start_point: AlacPoint,
    pub cell_count: usize,
    /// Cluster index for cursor positioning; zerowidth chars inherit from base.
    pub cluster_index: usize,
    pub fg: Rgb,
    pub bg: Rgb,
    pub flags: Flags,
}

/// Output of terminal_renderer::build_frame. TerminalElement ONLY consumes and paints.
#[derive(Debug, Clone)]
pub struct RenderableGrid {
    pub background_regions: Vec<LayoutRect>,
    pub text_runs: Vec<LayoutTextRun>,
    pub cursor_layout: CursorLayout,
}

impl RenderableGrid {
    /// Minimal empty grid for Phase 1.1 skeleton. Phase 1.5+ fills from build_frame.
    pub fn empty(_cols: u16, _rows: u16) -> Self {
        Self {
            background_regions: Vec::new(),
            text_runs: Vec::new(),
            cursor_layout: CursorLayout::default(),
        }
    }
}
