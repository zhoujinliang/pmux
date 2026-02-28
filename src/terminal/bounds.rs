//! TerminalBounds: pixel bounds + cell dimensions for cols/rows computation.
//! Used by ResizeController to derive (cols, rows) from window size.

use gpui::{Bounds, Pixels};

/// Terminal bounds with cell dimensions. Used to compute num_columns and num_lines
/// from pixel size for resize handling.
#[derive(Debug, Clone)]
pub struct TerminalBounds {
    pub cell_width: Pixels,
    pub cell_height: Pixels,
    pub bounds: Bounds<Pixels>,
}

impl TerminalBounds {
    pub fn new(cell_width: Pixels, cell_height: Pixels, bounds: Bounds<Pixels>) -> Self {
        Self {
            cell_width,
            cell_height,
            bounds,
        }
    }

    /// Number of lines from bounds.height / cell_height.
    pub fn num_lines(&self) -> u16 {
        let h = f32::from(self.bounds.size.height);
        let ch = f32::from(self.cell_height);
        if ch <= 0.0 {
            return 1;
        }
        let rows = (h / ch).round() as i32;
        rows.max(1).min(i32::from(u16::MAX)) as u16
    }

    /// Number of columns from bounds.width / cell_width.
    pub fn num_columns(&self) -> u16 {
        let w = f32::from(self.bounds.size.width);
        let cw = f32::from(self.cell_width);
        if cw <= 0.0 {
            return 1;
        }
        let cols = (w / cw).round() as i32;
        cols.max(1).min(i32::from(u16::MAX)) as u16
    }

    /// Clamp (cols, rows) to valid terminal range.
    pub fn clamped_dims(&self) -> (u16, u16) {
        let cols = self.num_columns().max(10).min(500);
        let rows = self.num_lines().max(5).min(200);
        (cols, rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    #[test]
    fn test_terminal_bounds_num_lines_columns() {
        let bounds = Bounds::new(gpui::Point::zero(), gpui::size(px(640.), px(384.)));
        let tb = TerminalBounds::new(px(8.), px(16.), bounds);
        assert_eq!(tb.num_columns(), 80, "640/8 = 80");
        assert_eq!(tb.num_lines(), 24, "384/16 = 24");
    }

    #[test]
    fn test_terminal_bounds_clamped_dims() {
        let bounds = Bounds::new(gpui::Point::zero(), gpui::size(px(100.), px(50.)));
        let tb = TerminalBounds::new(px(8.), px(16.), bounds);
        let (cols, rows) = tb.clamped_dims();
        assert_eq!(cols, 12, "100/8 rounded, clamped min 10 -> 12");
        assert_eq!(rows, 5, "50/16 rounded, clamped min 5 -> 5");
    }
}
