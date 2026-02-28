# Terminal Rendering Baseline Metrics

Phase 1 benchmark results (from `cargo bench -p terminal_bench`).

## Element Count (80×24 terminal)

### Baseline (per-cell rendering)
| Metric | Value |
|--------|-------|
| Cells per frame | 1,920 |
| Row divs | 24 |
| **Total elements/frame** | **1,944** |

At 60fps: ~116,640 element creations per second.

### Phase 2 (style-run batching)
| Metric | Value |
|--------|-------|
| Segments per frame | 36 |
| Row divs | 24 |
| **Total elements/frame** | **60** |
| **Improvement** | **96.9% reduction** (1,944 → 60) |

Target was 200–400 elements; achieved 60 for typical content (prompt + few output lines).

## Display Iter Processing Time

| Size | Time (µs) |
|------|-----------|
| 80×24 | ~2.3–2.5 |
| 80×48 | ~4.5–5.1 |
| 120×24 | ~3.7–4.4 |

## Segment Count Benchmark

| Benchmark | Time (µs) |
|-----------|-----------|
| segment_count_80x24 | ~22 |

Note: Full GPUI element rendering (with `--features bench`) may hit proc-macro SIGBUS on some macOS setups. The standalone `terminal_bench` crate measures display_iter + segment counting, avoiding GPUI compilation.

## Phase 3: Viewport Culling (80×1000 scrollback)

| Metric | All Rows | Viewport Only (24 lines) |
|--------|----------|--------------------------|
| Segments | ~1000+ | ~36 |
| Rows | 1000 | 24 |
| **Elements/frame** | **~1000+** | **~60** |
| **Improvement** | — | **~94% reduction** with large scrollback |

Run: `cd crates/terminal_bench && cargo bench`

Edge case handling:
- **Empty scrollback**: `scroll_offset >= total_rows` → empty output (saturating_add prevents overflow)
- **Partial viewport**: `scroll_offset + screen_lines > total_rows` → shows rows from scroll_offset to end
- **Cursor at boundaries**: Cursor shown only when its row is in visible range; when scrolled away, cursor hidden (standard behavior)
- **Partial rows**: Full-row filtering (rows fully in or out; no partial row rendering)

## Phase 4: Row-Level Caching (80×1000 scrollback)

| Benchmark | Description |
|-----------|-------------|
| scrolling_80x1000_no_cache | Process 50 scroll steps, recompute segments every time |
| scrolling_80x1000_with_cache | Process 50 scroll steps with LRU cache (200 rows) |

**Cache behavior:**
- Key: content hash (text + fg + bg + flags per segment)
- Value: `Vec<StyledSegment>` (avoids cloning GPUI elements)
- Cursor rows: not cached (cursor position changes frequently)
- Config: `terminal_row_cache_size` in config.json (default 200)

**Expected cache hit rate:** High when scrolling through repetitive content (e.g. many blank or similar rows). Typical terminal output has repeated patterns (prompts, blank lines).

**Run:** `cd crates/terminal_bench && cargo bench -- scrolling`

Note: terminal_bench may fail to build if gpui dependencies have version conflicts. Main pmux build uses stable toolchain.

## Final Results (Phases 1–5 complete)

| Metric | Baseline | Optimized | Improvement |
|--------|----------|-----------|--------------|
| Elements/frame (80×24) | 1,944 | 60 | **96.9% reduction** |
| Segments per frame | N/A | ~36 | — |
| Row divs | 24 | 24 | — |
| Viewport culling (80×1000) | ~1000+ elements | ~60 elements | ~94% reduction with scrollback |

Style-run batching + viewport culling + row-level caching deliver the target performance. See `src/ui/terminal_rendering.rs` for pipeline documentation.
