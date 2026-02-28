//! RowCache: LRU cache for row layout (rects + runs). Reuses unchanged rows for scroll perf.

use crate::terminal::TermBridge;
use crate::ui::terminal_renderer::renderable_grid::{LayoutRect, LayoutTextRun};
use alacritty_terminal::grid::Indexed;
use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::color::Colors;
use lru::LruCache;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;

/// Hash a row's cells for cache key. Uses char, fg, bg, flags per cell (resolved to Rgb).
pub fn hash_row_cells<'a, I>(cells: I, colors: &Colors) -> u64
where
    I: Iterator<Item = Indexed<&'a Cell>>,
{
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for indexed in cells {
        if indexed.cell.flags.contains(alacritty_terminal::term::cell::Flags::WIDE_CHAR_SPACER) {
            continue;
        }
        indexed.cell.c.hash(&mut hasher);
        let fg = TermBridge::resolve_color(indexed.cell.fg, colors);
        let bg = TermBridge::resolve_color(indexed.cell.bg, colors);
        fg.r.hash(&mut hasher);
        fg.g.hash(&mut hasher);
        fg.b.hash(&mut hasher);
        bg.r.hash(&mut hasher);
        bg.g.hash(&mut hasher);
        bg.b.hash(&mut hasher);
        indexed.cell.flags.bits().hash(&mut hasher);
        if let Some(zw) = indexed.cell.zerowidth() {
            for c in zw.iter() {
                c.hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

/// LRU cache for row layout: hash -> (Vec<LayoutRect>, Vec<LayoutTextRun>).
pub struct RowCache {
    cache: LruCache<u64, (Vec<LayoutRect>, Vec<LayoutTextRun>)>,
}

impl RowCache {
    pub fn new(cap: usize) -> Self {
        let cap = NonZeroUsize::new(cap.max(1)).unwrap();
        Self {
            cache: LruCache::new(cap),
        }
    }

    /// Get cached (rects, runs) or build via layout_grid. On cache hit, build_fn is not called.
    pub fn get_or_build<F>(&mut self, hash: u64, build_fn: F) -> (Vec<LayoutRect>, Vec<LayoutTextRun>)
    where
        F: FnOnce() -> (Vec<LayoutRect>, Vec<LayoutTextRun>),
    {
        if let Some(entry) = self.cache.get(&hash) {
            return entry.clone();
        }
        let result = build_fn();
        self.cache.put(hash, result.clone());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::terminal_renderer::layout_grid;
    use alacritty_terminal::index::{Column, Line, Point};
    use alacritty_terminal::term::cell::Flags;
    use alacritty_terminal::vte::ansi::Color;

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
    fn test_row_cache_reuses_unchanged_rows() {
        let mut cache = RowCache::new(200);
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
        let hash_a = hash_row_cells(cells.iter().cloned(), &colors);

        let mut build_count = 0u32;
        let (rects1, runs1) = cache.get_or_build(hash_a, || {
            build_count += 1;
            layout_grid(cells.iter().cloned(), &colors)
        });
        let (rects2, runs2) = cache.get_or_build(hash_a, || {
            build_count += 1;
            panic!("should not rebuild on cache hit");
        });

        assert_eq!(build_count, 1, "build_fn should run only once");
        assert_eq!(runs1.len(), runs2.len());
        assert_eq!(runs1[0].text, "hello");
        assert_eq!(rects1.len(), rects2.len());
    }
}
