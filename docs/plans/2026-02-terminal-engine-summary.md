# Terminal Engine Implementation Plans

**Status:** Implementation COMPLETED 2026-02-28. See `2026-02-terminal-engine-COMPLETED.md`.

## Overview

This directory contains the implementation plans for the pmux Terminal Engine - a complete rewrite of the terminal handling layer to achieve native-grade performance and TUI compatibility.

## Problem Statement

Current pmux terminal handling has several issues:
- **High CPU usage** - polling-based updates cause excessive redraws
- **Cursor drift** - incorrect position calculation in TUI apps
- **Input lag** - async hops in input path add latency
- **Resize bugs** - improper SIGWINCH sequence causes misalignment
- **Scrollback issues** - `visible_lines()` doesn't include history

## Solution Architecture

Based on Ghostty/Alacritty/Zed architecture:

```
PTY Reader Thread (blocking) → Byte Channel → Frame Loop (60fps) → alacritty_terminal → Render
```

Key principles:
1. **Terminal = Byte Stream** (not text)
2. **Never parse terminal text**
3. **UI doesn't directly consume PTY**
4. **Frame-based rendering** (not event-driven)
5. **alacritty_terminal is the single state machine**

## Implementation Phases

### Phase 1: Core Architecture
**File:** `2026-02-terminal-engine-phase1-core.md`

**Goal:** Establish foundational structures

**Tasks:**
- Create `TerminalEngine` struct (Term + Processor)
- Implement PTY reader thread (65536 byte buffer, blocking I/O)
- Refactor `LocalPtyRuntime` to use engine
- Add `renderable_content()` API to TermBridge

**Deliverable:** TerminalEngine processes PTY bytes correctly

---

### Phase 2: Frame Loop & Rendering
**File:** `2026-02-terminal-engine-phase2-frame-loop.md`

**Goal:** Implement 60fps frame loop and migrate rendering

**Tasks:**
- Add frame tick to AppRoot (16ms interval)
- Migrate TerminalView to use `renderable_content()`
- Remove `visible_lines()` usage
- Fix cursor rendering from renderable_content
- Remove polling-based content updates

**Deliverable:** 60fps rendering, no per-byte updates

---

### Phase 3: Input & Resize
**File:** `2026-02-terminal-engine-phase3-input-resize.md`

**Goal:** Direct PTY write and correct resize handling

**Tasks:**
- Create `PtyWriter` for direct writes
- Update input handler (no async)
- Implement resize sequence: winsize → SIGWINCH → engine.resize
- Handle window resize events

**Deliverable:** <1ms input latency, correct TUI resize

---

### Phase 4: Integration & Polish
**File:** `2026-02-terminal-engine-phase4-integration.md`

**Goal:** Complete migration and verification

**Tasks:**
- Migrate all pane creation to TerminalEngine
- Remove deprecated `visible_lines()` usage
- Clean up old polling code
- TUI application test suite (vim, Claude Code, lazygit)
- Performance benchmarks
- Documentation updates

**Deliverable:** Production-ready terminal engine

## Success Criteria

| Criteria | Before | After |
|----------|--------|-------|
| CPU (idle) | High | < 5% |
| CPU (TUI) | High | < 10% |
| Input latency | ~50ms | < 1ms |
| Updates/sec | 1000s | 60 |
| Cursor accuracy | Offset | Exact |
| Scrollback | Broken | Working |

## Relationship to Other Plans

This Terminal Engine replaces the terminal handling portions of:
- `2026-02-runtime-phase1-streaming-terminal.md` - Superseded
- `2026-02-runtime-phase4-input-rewrite.md` - Superseded

The Terminal Engine is a prerequisite for:
- Full TUI application support
- Performance optimization
- Multi-pane stability

## Key Design Decisions

### Why Blocking PTY Reader?
PTY output comes in bursts (thousands of bytes at once). Blocking reads:
- Minimize syscalls
- Avoid async runtime overhead
- Provide lowest latency

### Why Frame Loop?
PTY can generate 100,000+ events/second during heavy output.
Rendering each would overwhelm the UI.
Batching to 60fps:
- Reduces CPU by 1000x
- Still perceptually instant
- Matches display refresh rate

### Why alacritty_terminal?
- Battle-tested VT parser
- Correct alternate screen handling
- Proper scrollback management
- Industry standard (Ghostty, Zed use it)

## Getting Started

1. Read `2026-02-terminal-engine-phase1-core.md`
2. Implement Phase 1 tasks
3. Verify with `cargo test terminal::`
4. Proceed to Phase 2

## Notes

- These plans assume familiarity with `alacritty_terminal` crate
- PTY handling uses `portable-pty` and `libc`
- Frame loop integrates with GPUI's rendering
- All phases must be completed for full TUI support

## Questions?

See the original architecture document:
`writing-plans` (user-provided reference implementation)
