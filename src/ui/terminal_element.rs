//! Zed-style terminal element: 1 element, direct paint_quad + shape_line.
//! Holds RenderableGrid (from renderer), does NOT hold engine. TerminalElement ONLY paint.

use alacritty_terminal::index::Point as AlacPoint;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::CursorShape;
use crate::ui::terminal_renderer::{rgb_to_hsla, CacheKey, CursorLayout, RenderableGrid, ShapedLineCache};
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Display cursor: viewport-relative (line, col). From AlacPoint + display_offset.
#[derive(Debug, Clone, Copy)]
pub struct DisplayCursor {
    pub line: i32,
    pub col: usize,
}

impl DisplayCursor {
    pub fn from(point: AlacPoint, display_offset: usize) -> Self {
        Self {
            line: point.line.0 + display_offset as i32,
            col: point.column.0,
        }
    }

    /// From CursorLayout when already in viewport-relative coordinates.
    pub fn from_layout(layout: &CursorLayout) -> Self {
        Self {
            line: layout.line,
            col: layout.col,
        }
    }

    pub fn line(&self) -> i32 {
        self.line
    }

    pub fn col(&self) -> usize {
        self.col
    }
}

/// Zed-style terminal element: 1 element, direct paint_quad + shape_line.
/// Holds RenderableGrid (from renderer), does NOT hold engine. TerminalElement ONLY paint.
pub struct TerminalElement {
    grid: RenderableGrid,
    cols: u16,
    rows: u16,
    cell_width: Pixels,
    cell_height: Pixels,
    /// When Some, use cache for shape_line to avoid 200+ Harfbuzz calls per frame (vim scroll).
    shaped_line_cache: Option<Rc<RefCell<ShapedLineCache<ShapedLine>>>>,
}

impl TerminalElement {
    pub fn new(
        grid: RenderableGrid,
        cols: u16,
        rows: u16,
        cell_width: Pixels,
        cell_height: Pixels,
        shaped_line_cache: Option<Rc<RefCell<ShapedLineCache<ShapedLine>>>>,
    ) -> Self {
        Self {
            grid,
            cols,
            rows,
            cell_width,
            cell_height,
            shaped_line_cache,
        }
    }

    /// Compute terminal size in pixels (for tests and request_layout).
    pub fn compute_size(&self) -> (Pixels, Pixels) {
        let w = self.cell_width * self.cols as f32;
        let h = self.cell_height * self.rows as f32;
        (w, h)
    }

    /// Cursor position in pixels. Returns None if cursor is outside visible area.
    pub fn cursor_position(
        cursor: DisplayCursor,
        rows: u16,
        cell_width: Pixels,
        cell_height: Pixels,
    ) -> Option<Point<Pixels>> {
        if cursor.line() < 0 || (cursor.line() as u16) >= rows {
            return None;
        }
        Some(Point::new(
            px(cursor.col() as f32 * f32::from(cell_width)),
            px(cursor.line() as f32 * f32::from(cell_height)),
        ))
    }

    /// Effective cursor width for paint. When shaped_width is 0 (e.g. combining marks),
    /// use cell_width to avoid invisible cursor. Otherwise use max(shaped_width, cell_width).
    pub fn cursor_width(shaped_width: Pixels, cell_width: Pixels) -> Pixels {
        if shaped_width == px(0.) {
            cell_width
        } else {
            px(f32::from(shaped_width).max(f32::from(cell_width)))
        }
    }
}

impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let w = (self.cols as f32) * f32::from(self.cell_width);
        let h = (self.rows as f32) * f32::from(self.cell_height);
        let mut style = Style::default();
        style.size.width = px(w).into();
        style.size.height = px(h).into();
        let layout_id = window.request_layout(style, [], cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let origin = bounds.origin;
        let font = font("Menlo");
        let font_size = self.cell_height;

        for rect in &self.grid.background_regions {
            rect.paint(origin, self.cell_width, self.cell_height, window);
        }

        for run in &self.grid.text_runs {
            let fg_hsla = rgb_to_hsla(&run.fg);
            let bg_hsla = rgb_to_hsla(&run.bg);
            let underline = if run.flags.contains(Flags::UNDERLINE)
                || run.flags.intersects(Flags::ALL_UNDERLINES)
            {
                Some(UnderlineStyle {
                    thickness: px(1.),
                    ..Default::default()
                })
            } else {
                None
            };
            let strikethrough = if run.flags.contains(Flags::STRIKEOUT) {
                Some(StrikethroughStyle {
                    thickness: px(1.),
                    ..Default::default()
                })
            } else {
                None
            };
            let style = TextRun {
                len: run.text.len(),
                font: font.clone(),
                color: fg_hsla,
                background_color: Some(bg_hsla),
                underline,
                strikethrough,
            };
            let pos = Point::new(
                origin.x + px(run.start_point.column.0 as f32 * f32::from(self.cell_width)),
                origin.y + px(run.start_point.line.0 as f32 * f32::from(self.cell_height)),
            );

            let paint_result = if let Some(ref cache) = self.shaped_line_cache {
                let key = CacheKey::new(&run.text, &font, font_size, &style);
                let shaped = cache.borrow_mut().get_or_insert(&key, || {
                    window.text_system().shape_line(
                        run.text.clone().into(),
                        font_size,
                        std::slice::from_ref(&style),
                        Some(self.cell_width),
                    )
                });
                shaped.paint(pos, self.cell_height, TextAlign::Left, None, window, cx)
            } else {
                window
                    .text_system()
                    .shape_line(
                        run.text.clone().into(),
                        font_size,
                        std::slice::from_ref(&style),
                        Some(self.cell_width),
                    )
                    .paint(pos, self.cell_height, TextAlign::Left, None, window, cx)
            };

            if let Err(_) = paint_result {
                if let Some(ref cache) = self.shaped_line_cache {
                    cache.borrow_mut().clear();
                }
            }
        }

        if self.grid.cursor_layout.visible
            && self.grid.cursor_layout.shape != CursorShape::Hidden
        {
            let cursor = DisplayCursor::from_layout(&self.grid.cursor_layout);
            if let Some(cursor_pos) = Self::cursor_position(
                cursor,
                self.rows,
                self.cell_width,
                self.cell_height,
            ) {
                let cursor_color = hsla(0.58, 0.52, 0.6, 1.0);
                let x = origin.x + cursor_pos.x;
                let y = origin.y + cursor_pos.y;
                let cw = self.cell_width;
                let ch = self.cell_height;
                let stroke_w = px(2.0f32.max(f32::from(ch) * 0.08));

                match self.grid.cursor_layout.shape {
                    CursorShape::Block => {
                        let bounds = Bounds::new(Point::new(x, y), Size::new(cw, ch));
                        window.paint_quad(fill(bounds, cursor_color));
                    }
                    CursorShape::Beam => {
                        let bounds = Bounds::new(Point::new(x, y), Size::new(stroke_w, ch));
                        window.paint_quad(fill(bounds, cursor_color));
                    }
                    CursorShape::Underline => {
                        let y_bottom = y + ch - stroke_w;
                        let bounds = Bounds::new(
                            Point::new(x, y_bottom),
                            Size::new(cw, stroke_w),
                        );
                        window.paint_quad(fill(bounds, cursor_color));
                    }
                    CursorShape::HollowBlock => {
                        let top = Bounds::new(Point::new(x, y), Size::new(cw, stroke_w));
                        let bottom = Bounds::new(
                            Point::new(x, y + ch - stroke_w),
                            Size::new(cw, stroke_w),
                        );
                        let left = Bounds::new(Point::new(x, y), Size::new(stroke_w, ch));
                        let right = Bounds::new(
                            Point::new(x + cw - stroke_w, y),
                            Size::new(stroke_w, ch),
                        );
                        window.paint_quad(fill(top, cursor_color));
                        window.paint_quad(fill(bottom, cursor_color));
                        window.paint_quad(fill(left, cursor_color));
                        window.paint_quad(fill(right, cursor_color));
                    }
                    CursorShape::Hidden => {}
                }
            }
        }

    }
}

impl IntoElement for TerminalElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::index::{Column, Line};

    #[test]
    fn test_terminal_bounds_compute_size() {
        let grid = RenderableGrid::empty(80, 24);
        let elem = TerminalElement::new(grid, 80, 24, px(8.0), px(16.0), None);
        let (w, h) = elem.compute_size();
        assert_eq!(w, px(640.0), "80 * 8 = 640");
        assert_eq!(h, px(384.0), "24 * 16 = 384");
    }

    #[test]
    fn test_display_cursor_from_point_and_offset() {
        let point = AlacPoint::new(Line(0), Column(5));
        let dc = DisplayCursor::from(point, 0);
        assert_eq!(dc.line(), 0);
        assert_eq!(dc.col(), 5);
    }

    #[test]
    fn test_display_cursor_with_display_offset() {
        let point = AlacPoint::new(Line(-3), Column(10));
        let dc = DisplayCursor::from(point, 5);
        assert_eq!(dc.line(), 2, "-3 + 5 = 2");
        assert_eq!(dc.col(), 10);
    }

    #[test]
    fn test_cursor_position_pixel_coords() {
        let dc = DisplayCursor { line: 1, col: 10 };
        let pos = TerminalElement::cursor_position(dc, 24, px(16.), px(8.));
        assert!(pos.is_some());
        let p = pos.unwrap();
        assert_eq!(p.x, px(160.), "10 * 16 = 160");
        assert_eq!(p.y, px(8.), "1 * 8 = 8");
    }

    #[test]
    fn test_cursor_position_outside_rows_returns_none() {
        let dc = DisplayCursor { line: 30, col: 0 };
        let pos = TerminalElement::cursor_position(dc, 24, px(8.), px(16.));
        assert!(pos.is_none());
    }

    #[test]
    fn test_cursor_position_with_display_offset() {
        // Scrollback: cursor at grid line -3, display_offset 5 -> viewport line 2
        let point = AlacPoint::new(Line(-3), Column(7));
        let dc = DisplayCursor::from(point, 5);
        assert_eq!(dc.line(), 2, "scrollback: -3 + 5 = 2");
        let pos = TerminalElement::cursor_position(dc, 24, px(8.), px(16.));
        assert!(pos.is_some(), "cursor at viewport line 2 should be visible");
        let p = pos.unwrap();
        assert_eq!(p.x, px(56.), "col 7 * 8 = 56");
        assert_eq!(p.y, px(32.), "line 2 * 16 = 32");
    }

    #[test]
    fn test_cursor_width_grapheme_fallback() {
        // shaped_width == 0 (e.g. combining mark) -> use cell_width
        let w = TerminalElement::cursor_width(px(0.), px(8.));
        assert_eq!(w, px(8.), "shaped_width 0 should fall back to cell_width");
        // shaped_width > 0 -> use max(shaped, cell)
        let w = TerminalElement::cursor_width(px(12.), px(8.));
        assert_eq!(w, px(12.), "wide char: shaped 12 > cell 8");
        let w = TerminalElement::cursor_width(px(4.), px(8.));
        assert_eq!(w, px(8.), "narrow char: cell_width wins");
    }
}
