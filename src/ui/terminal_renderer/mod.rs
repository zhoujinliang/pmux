//! Terminal renderer: layout_grid, RowCache, ShapedLineCache, RenderableGrid.
//! Produces RenderableGrid for TerminalElement to paint. TerminalElement ONLY paint.

mod batched_text_run;
mod build_frame;
mod layout_grid;
mod renderable_grid;
mod row_cache;
mod shaped_line_cache;

pub use batched_text_run::BatchedTextRun;
pub use build_frame::build_frame;
pub use layout_grid::{layout_grid, rgb_to_hsla};
pub use renderable_grid::{CursorLayout, LayoutRect, LayoutTextRun, RenderableGrid};
pub use row_cache::{hash_row_cells, RowCache};
pub use shaped_line_cache::{CacheKey, ShapedLineCache};
