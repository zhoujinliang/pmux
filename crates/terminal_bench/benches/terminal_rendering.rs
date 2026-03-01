//! Terminal rendering benchmark - display_iter processing for alacritty_terminal.
//!
//! Standalone crate (no pmux dependency) to avoid gpui_macros SIGBUS when benchmarking.
//! Measures display_iter processing time for 80x24 terminal.
//!
//! Run from crate dir: cargo bench
//! Run from workspace root: cargo bench -p terminal_bench (if in workspace)
//!
//! Note: Legacy style-run batching benchmarks (segment_count, viewport_culling, row_cache)
//! were removed when pmux switched to gpui-terminal.

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
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

criterion_group!(benches, benchmark_terminal_rendering);
criterion_main!(benches);
