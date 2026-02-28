# Design: Terminal Rendering Optimization

## Context

**Previous** (per-cell, inefficient):
```
alacritty_terminal Grid
  ↓ display_iter() → Iterator<Indexed<Cell>>
  ↓ per-cell: create div() with styling
  ↓ N elements per row → Vec<AnyElement>
  ↓ GPUI render each element separately
```

For 80 columns × 24 rows = 1,920 cells + 24 row divs = **1,944 elements/frame** at 60fps.

**Current** (style-run batching, viewport culling, row cache):
- **Final element count**: 60 vs 1,944 baseline (**96.9% reduction**)
- Viewport culling: with large scrollback (e.g. 80×1000), only visible 24 rows rendered → ~60 elements vs ~1000+ without culling

## Goals

1. Reduce element creation from O(cells) to O(style-runs)
2. Implement viewport culling for scrollback efficiency
3. Add row-level caching to skip rebuilding unchanged rows
4. Maintain visual fidelity (colors, styles, cursor)

## Non-Goals

- GPU shader rendering (out of scope, requires GPUI shader work)
- Font ligature support (separate feature)
- Full terminal backend replacement

## Decisions

### Decision 1: Style-run based batching

**What**: Group consecutive cells with identical (fg, bg, flags) into styled segments.

**Why**: Typical terminal content has long runs of uniform styling (normal text, colored prompts). Batching reduces elements from N cells to ~N/10 style runs.

**Implementation**:
```rust
struct StyledSegment {
    text: String,
    fg: RgbColor,
    bg: RgbColor,
    flags: Flags,
}

// One element per segment, not per cell
fn render_row(segments: Vec<StyledSegment>) -> AnyElement {
    // Single flex row with styled text spans
}
```

**Alternative considered**: GPU texture atlas - too complex for this phase.

### Decision 2: Viewport culling via alacritty's display_iter

**What**: Use `grid.display_iter()` with visible bounds filtering.

**Why**: alacritty_terminal already manages scrollback; we just need to skip rendering off-screen rows. `display_iter()` gives us line numbers to filter.

### Decision 3: Row cache keyed by content hash

**What**: Cache `Vec<AnyElement>` per row, keyed by (row_content_hash, style_hash).

**Why**: Terminal content changes incrementally (new lines at bottom, scrolling). Most rows remain identical between frames.

**Invalidation**: Any change to row content or scroll position invalidates that row's cache.

### Decision 4: GPUI TextRun or custom Element

**What**: Evaluate both approaches:
- **TextRun**: GPUI's built-in multi-style text, may have limitations
- **Custom Element**: Full control, more code

**Decision**: Start with custom Element using `div().children()` with styled text spans. If GPUI TextRun supports our needs, migrate later.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Wide characters (CJK) break layout | Test with CJK content, use proper width calculation |
| Cursor positioning becomes complex | Keep cursor as separate overlay element |
| Cache memory growth | Limit cache to visible rows + small buffer |
| Style-run detection overhead | Benchmark vs per-cell; expect net win |

## Implementation Plan

### Phase 1: Style-run batching
1. Implement `StyledSegment` grouping in `render_from_display_iter()`
2. Create `render_batch_row()` to produce single element per row
3. Compare benchmarks with original

### Phase 2: Viewport culling
1. Calculate visible row range from scroll_offset
2. Filter `display_iter()` to only visible rows
3. Measure improvement with large scrollback

### Phase 3: Row caching
1. Add `LruCache<(row_idx, content_hash), Vec<AnyElement>>`
2. Hash row content for cache key
3. Benchmark scrolling performance

## Open Questions

1. Should we use GPUI's `TextRun` or custom `Element`? (Spike: 1-2 hours)
2. How to handle double-width characters efficiently?
3. Cache size limit - what LRU bound works for typical usage?
