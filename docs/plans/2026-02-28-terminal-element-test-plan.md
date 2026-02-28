# TerminalElement 重构测试方案

> **Goal:** 系统化验证 TerminalElement 重构的正确性、性能、兼容性。利用 macOS 自动化能力。

---

## 测试架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Test Orchestrator                         │
│                  (tests/terminal_e2e/)                       │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │ Unit     │  │Integration│  │ Visual   │  │Performance│    │
│  │ Tests    │  │ Tests     │  │ Tests    │  │ Tests     │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │
├─────────────────────────────────────────────────────────────┤
│                    macOS Automation Layer                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ AppleScript  │  │ screencapture│  │ log stream   │       │
│  │ app control  │  │ screenshot   │  │ system logs  │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ ffmpeg       │  │ Instruments  │  │ sample       │       │
│  │ video record │  │ profiling    │  │ CPU sampling │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└─────────────────────────────────────────────────────────────┘
```

---

## Part 1: Unit Tests (Rust native)

### 1.1 Core Data Structures

| File | Test | Description |
|------|------|-------------|
| `terminal_renderer/layout_grid.rs` | `test_layout_grid_empty` | 空输入返回空输出 |
| `terminal_renderer/layout_grid.rs` | `test_layout_grid_merges_cells` | 同 style cells 合并为 1 run |
| `terminal_renderer/layout_grid.rs` | `test_layout_grid_zerowidth_append` | zerowidth 附加到前 run，继承 cluster index |
| `terminal_renderer/layout_grid.rs` | `test_layout_grid_zerowidth_cluster_index` | emoji skin-tone cursor 不在中间 |
| `terminal_renderer/row_cache.rs` | `test_row_cache_hit` | hash 命中返回缓存 |
| `terminal_renderer/row_cache.rs` | `test_row_cache_miss` | hash 未命中调用 builder |
| `terminal_renderer/shaped_line_cache.rs` | `test_shaped_line_cache_hit` | 缓存命中跳过 shape_line |
| `terminal_renderer/shaped_line_cache.rs` | `test_shaped_line_cache_invalidation` | DPI/font 变化清空缓存 |

### 1.2 Cursor

| File | Test | Description |
|------|------|-------------|
| `terminal_element.rs` | `test_display_cursor_from_point` | AlacPoint → DisplayCursor |
| `terminal_element.rs` | `test_cursor_position_pixel_coords` | DisplayCursor → pixel coords |
| `terminal_element.rs` | `test_cursor_width_wide_char` | emoji cursor_width = max(shaped, cell) |
| `terminal_element.rs` | `test_cursor_width_whitespace` | whitespace cursor_width = cell_width |
| `terminal_element.rs` | `test_cursor_width_grapheme_fallback` | shaped_width == 0 时用 cell_width |
| `terminal_element.rs` | `test_cursor_shape_block` | Block shape 绘制 |
| `terminal_element.rs` | `test_cursor_shape_bar` | Bar/Beam shape 绘制 |
| `terminal_element.rs` | `test_cursor_shape_underline` | Underline shape 绘制 |
| `terminal_element.rs` | `test_cursor_shape_hollow` | Hollow shape 绘制 |
| `terminal_element.rs` | `test_cursor_visibility_dectcem` | `\x1b[?25l` hide, `\x1b[?25h` show |

### 1.3 Background Regions

| File | Test | Description |
|------|------|-------------|
| `terminal_renderer/layout_grid.rs` | `test_background_region_merge_same_line` | 同行同色相邻合并 |
| `terminal_renderer/layout_grid.rs` | `test_background_region_no_cross_wrap` | 禁止跨 wrap 行合并 |

### 1.4 Viewport Culling

| File | Test | Description |
|------|------|-------------|
| `terminal_element.rs` | `test_viewport_culling_empty_intersection` | 完全不可见返回空 |
| `terminal_element.rs` | `test_viewport_culling_partial_rows` | 部分行可见 |
| `terminal_element.rs` | `test_viewport_culling_horizontal` | 宽终端水平裁剪 |

### 1.5 RenderableGrid & Snapshot

| File | Test | Description |
|------|------|-------------|
| `terminal/renderable_snapshot.rs` | `test_snapshot_from_engine` | 单次 lock 内构建 |
| `terminal/renderable_grid.rs` | `test_renderable_grid_empty` | empty() 返回有效空 grid |

---

## Part 2: Integration Tests (Rust + mock PTY)

### 2.1 TerminalEngine Integration

```rust
// tests/terminal_engine_integration.rs

#[test]
fn test_engine_resize_updates_grid() {
    let (tx, rx) = flume::unbounded();
    let engine = TerminalEngine::new(80, 24, rx);
    engine.resize(100, 30);
    let term = engine.terminal();
    assert_eq!(term.grid().columns(), 100);
    assert_eq!(term.grid().screen_lines(), 30);
}

#[test]
fn test_concurrent_advance_and_render() {
    // 模拟 PTY 持续输出 + render 线程读取
    let engine = Arc::new(TerminalEngine::new(80, 24, rx));
    
    // Writer thread
    let writer = {
        let engine = engine.clone();
        thread::spawn(move || {
            for i in 0..1000 {
                tx.send(format!("line {}\n", i).into_bytes()).unwrap();
                engine.advance_bytes();
                thread::sleep(Duration::from_micros(100));
            }
        })
    };
    
    // Reader thread
    let reader = {
        let engine = engine.clone();
        thread::spawn(move || {
            for _ in 0..100 {
                engine.try_renderable_content(|_, _, _| {});
                thread::sleep(Duration::from_millis(1));
            }
        })
    };
    
    writer.join().unwrap();
    reader.join().unwrap();
    // 无 deadlock，无 panic
}
```

### 2.2 Resize + Heavy Output

```rust
// tests/terminal_resize_stress.rs

#[test]
fn test_resize_during_heavy_output() {
    // 1. 启动 engine + PTY mock
    // 2. 快速 resize（模拟拖拽）
    // 3. 同时大量输出
    // 4. 验证：无 panic、最终内容正确
}

#[test]
fn test_pty_flood_gpu_stall() {
    // 1. PTY 快速输出（yes 模拟）
    // 2. 模拟 GPU stall（短暂 block paint）
    // 3. 验证：无 backlog 导致内容错乱
}
```

### 2.3 TUI Cursor (vim, Claude, OpenCode)

```rust
// tests/terminal_tui_cursor.rs

#[test]
fn test_vim_cursor_shape_change() {
    let engine = make_engine();
    // Normal mode: block
    send_bytes(b"\x1b[2 q");
    engine.advance_bytes();
    assert_eq!(get_cursor_shape(&engine), CursorShape::Block);
    
    // Insert mode: bar
    send_bytes(b"\x1b[5 q");
    engine.advance_bytes();
    assert_eq!(get_cursor_shape(&engine), CursorShape::Bar);
}

#[test]
fn test_alternate_screen_cursor() {
    let engine = make_engine();
    // Enter alternate screen (vim)
    send_bytes(b"\x1b[?1049h");
    engine.advance_bytes();
    assert!(engine.is_tui_active());
    // Cursor 应跟随 engine.cursor
}
```

---

## Part 3: Visual Regression Tests (macOS automation)

### 3.1 Screenshot Comparison Framework

```bash
# tests/visual/screenshot_test.sh

#!/bin/bash
set -e

PMUX_APP="/Users/matt.chow/workspace/pmux/target/debug/pmux"
SCREENSHOT_DIR="/tmp/pmux_screenshots"
BASELINE_DIR="tests/visual/baselines"
DIFF_DIR="/tmp/pmux_diffs"

mkdir -p "$SCREENSHOT_DIR" "$DIFF_DIR"

# 1. 启动 pmux
open -a "$PMUX_APP"
sleep 2

# 2. 等待终端渲染
sleep 1

# 3. 截图
screencapture -x "$SCREENSHOT_DIR/current.png"

# 4. 关闭 pmux
osascript -e 'tell application "pmux" to quit'

# 5. 对比
if [ -f "$BASELINE_DIR/terminal_basic.png" ]; then
    # 使用 ImageMagick 对比
    compare -metric AE "$BASELINE_DIR/terminal_basic.png" "$SCREENSHOT_DIR/current.png" "$DIFF_DIR/diff.png" 2>/dev/null || {
        echo "FAIL: Visual regression detected"
        exit 1
    }
    echo "PASS: Visual match"
else
    echo "INFO: No baseline, saving current as baseline"
    cp "$SCREENSHOT_DIR/current.png" "$BASELINE_DIR/terminal_basic.png"
fi
```

### 3.2 AppleScript App Control

```applescript
-- tests/visual/app_control.applescript

on run argv
    set appPath to item 1 of argv
    set action to item 2 of argv
    
    tell application "Finder"
        open POSIX file appPath
    end tell
    
    delay 2
    
    if action is "type_text" then
        set textToType to item 3 of argv
        tell application "System Events"
            keystroke textToType
        end tell
    else if action is "press_key" then
        set keyCode to item 3 of argv
        tell application "System Events"
            key code keyCode
        end tell
    else if action is "resize_window" then
        set newWidth to item 3 of argv as integer
        set newHeight to item 4 of argv as integer
        tell application "pmux"
            set bounds of front window to {100, 100, 100 + newWidth, 100 + newHeight}
        end tell
    end if
end run
```

### 3.3 Visual Test Cases

| Test ID | Description | Setup | Verification |
|---------|-------------|-------|--------------|
| V001 | Basic text rendering | 启动 pmux，显示 `$ ` prompt | 截图对比 baseline |
| V002 | ANSI colors | `echo -e "\e[31mred\e[0m \e[32mgreen\e[0m"` | 截图验证颜色正确 |
| V003 | Bold/underline | `echo -e "\e[1mbold\e[0m \e[4munderline\e[0m"` | 截图验证样式 |
| V004 | Cursor position | 空终端，光标在左上 | 截图验证光标位置 |
| V005 | Cursor shape - block | vim normal mode | 截图验证 block 光标 |
| V006 | Cursor shape - bar | vim insert mode | 截图验证 bar 光标 |
| V007 | Cursor visibility | `\x1b[?25l` 隐藏光标 | 截图验证无光标 |
| V008 | Resize terminal | 拖拽窗口 resize | 截图验证内容正确重排 |
| V009 | Wide chars (emoji) | `echo "🎉 hello 👋"` | 截图验证 emoji 正确渲染 |
| V010 | Scrollback | 大量输出后滚动 | 截图验证滚动正确 |
| V011 | Powerline chars | Powerline 字体提示符 | 截图验证分隔符正确 |

---

## Part 4: Screen Recording Tests

### 4.1 Video Recording Framework

```bash
# tests/performance/record_perf.sh

#!/bin/bash

RECORD_DIR="/tmp/pmux_recordings"
mkdir -p "$RECORD_DIR"

# 使用 ffmpeg 录制屏幕
ffmpeg -f avfoundation -i "1" -r 30 -t 10 "$RECORD_DIR/test_$(date +%s).mp4" &
FFMPEG_PID=$!

# 运行测试场景
sleep 1

# 场景 1: 快速滚动
osascript -e 'tell application "System Events" to keystroke "c"' 
osascript -e 'tell application "System Events" to keystroke "a"' 
osascript -e 'tell application "System Events" to keystroke "t"' 
osascript -e 'tell application "System Events" to keystroke space' 
osascript -e 'tell application "System Events" to keystroke "/usr/share/dict/words"' 
osascript -e 'tell application "System Events" to key code 36' # Enter

sleep 5

# 停止录制
kill $FFMPEG_PID 2>/dev/null

echo "Recording saved to $RECORD_DIR"
```

### 4.2 Performance Metrics from Video

```python
# tests/performance/analyze_video.py

import cv2
import numpy as np

def count_frames_with_changes(video_path, threshold=30):
    """统计有视觉变化的帧数，估算 FPS"""
    cap = cv2.VideoCapture(video_path)
    prev_frame = None
    change_count = 0
    total_frames = 0
    
    while True:
        ret, frame = cap.read()
        if not ret:
            break
        total_frames += 1
        
        gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
        if prev_frame is not None:
            diff = cv2.absdiff(prev_frame, gray)
            if np.mean(diff) > threshold:
                change_count += 1
        prev_frame = gray
    
    cap.release()
    
    duration = total_frames / 30  # assuming 30fps recording
    fps = change_count / duration if duration > 0 else 0
    
    print(f"Total frames: {total_frames}")
    print(f"Frames with changes: {change_count}")
    print(f"Estimated render FPS: {fps:.1f}")
    
    return fps

if __name__ == "__main__":
    import sys
    video_path = sys.argv[1]
    fps = count_frames_with_changes(video_path)
    
    # Pass/Fail
    if fps >= 30:
        print("PASS: FPS >= 30")
    else:
        print(f"FAIL: FPS {fps:.1f} < 30")
```

---

## Part 5: Log Collection & Analysis

### 5.1 System Log Stream

```bash
# tests/logs/collect_logs.sh

#!/bin/bash

LOG_DIR="/tmp/pmux_logs"
mkdir -p "$LOG_DIR"

# 启动 log stream
log stream --predicate 'process == "pmux"' --style compact > "$LOG_DIR/system.log" &
LOG_PID=$!

# 同时捕获 pmux 内部日志（如果配置了）
tail -f /tmp/pmux.log 2>/dev/null > "$LOG_DIR/app.log" &
TAIL_PID=$!

# 运行测试
# ... test commands ...

# 停止日志采集
sleep 2
kill $LOG_PID $TAIL_PID 2>/dev/null

echo "Logs saved to $LOG_DIR"
```

### 5.2 Performance Logging in Code

```rust
// 在 TerminalElement::paint 中添加性能日志

impl Element for TerminalElement {
    fn paint(&mut self, ... ) {
        let frame_start = std::time::Instant::now();
        
        // ... paint logic ...
        
        let elapsed = frame_start.elapsed();
        if elapsed.as_millis() > 8 {  // > 8ms = < 120fps
            log::warn!(
                target: "terminal_perf",
                "paint took {:?} (frame {})",
                elapsed,
                self.frame_count
            );
        }
    }
}
```

### 5.3 Log Analysis Script

```python
# tests/logs/analyze_logs.py

import re
from collections import defaultdict

def parse_performance_logs(log_path):
    """解析性能日志，输出统计"""
    paint_times = []
    resize_events = []
    cache_hits = 0
    cache_misses = 0
    
    with open(log_path) as f:
        for line in f:
            if 'paint took' in line:
                match = re.search(r'(\d+)ms', line)
                if match:
                    paint_times.append(int(match.group(1)))
            elif 'resize' in line.lower():
                resize_events.append(line.strip())
            elif 'cache hit' in line:
                cache_hits += 1
            elif 'cache miss' in line:
                cache_misses += 1
    
    print("=== Performance Summary ===")
    print(f"Total paints: {len(paint_times)}")
    if paint_times:
        print(f"Average paint time: {sum(paint_times)/len(paint_times):.1f}ms")
        print(f"P95 paint time: {sorted(paint_times)[int(len(paint_times)*0.95)]}ms")
        print(f"Max paint time: {max(paint_times)}ms")
    print(f"Resize events: {len(resize_events)}")
    print(f"Cache hit rate: {cache_hits/(cache_hits+cache_misses)*100:.1f}%" if cache_hits+cache_misses > 0 else "N/A")
    
    # Pass/Fail
    p95 = sorted(paint_times)[int(len(paint_times)*0.95)] if paint_times else 0
    if p95 < 16:
        print("\nPASS: P95 paint time < 16ms")
    else:
        print(f"\nFAIL: P95 paint time {p95}ms >= 16ms")

if __name__ == "__main__":
    import sys
    parse_performance_logs(sys.argv[1])
```

---

## Part 6: CPU/Memory Profiling

### 6.1 Instruments Profiling

```bash
# tests/performance/profile.sh

#!/bin/bash

PMUX_APP="/Users/matt.chow/workspace/pmux/target/debug/pmux"
TRACE_DIR="/tmp/pmux_traces"

mkdir -p "$TRACE_DIR"

# 使用 Instruments 记录 Time Profiler
instruments -t "Time Profiler" -D "$TRACE_DIR/trace.trace" "$PMUX_APP" &
INSTRUMENTS_PID=$!

sleep 5

# 触发性能场景
osascript -e 'tell application "System Events" to keystroke "c"' 
osascript -e 'tell application "System Events" to keystroke "a"' 
osascript -e 'tell application "System Events" to keystroke "t"' 
osascript -e 'tell application "System Events" to keystroke " /usr/share/dict/words"' 
osascript -e 'tell application "System Events" to key code 36' 

sleep 10

# 停止
kill $INSTRUMENTS_PID 2>/dev/null

echo "Trace saved to $TRACE_DIR/trace.trace"
echo "Open with: open $TRACE_DIR/trace.trace"
```

### 6.2 Sample CPU

```bash
# tests/performance/sample_cpu.sh

#!/bin/bash

PMUX_PID=$(pgrep -f "target/debug/pmux")

if [ -z "$PMUX_PID" ]; then
    echo "pmux not running"
    exit 1
fi

# 采样 5 秒
sample $PMUX_PID 5 -f /tmp/pmux_sample.txt

# 分析热点
grep -A 20 "Total number in stack" /tmp/pmux_sample.txt
```

---

## Part 7: Test Runner

### 7.1 Main Test Script

```bash
# tests/run_terminal_tests.sh

#!/bin/bash
set -e

echo "=== TerminalElement Test Suite ==="
echo ""

# Phase 1: Unit tests
echo "Phase 1: Unit Tests..."
cargo test --lib terminal_ -- --nocapture
echo "✓ Unit tests passed"

# Phase 2: Integration tests
echo "Phase 2: Integration Tests..."
cargo test --test terminal_ -- --nocapture
echo "✓ Integration tests passed"

# Phase 3: Visual regression (requires GUI)
if [ -z "$SKIP_VISUAL" ]; then
    echo "Phase 3: Visual Regression Tests..."
    ./tests/visual/run_visual_tests.sh
    echo "✓ Visual tests passed"
fi

# Phase 4: Performance tests
if [ -z "$SKIP_PERF" ]; then
    echo "Phase 4: Performance Tests..."
    ./tests/performance/run_perf_tests.sh
    echo "✓ Performance tests passed"
fi

# Phase 5: E2E tests
if [ -z "$SKIP_E2E" ]; then
    echo "Phase 5: E2E Tests..."
    ./tests/e2e/run_all.sh
    echo "✓ E2E tests passed"
fi

echo ""
echo "=== All Tests Passed ==="
```

### 7.2 CI Integration

```yaml
# .github/workflows/terminal_tests.yml

name: Terminal Tests

on:
  push:
    paths:
      - 'src/terminal/**'
      - 'src/ui/terminal_*/**'
      - 'tests/terminal_*/**'

jobs:
  unit:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --lib terminal_
      
  integration:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --test terminal_
      
  visual:
    runs-on: macos-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
      - run: ./tests/visual/run_visual_tests.sh
      - uses: actions/upload-artifact@v4
        with:
          name: visual-diffs
          path: /tmp/pmux_diffs/
```

---

## Part 8: Test Matrix

### 8.1 Platforms

| Platform | Unit | Integration | Visual | Performance |
|----------|------|-------------|--------|-------------|
| macOS (Intel) | ✓ | ✓ | ✓ | ✓ |
| macOS (Apple Silicon) | ✓ | ✓ | ✓ | ✓ |
| Linux | ✓ | ✓ | - | ✓ |

### 8.2 Scenarios

| Scenario | Test Type | Frequency |
|----------|-----------|-----------|
| Basic render | Visual | Every PR |
| ANSI colors | Visual | Every PR |
| Cursor shapes | Visual + Unit | Every PR |
| Resize | Integration + Visual | Every PR |
| Scroll performance | Performance | Daily |
| Memory leak | Performance | Weekly |
| Concurrent PTY | Integration | Every PR |
| TUI (vim, Claude) | E2E | Every PR |

---

## Part 9: Acceptance Criteria

### 9.1 Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Paint time (P95) | < 16ms | Log analysis |
| FPS (fast scroll) | ≥ 30 | Video analysis |
| Memory growth | < 10MB/hour | Instruments |
| Cache hit rate | > 90% | Log analysis |

### 9.2 Correctness

| Feature | Test | Pass Criteria |
|---------|------|---------------|
| Text rendering | Visual | Screenshot matches baseline |
| ANSI colors | Visual | All 16 colors correct |
| Cursor position | Visual + Unit | Pixel-accurate |
| Cursor shape | Visual | Block/Bar/Underline/Hollow |
| Wide chars | Visual | Emoji rendered correctly |
| Resize | Integration | No content loss/corruption |
| TUI | E2E | vim insert/normal works |

### 9.3 Compatibility

| App | Test | Expected Behavior |
|-----|------|-------------------|
| vim | E2E | Normal: block cursor, Insert: bar cursor |
| neovim | E2E | Same as vim |
| tmux | E2E | Cursor follows pane focus |
| Claude Code | E2E | Terminal cursor visible |
| OpenCode | E2E | Terminal cursor visible |

---

## Appendix: Helper Scripts

### A.1 Screenshot Helper

```bash
#!/bin/bash
# tests/helpers/screenshot.sh

WINDOW_NAME=$1
OUTPUT=$2

# 获取窗口 ID
WINDOW_ID=$(osascript -e "tell application \"Finder\" to get id of window \"$WINDOW_NAME\"" 2>/dev/null)

if [ -n "$WINDOW_ID" ]; then
    screencapture -l $WINDOW_ID "$OUTPUT"
else
    # Fallback: 截取整个屏幕
    screencapture "$OUTPUT"
fi
```

### A.2 Window Control

```bash
#!/bin/bash
# tests/helpers/window_control.sh

ACTION=$1
shift

case $ACTION in
    "resize")
        WIDTH=$1
        HEIGHT=$2
        osascript -e "tell application \"pmux\" to set bounds of front window to {100, 100, $((100+WIDTH)), $((100+HEIGHT))}"
        ;;
    "move")
        X=$1
        Y=$2
        osascript -e "tell application \"pmux\" to set position of front window to {$X, $Y}"
        ;;
    "focus")
        osascript -e "tell application \"pmux\" to activate"
        ;;
    "close")
        osascript -e "tell application \"pmux\" to quit"
        ;;
esac
```

### A.3 Keyboard Input

```bash
#!/bin/bash
# tests/helpers/keyboard.sh

TYPE=$1
shift

case $TYPE in
    "text")
        TEXT=$1
        osascript -e "tell application \"System Events\" to keystroke \"$TEXT\""
        ;;
    "key")
        KEY=$1
        osascript -e "tell application \"System Events\" to key code $KEY"
        ;;
    "combo")
        # e.g., "cmd shift 3"
        MODIFIERS=$1
        KEY=$2
        osascript -e "tell application \"System Events\" to keystroke \"$KEY\" using {$MODIFIERS down}"
        ;;
esac
```

### A.4 Key Codes Reference

| Key | Code |
|-----|------|
| Return | 36 |
| Tab | 48 |
| Space | 49 |
| Delete | 51 |
| Escape | 53 |
| Arrow Left | 123 |
| Arrow Right | 124 |
| Arrow Down | 125 |
| Arrow Up | 126 |
| A | 0 |
| S | 1 |
| D | 2 |
| F | 3 |
| H | 4 |
| G | 5 |
| Z | 6 |
| X | 7 |
| C | 8 |
| V | 9 |
| Q | 12 |
| W | 13 |
| E | 14 |
| R | 15 |
| 0 | 29 |
| 1 | 18 |
| 2 | 19 |
| 3 | 20 |
| 4 | 21 |
| 5 | 23 |
| 6 | 22 |
| 7 | 26 |
| 8 | 28 |
| 9 | 25 |