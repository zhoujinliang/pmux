# Change: Optimize Terminal Rendering Performance

## Why

Current terminal rendering in `terminal_view.rs` creates a separate GPUI `AnyElement` for every character cell. For an 80x24 terminal at 60fps, this generates ~115,000 element creations per second, causing high CPU usage and potential frame drops. This is especially problematic when:
- Multiple panes are visible simultaneously
- Terminal content updates rapidly (scrolling, animations)
- Running on lower-end hardware

## What Changes

- **Batch row rendering**: Merge consecutive cells with identical styling into single TextElement segments
- **Viewport culling**: Only render visible rows, skip scrollback lines outside viewport
- **Row-level caching**: Cache rendered rows (string + style hash) to avoid rebuilding elements for unchanged content
- **Performance measurements**: Add benchmarks to verify improvements

## Impact

- Affected specs: terminal-rendering (new)
- Affected code:
  - `src/ui/terminal_view.rs` - core rendering logic
  - `src/terminal/engine.rs` - potential cache integration hooks
- Expected performance gain: 2-5x CPU reduction for typical terminal workloads
- No breaking changes to user-facing behavior
