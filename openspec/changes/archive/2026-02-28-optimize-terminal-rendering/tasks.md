## 1. Setup & Benchmarking
- [x] 1.1 Create rendering benchmark (frames/sec, element count)
- [x] 1.2 Establish baseline metrics with current implementation
- [x] 1.3 Add performance test with 80x24 terminal output

## 2. Style-Run Batching
- [x] 2.1 Implement `StyledSegment` struct with (text, fg, bg, flags)
- [x] 2.2 Add `group_cells_into_segments()` to merge consecutive same-style cells
- [x] 2.3 Create `render_batch_row()` using segments
- [x] 2.4 Replace per-cell rendering in `render_from_display_iter()`
- [x] 2.5 Verify visual fidelity (colors, bold, underline)
- [x] 2.6 Benchmark and document improvement

## 3. Viewport Culling
- [x] 3.1 Calculate visible row range from scroll_offset and terminal dimensions
- [x] 3.2 Filter display_iter to only visible rows
- [x] 3.3 Handle edge cases (partial rows, cursor at boundaries)
- [x] 3.4 Benchmark with large scrollback (10,000+ lines)

## 4. Row-Level Caching
- [x] 4.1 Add content hash function for grid rows
- [x] 4.2 Implement row cache with LRU eviction
- [x] 4.3 Integrate cache into `render_from_display_iter()`
- [x] 4.4 Set cache size limit (configurable, default 200 rows)
- [x] 4.5 Benchmark scrolling performance

## 5. Testing & Validation
- [x] 5.1 Test with various terminal content (plain, colored, TUI apps)
- [x] 5.2 Test CJK/wide character rendering
- [x] 5.3 Test cursor visibility and positioning
- [x] 5.4 Verify no regressions in status detection
- [x] 5.5 Run full test suite

## 6. Documentation
- [x] 6.1 Document new rendering pipeline
- [x] 6.2 Update performance numbers in design.md
- [x] 6.3 Add code comments explaining batching logic
