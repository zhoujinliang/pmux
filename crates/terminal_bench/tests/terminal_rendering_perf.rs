//! Terminal rendering performance test - Phase 1 of optimize-terminal-rendering.
//!
//! Verifies 80x24 terminal output can be processed within performance bounds.
//! Run: cargo test -p terminal_bench terminal_rendering_perf

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term};
use std::sync::Mutex;
use std::time::Instant;
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

fn create_populated_80x24() -> Mutex<Term<VoidListener>> {
    let size = TermDimensions {
        columns: 80,
        screen_lines: 24,
    };
    let term = Term::new(Config::default(), &size, VoidListener);
    let mut content = String::with_capacity(80 * 24);
    for row in 0..24 {
        if row == 0 {
            content.push_str("user@host ~ $ ");
            content.push_str(&" ".repeat(66));
        } else if row == 1 {
            content.push_str("$ cargo build");
            content.push_str(&" ".repeat(68));
        } else if row == 2 {
            content.push_str("   Compiling pmux v0.1.0");
            content.push_str(&" ".repeat(56));
        } else {
            content.push_str(&" ".repeat(80));
        }
        if row < 23 {
            content.push('\n');
        }
    }
    let mutex = Mutex::new(term);
    let mut parser = Processor::<StdSyncHandler>::new();
    parser.advance(&mut *mutex.lock().unwrap(), content.as_bytes());
    mutex
}

#[test]
fn test_80x24_realistic_output_cell_count() {
    let term = create_populated_80x24();
    let mut cell_count = 0usize;
    {
        let t = term.lock().unwrap();
        for indexed in t.grid().display_iter() {
            if !indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                cell_count += 1;
            }
        }
    }
    assert_eq!(cell_count, 1920, "80x24 terminal should have 1920 cells");
}

#[test]
fn test_80x24_display_iter_performance() {
    let term = create_populated_80x24();
    let iterations = 1000;
    let start = Instant::now();
    for _ in 0..iterations {
        let t = term.lock().unwrap();
        let mut n = 0usize;
        for indexed in t.grid().display_iter() {
            if !indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                n += 1;
            }
        }
        assert_eq!(n, 1920);
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed.as_secs_f64() / iterations as f64;
    assert!(
        per_iter < 0.001,
        "display_iter should process 80x24 in <1ms per frame, got {:.3}ms",
        per_iter * 1000.0
    );
}
