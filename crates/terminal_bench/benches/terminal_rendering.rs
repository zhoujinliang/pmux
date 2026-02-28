//! Terminal rendering benchmark - Phase 1-4 of optimize-terminal-rendering.
//!
//! Standalone crate to avoid gpui_macros SIGBUS when benchmarking.
//! Measures cell count and display_iter processing time for 80x24 terminal.
//! Baseline: 80x24 = 1,920 cells + 24 row divs = 1,944 elements/frame.
//!
//! Run from crate dir: cargo bench
//! Run from workspace root: cargo bench -p terminal_bench (if in workspace)

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use vte::ansi::{Processor, StdSyncHandler};

struct TermDimensions {
    columns: usize,
    screen_lines: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize { self.screen_lines }
    fn screen_lines(&self) -> usize { self.screen_lines }
    fn columns(&self) -> usize { self.columns }
}

/// Dimensions with scrollback: total_lines > screen_lines for large history.
struct ScrollbackDimensions {
    columns: usize,
    screen_lines: usize,
    total_lines: usize,
}

impl Dimensions for ScrollbackDimensions {
    fn total_lines(&self) -> usize { self.total_lines }
    fn screen_lines(&self) -> usize { self.screen_lines }
    fn columns(&self) -> usize { self.columns }
}

fn create_populated_term(cols: usize, rows: usize) -> Mutex<Term<VoidListener>> {
    let size = TermDimensions { columns: cols, screen_lines: rows };
    let term = Term::new(Config::default(), &size, VoidListener);
    let mut content = String::with_capacity(cols * rows);
    for row in 0..rows {
        if row == 0 {
            content.push_str("user@host ~ $ ");
            content.push_str(&" ".repeat(cols.saturating_sub(14)));
        } else if row == 1 {
            content.push_str("$ cargo build");
            content.push_str(&" ".repeat(cols.saturating_sub(12)));
        } else if row == 2 {
            content.push_str("   Compiling pmux v0.1.0");
            content.push_str(&" ".repeat(cols.saturating_sub(24)));
        } else {
            content.push_str(&" ".repeat(cols));
        }
        if row < rows - 1 {
            content.push('\n');
        }
    }
    let mutex = Mutex::new(term);
    let mut parser = Processor::<StdSyncHandler>::new();
    parser.advance(&mut *mutex.lock().unwrap(), content.as_bytes());
    mutex
}

/// Create term with large scrollback (e.g. 80 cols, 24 visible, 1000 total).
fn create_populated_term_with_scrollback(
    cols: usize,
    screen_lines: usize,
    total_lines: usize,
) -> Mutex<Term<VoidListener>> {
    let size = ScrollbackDimensions {
        columns: cols,
        screen_lines,
        total_lines,
    };
    let term = Term::new(Config::default(), &size, VoidListener);
    let mut content = String::with_capacity(cols * total_lines);
    for row in 0..total_lines {
        if row == 0 {
            content.push_str("user@host ~ $ ");
            content.push_str(&" ".repeat(cols.saturating_sub(14)));
        } else if row == 1 {
            content.push_str("$ cargo build");
            content.push_str(&" ".repeat(cols.saturating_sub(12)));
        } else if row == 2 {
            content.push_str("   Compiling pmux v0.1.0");
            content.push_str(&" ".repeat(cols.saturating_sub(24)));
        } else {
            content.push_str(&" ".repeat(cols));
        }
        if row < total_lines - 1 {
            content.push('\n');
        }
    }
    let mutex = Mutex::new(term);
    let mut parser = Processor::<StdSyncHandler>::new();
    parser.advance(&mut *mutex.lock().unwrap(), content.as_bytes());
    mutex
}

fn benchmark_terminal_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("terminal_rendering");

    for size in [(80, 24), (80, 48), (120, 24)].iter() {
        let (cols, rows) = *size;
        let term = create_populated_term(cols, rows);

        group.bench_with_input(
            BenchmarkId::new("display_iter", format!("{}x{}", cols, rows)),
            &term,
            |b, t| {
                b.iter(|| {
                    let term = t.lock().unwrap();
                    let display_iter = term.grid().display_iter();
                    let mut n = 0usize;
                    for indexed in display_iter {
                        if !indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                            n += 1;
                        }
                    }
                    black_box(n)
                })
            },
        );
    }

    group.finish();
}

fn benchmark_element_count(c: &mut Criterion) {
    let term = create_populated_term(80, 24);
    let mut cell_count = 0usize;
    {
        let t = term.lock().unwrap();
        for indexed in t.grid().display_iter() {
            if !indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                cell_count += 1;
            }
        }
    }
    let rows = 24;
    let baseline_element_count = cell_count + rows;
    eprintln!(
        "[BASELINE] 80x24 terminal: {} cells, {} elements/frame (cells + row divs)",
        cell_count, baseline_element_count
    );

    let (segment_count, row_count) = {
        let t = term.lock().unwrap();
        let content = t.renderable_content();
        let display_iter = t.grid().display_iter();
        pmux::ui::terminal_rendering::count_segments_for_benchmark(display_iter, content.colors)
    };
    let batched_element_count = segment_count + row_count;
    eprintln!(
        "[PHASE2] 80x24 terminal: {} segments, {} rows, {} elements/frame (batched)",
        segment_count, row_count, batched_element_count
    );
    eprintln!(
        "[IMPROVEMENT] {} -> {} elements ({:.1}% reduction)",
        baseline_element_count,
        batched_element_count,
        (1.0 - batched_element_count as f64 / baseline_element_count as f64) * 100.0
    );

    c.bench_function("element_count_80x24", |b| {
        b.iter(|| {
            let t = term.lock().unwrap();
            let mut n = 0usize;
            for indexed in t.grid().display_iter() {
                if !indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    n += 1;
                }
            }
            black_box((n, n + rows))
        })
    });

    c.bench_function("segment_count_80x24", |b| {
        b.iter(|| {
            let t = term.lock().unwrap();
            let content = t.renderable_content();
            let display_iter = t.grid().display_iter();
            let (seg, rows) = pmux::ui::terminal_rendering::count_segments_for_benchmark(
                display_iter,
                content.colors,
            );
            black_box((seg, rows, seg + rows))
        })
    });
}

/// Phase 3: Viewport culling benchmark with large scrollback (80x1000).
fn benchmark_viewport_culling(c: &mut Criterion) {
    let cols = 80;
    let screen_lines = 24;
    let total_lines = 1000;
    let term = create_populated_term_with_scrollback(cols, screen_lines, total_lines);

    let (all_seg, all_rows) = {
        let t = term.lock().unwrap();
        let content = t.renderable_content();
        let display_iter = t.grid().display_iter();
        pmux::ui::terminal_rendering::count_segments_for_benchmark(
            display_iter,
            content.colors,
        )
    };
    let (vis_seg, vis_rows) = {
        let t = term.lock().unwrap();
        let content = t.renderable_content();
        let display_iter = t.grid().display_iter();
        pmux::ui::terminal_rendering::count_segments_for_benchmark_with_viewport(
            display_iter,
            content.colors,
            0,
            screen_lines,
        )
    };

    eprintln!(
        "[PHASE3] 80x{} scrollback, {} visible: all={} seg+{} rows, viewport={} seg+{} rows",
        total_lines, screen_lines, all_seg, all_rows, vis_seg, vis_rows
    );
    eprintln!(
        "[IMPROVEMENT] {} -> {} elements ({:.1}% reduction with viewport culling)",
        all_seg + all_rows,
        vis_seg + vis_rows,
        (1.0 - (vis_seg + vis_rows) as f64 / (all_seg + all_rows) as f64) * 100.0
    );

    c.bench_function("segment_count_80x1000_all_rows", |b| {
        b.iter(|| {
            let t = term.lock().unwrap();
            let content = t.renderable_content();
            let display_iter = t.grid().display_iter();
            let (seg, rows) = pmux::ui::terminal_rendering::count_segments_for_benchmark(
                display_iter,
                content.colors,
            );
            black_box((seg, rows))
        })
    });

    c.bench_function("segment_count_80x1000_viewport_only", |b| {
        b.iter(|| {
            let t = term.lock().unwrap();
            let content = t.renderable_content();
            let display_iter = t.grid().display_iter();
            let (seg, rows) = pmux::ui::terminal_rendering::count_segments_for_benchmark_with_viewport(
                display_iter,
                content.colors,
                0,
                screen_lines,
            );
            black_box((seg, rows))
        })
    });
}

/// Phase 4: Row-level cache benchmark. Simulates scrolling through large scrollback.
/// Compares processing time with vs without cache; documents cache hit rate.
fn benchmark_row_cache_scrolling(c: &mut Criterion) {
    let cols = 80;
    let screen_lines = 24;
    let total_lines = 1000;
    let term = create_populated_term_with_scrollback(cols, screen_lines, total_lines);
    let scroll_steps = 50; // Simulate 50 scroll steps through scrollback

    // Benchmark: scroll through scrollback WITHOUT cache (recompute every time)
    c.bench_function("scrolling_80x1000_no_cache", |b| {
        b.iter(|| {
            for scroll_offset in (0..total_lines.saturating_sub(screen_lines))
                .step_by((total_lines / scroll_steps).max(1))
            {
                let t = term.lock().unwrap();
                let content = t.renderable_content();
                let display_iter = t.grid().display_iter();
                let _rows = pmux::ui::terminal_rendering::segments_per_visible_row_for_benchmark(
                    display_iter,
                    content.colors,
                    scroll_offset,
                    scroll_offset + screen_lines,
                );
                black_box(&_rows);
            }
        })
    });

    // Benchmark: scroll through scrollback WITH cache
    let mut cache = LruCache::new(NonZeroUsize::new(200).unwrap());
    let mut total_hits = 0usize;
    let mut total_misses = 0usize;

    c.bench_function("scrolling_80x1000_with_cache", |b| {
        b.iter(|| {
            cache.clear();
            let mut hits = 0usize;
            let mut misses = 0usize;
            for scroll_offset in (0..total_lines.saturating_sub(screen_lines))
                .step_by((total_lines / scroll_steps).max(1))
            {
                let t = term.lock().unwrap();
                let content = t.renderable_content();
                let display_iter = t.grid().display_iter();
                let rows = pmux::ui::terminal_rendering::segments_per_visible_row_for_benchmark(
                    display_iter,
                    content.colors,
                    scroll_offset,
                    scroll_offset + screen_lines,
                );
                for segs in &rows {
                    let h = pmux::ui::terminal_rendering::hash_row_content(segs);
                    if cache.get(&h).is_some() {
                        hits += 1;
                    } else {
                        misses += 1;
                        cache.put(h, segs.clone());
                    }
                }
                black_box(&rows);
            }
            total_hits = hits;
            total_misses = misses;
        })
    });

    let total_lookups = total_hits + total_misses;
    let hit_rate = if total_lookups > 0 {
        (total_hits as f64 / total_lookups as f64) * 100.0
    } else {
        0.0
    };

    eprintln!(
        "[PHASE4] 80x{} scrollback, {} visible, {} scroll steps: cache hits={}, misses={}, hit_rate={:.1}%",
        total_lines, screen_lines, scroll_steps, total_hits, total_misses, hit_rate
    );
    eprintln!(
        "[PHASE4] Memory: LRU cache 200 rows, ~{} entries after scrolling",
        cache.len()
    );
}

criterion_group!(
    benches,
    benchmark_terminal_rendering,
    benchmark_element_count,
    benchmark_viewport_culling,
    benchmark_row_cache_scrolling
);
criterion_main!(benches);
