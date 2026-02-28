//! ResizeController: Window bounds observer with 16ms debounce.
//! Resize is driven by window bounds, NOT by request_layout/prepaint/paint.
//! Sends (cols, rows) to resize channel for engine.resize / runtime.resize.

use crate::terminal::TerminalBounds;
use gpui::{point, px, size, Bounds};
use std::sync::atomic::{AtomicBool, Ordering};

/// Approximate pixels per character (matches app_root CHAR_WIDTH_PX).
const CELL_WIDTH_PX: f32 = 8.0;
/// Line height in pixels (matches app_root LINE_HEIGHT_PX).
const CELL_HEIGHT_PX: f32 = 20.0;
/// Pixels for topbar + tabbar + status bar + terminal header.
const CHROME_HEIGHT_PX: f32 = 120.0;
/// ResizeController: observes window bounds, debounces, computes (cols, rows), sends to callback.
/// Used by AppRoot - NOT by TerminalElement or TerminalView.
pub struct ResizeController {
    /// Last (cols, rows) we sent - skip resize if unchanged (float tolerance).
    last_sent: Option<(u16, u16)>,
    /// Pending resize scheduled for next frame.
    pending: AtomicBool,
}

impl ResizeController {
    pub fn new() -> Self {
        Self {
            last_sent: None,
            pending: AtomicBool::new(false),
        }
    }

    /// Compute (cols, rows) from window pixel bounds, sidebar state.
    /// Uses TerminalBounds for consistency.
    pub fn compute_dims_from_bounds(
        bounds_w: f32,
        bounds_h: f32,
        sidebar_visible: bool,
        sidebar_width: f32,
    ) -> (u16, u16) {
        let term_w = if sidebar_visible {
            (bounds_w - sidebar_width).max(80.)
        } else {
            bounds_w.max(80.)
        };
        let term_h = (bounds_h - CHROME_HEIGHT_PX).max(200.);
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(term_w), px(term_h)));
        let tb = TerminalBounds::new(px(CELL_WIDTH_PX), px(CELL_HEIGHT_PX), bounds);
        tb.clamped_dims()
    }

    /// Check if we should schedule a debounced resize. Returns Some((cols, rows)) if dimensions
    /// changed from last_sent. Caller applies resize and then set_last_sent.
    pub fn maybe_resize(
        &mut self,
        cols: u16,
        rows: u16,
    ) -> Option<(u16, u16)> {
        let new_dims = (cols, rows);
        if self.last_sent == Some(new_dims) {
            return None;
        }
        self.last_sent = Some(new_dims);
        Some(new_dims)
    }

    pub fn set_last_sent(&mut self, cols: u16, rows: u16) {
        self.last_sent = Some((cols, rows));
    }

    pub fn last_sent(&self) -> Option<(u16, u16)> {
        self.last_sent
    }

    /// Mark that a debounced flush is pending. Used to coalesce rapid updates.
    pub fn set_pending(&self, pending: bool) {
        self.pending.store(pending, Ordering::Relaxed);
    }

    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed)
    }
}

impl Default for ResizeController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_debounced() {
        // Simulate: 5 rapid "resize" events that produce same (cols, rows).
        // ResizeController.maybe_resize returns Some only on first change.
        let mut ctrl = ResizeController::new();
        let dims = (80u16, 24u16);

        let r1 = ctrl.maybe_resize(dims.0, dims.1);
        assert!(r1.is_some(), "first resize should apply");
        assert_eq!(r1.unwrap(), dims);

        for _ in 0..4 {
            let r = ctrl.maybe_resize(dims.0, dims.1);
            assert!(r.is_none(), "same dims should not trigger resize");
        }

        let r_diff = ctrl.maybe_resize(100, 30);
        assert!(r_diff.is_some(), "different dims should trigger");
        assert_eq!(r_diff.unwrap(), (100, 30));
    }

    #[test]
    fn test_compute_dims_from_bounds() {
        let (cols, rows) = ResizeController::compute_dims_from_bounds(1200., 800., true, 280.);
        // term_w = 1200 - 280 = 920, term_h = 800 - 120 = 680
        assert_eq!(cols, 115, "920/8 = 115");
        assert_eq!(rows, 34, "680/20 = 34");
    }

    #[test]
    fn test_resize_float_tolerance() {
        // 639.9 vs 640.0 both round to same cols after compute_dims
        let (cols1, _) = ResizeController::compute_dims_from_bounds(639.9, 400., false, 0.);
        let (cols2, _) = ResizeController::compute_dims_from_bounds(640.0, 400., false, 0.);
        assert_eq!(cols1, cols2, "float jitter should produce same cols");
    }
}
