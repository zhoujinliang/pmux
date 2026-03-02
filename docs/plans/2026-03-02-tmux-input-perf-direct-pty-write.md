# tmux 输入性能优化：Direct PTY Write + Render Batching

> **For Claude:** Use TDD when implementing. Consider `subagent-driven-development` for parallel tasks (Task 1 and Task 2 are independent).

**Goal:** 消除 tmux control mode 下的输入延迟，使打字/方向键体验接近 Ghostty。
**Architecture:** 输入路径绕过 `send-keys` 命令，直接写入 pane 的 TTY 设备 (`#{pane_tty}`)；渲染路径使用已有的 `BatchedTextRun` 替代逐字符 `shape_line`。
**Tech Stack:** Rust, GPUI, tmux, libc (PTY)

---

## 背景

当前 tmux-cc 后端每个按键都通过 `send-keys -l -t %0 'x'` 命令发送，需要：
- Mutex lock + String format + PTY write + flush
- tmux 解析命令 → 查找 pane → 注入 shell PTY

每次 ~0.5-2ms，快速打字时延迟累积明显。同时渲染器虽然有 `BatchedTextRun` 基础设施，但 paint() 未使用，仍逐字符调用 `shape_line()`。

**对比 Ghostty:** 直接 `write(fd, bytes, len)` 到 PTY master，0 中间层。

---

## Task 1: Direct PTY Write — 输入直写 pane TTY

**Files:**
- Modify: `src/runtime/backends/tmux_control_mode.rs`

### Step 1: 添加 pane TTY 缓存到 struct

在 `TmuxControlModeRuntime` 结构体中添加 per-pane TTY fd 缓存：

```rust
pub struct TmuxControlModeRuntime {
    session_name: String,
    window_name: Mutex<String>,
    pty_writer: Arc<Mutex<std::fs::File>>,
    pane_outputs: Arc<Mutex<HashMap<String, flume::Sender<Vec<u8>>>>>,
    _control_child: Arc<Mutex<Child>>,
    /// Cached file handles to pane TTY devices for direct write (bypass send-keys)
    pane_tty_writers: Arc<Mutex<HashMap<String, std::fs::File>>>,
}
```

在 `new()` 末尾初始化：

```rust
let rt = Self {
    session_name: session_name.to_string(),
    window_name: Mutex::new(window_name.to_string()),
    pty_writer: Arc::new(Mutex::new(master_writer)),
    pane_outputs,
    _control_child: Arc::new(Mutex::new(child)),
    pane_tty_writers: Arc::new(Mutex::new(HashMap::new())),
};
```

### Step 2: 添加 `resolve_pane_tty()` 方法

在 `impl TmuxControlModeRuntime` 块中添加：

```rust
/// Resolve the TTY device path for a tmux pane (e.g. /dev/ttys042).
/// Result is cached in pane_tty_writers for subsequent calls.
fn resolve_pane_tty(&self, pane_id: &str) -> Option<std::fs::File> {
    // Check cache first
    if let Ok(cache) = self.pane_tty_writers.lock() {
        if cache.contains_key(pane_id) {
            return None; // Signal caller to use cached writer directly
        }
    }

    // Query tmux for the pane's TTY path
    let output = Command::new("tmux")
        .args(["display-message", "-t", pane_id, "-p", "#{pane_tty}"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let tty_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if tty_path.is_empty() || !tty_path.starts_with("/dev/") {
        return None;
    }

    // Open the TTY device for writing
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&tty_path)
        .ok()?;

    Some(file)
}
```

### Step 3: 添加 `direct_write()` 方法

```rust
/// Write bytes directly to a pane's TTY device, bypassing tmux send-keys.
/// Returns Ok(true) if written directly, Ok(false) if cache miss (caller should resolve).
fn direct_write(&self, pane_id: &str, bytes: &[u8]) -> Result<bool, RuntimeError> {
    let mut cache = self.pane_tty_writers.lock()
        .map_err(|e| RuntimeError::Backend(format!("tty cache lock: {}", e)))?;

    if let Some(writer) = cache.get_mut(pane_id) {
        writer.write_all(bytes)
            .map_err(|e| {
                // TTY gone (pane killed?) — remove from cache, caller will fallback
                cache.remove(pane_id);
                RuntimeError::Backend(format!("tty write: {}", e))
            })?;
        writer.flush()
            .map_err(|e| RuntimeError::Backend(format!("tty flush: {}", e)))?;
        return Ok(true);
    }

    Ok(false) // Not cached yet
}
```

### Step 4: 重写 `send_input()` 使用 direct write

替换现有的 `send_input` 实现：

```rust
fn send_input(&self, pane_id: &PaneId, bytes: &[u8]) -> Result<(), RuntimeError> {
    if bytes.is_empty() {
        return Ok(());
    }

    // Fast path: direct write to pane TTY (bypasses tmux send-keys entirely)
    match self.direct_write(pane_id, bytes) {
        Ok(true) => return Ok(()),  // Written directly
        Ok(false) => {
            // Cache miss — resolve TTY and try again
            if let Some(file) = self.resolve_pane_tty(pane_id) {
                if let Ok(mut cache) = self.pane_tty_writers.lock() {
                    cache.insert(pane_id.clone(), file);
                }
                // Retry with cache populated
                if let Ok(true) = self.direct_write(pane_id, bytes) {
                    return Ok(());
                }
            }
            // Fallback to send-keys if TTY resolution fails
        }
        Err(_) => {
            // Direct write failed (pane gone?) — fallback to send-keys
        }
    }

    // Fallback: use send-keys (original path, slow but always works)
    self.send_input_via_send_keys(pane_id, bytes)
}
```

### Step 5: 重命名原 send_input 为 fallback

将当前的 `send_input` 方法体移到一个新的私有方法 `send_input_via_send_keys`：

```rust
/// Fallback: send input via tmux send-keys command (slower but works when direct PTY write is unavailable).
fn send_input_via_send_keys(&self, pane_id: &PaneId, bytes: &[u8]) -> Result<(), RuntimeError> {
    // ... (current send_input body, unchanged)
}
```

这个方法放在 `impl TmuxControlModeRuntime` 块中（非 trait impl）。然后在 `AgentRuntime` trait impl 的 `send_input` 中调用新逻辑。

### Step 6: switch_window 时清除 TTY 缓存

在 `switch_window()` 中，除了清除 `pane_outputs`，也清除 TTY 缓存：

```rust
fn switch_window(&self, window_name: &str, start_dir: Option<&Path>) -> Result<(), RuntimeError> {
    // ... (existing logic unchanged)

    if let Ok(mut map) = self.pane_outputs.lock() {
        map.clear();
    }

    // Clear cached TTY writers since pane IDs change with window switch
    if let Ok(mut cache) = self.pane_tty_writers.lock() {
        cache.clear();
    }

    Ok(())
}
```

### Step 7: 写测试

在 `#[cfg(test)] mod tests` 中添加：

```rust
#[test]
fn test_resolve_pane_tty_returns_path() {
    if !crate::runtime::backends::tmux_available() {
        eprintln!("skipping: tmux not available");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let rt = TmuxControlModeRuntime::new("pmux-test-tty", "main", Some(dir.path()))
        .expect("should create runtime");

    let panes = rt.list_panes(&String::new());
    let pane_id = panes.first().cloned().unwrap_or_else(|| "%0".to_string());

    // resolve_pane_tty should return a writable file
    let file = rt.resolve_pane_tty(&pane_id);
    assert!(file.is_some(), "should resolve pane TTY for {}", pane_id);

    // Cleanup
    let _ = Command::new("tmux").args(["kill-session", "-t", "pmux-test-tty"]).output();
}

#[test]
fn test_direct_write_caches_and_writes() {
    if !crate::runtime::backends::tmux_available() {
        eprintln!("skipping: tmux not available");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let rt = TmuxControlModeRuntime::new("pmux-test-dw", "main", Some(dir.path()))
        .expect("should create runtime");

    let panes = rt.list_panes(&String::new());
    let pane_id = panes.first().cloned().unwrap_or_else(|| "%0".to_string());

    // First call: cache miss
    let result = rt.direct_write(&pane_id, b"hello");
    assert!(matches!(result, Ok(false)), "first call should be cache miss");

    // Resolve and populate cache
    if let Some(file) = rt.resolve_pane_tty(&pane_id) {
        rt.pane_tty_writers.lock().unwrap().insert(pane_id.clone(), file);
    }

    // Second call: cache hit, direct write
    let result = rt.direct_write(&pane_id, b"hello");
    assert!(matches!(result, Ok(true)), "second call should succeed");

    // Cleanup
    let _ = Command::new("tmux").args(["kill-session", "-t", "pmux-test-dw"]).output();
}
```

### Step 8: 运行测试

```bash
RUSTUP_TOOLCHAIN=stable cargo test tmux_control_mode::tests::test_resolve_pane_tty
RUSTUP_TOOLCHAIN=stable cargo test tmux_control_mode::tests::test_direct_write
```

Expected: PASS

### Step 9: 手动验证

```bash
RUSTUP_TOOLCHAIN=stable cargo run
```

1. 打开一个 workspace
2. 快速连打 `aaaaaa...`，观察是否有延迟
3. 按住方向键，观察是否流畅
4. 运行 `vim`，测试 `hjkl` 移动和 `i` 插入模式打字
5. 运行 `fzf`，测试方向键选择

---

## Task 2: 渲染使用 BatchedTextRun（替代逐字符 shape_line）

**Files:**
- Modify: `vendor/gpui-terminal/src/render.rs`

### 问题

`layout_row()` 已经产生 `Vec<BatchedTextRun>`（按样式合并的文本块），但 `paint()` 在 line 508 丢弃了它：

```rust
let (backgrounds, _) = self.layout_row(line_idx, cells.iter().cloned(), colors);
//                  ^ text_runs 被丢弃！
```

然后在第三个 pass 里逐字符调用 `shape_line`，80 列的一行产生最多 80 次 shape_line 调用。

### Step 1: 保留 text_runs 并用于渲染

将 paint() 中的 "Third pass: draw regular text characters" 循环替换为基于 `BatchedTextRun` 的渲染。

**修改位置:** `paint()` 函数内，约 line 494-723

**改前:** line 508:
```rust
let (backgrounds, _) = self.layout_row(line_idx, cells.iter().cloned(), colors);
```

**改后:**
```rust
let (backgrounds, text_runs) = self.layout_row(line_idx, cells.iter().cloned(), colors);
```

**然后删除整个 "Third pass: draw regular text characters" 循环** (约 line 652-722)，替换为：

```rust
// Render batched text runs (one shape_line per style run instead of per character)
let base_height = self.cell_height / self.line_height_multiplier;
let vertical_offset = (self.cell_height - base_height) / 2.0;
let y_base = origin.y + self.cell_height * (line_idx as f32);

for run in &text_runs {
    // Skip whitespace-only runs
    if run.text.chars().all(|c| c == ' ' || c == '\0') {
        continue;
    }

    // Skip box-drawing characters (handled in earlier passes)
    if run.text.chars().all(|c| box_drawing::is_box_drawing_char(c)) {
        continue;
    }

    let x = origin.x + self.cell_width * (run.start_col as f32);
    let y = y_base + vertical_offset;

    let font = Font {
        family: self.font_family.clone().into(),
        features: FontFeatures::default(),
        fallbacks: None,
        weight: if run.bold { FontWeight::BOLD } else { FontWeight::NORMAL },
        style: if run.italic { FontStyle::Italic } else { FontStyle::Normal },
    };

    // Filter out box-drawing chars from the run text, keeping positions
    let filtered: String = run.text.chars()
        .map(|c| if box_drawing::is_box_drawing_char(c) { ' ' } else { c })
        .collect();

    let text_run = TextRun {
        len: filtered.len(),
        font,
        color: run.fg_color,
        background_color: None,
        underline: if run.underline {
            Some(UnderlineStyle {
                thickness: px(1.0),
                color: Some(run.fg_color),
                wavy: false,
            })
        } else {
            None
        },
        strikethrough: None,
    };

    let text: SharedString = filtered.into();
    let shaped_line = window.text_system().shape_line(
        text, self.font_size, &[text_run], None,
    );
    let _ = shaped_line.paint(
        Point { x, y },
        self.cell_height,
        TextAlign::default(),
        None,
        window,
        _cx,
    );
}
```

### Step 2: 运行测试

```bash
RUSTUP_TOOLCHAIN=stable cargo test -p gpui-terminal
```

Expected: PASS（现有 renderer 测试应继续通过）

### Step 3: 手动验证渲染正确性

```bash
RUSTUP_TOOLCHAIN=stable cargo run
```

1. 打开终端，运行 `ls --color` 验证颜色正确
2. 运行 `htop` 或 `btop` 验证 box-drawing + 色彩混合
3. 运行 `git log --oneline --graph --all --color` 验证复杂输出
4. 运行 `cat /dev/urandom | head -c 1000 | xxd | head -20` 验证密集文本

---

## Task 3: 清理渲染热路径中的 debug 日志

**Files:**
- Modify: `vendor/gpui-terminal/src/render.rs` (line 725-742)
- Modify: `vendor/gpui-terminal/src/view.rs` (line 1012-1020)

### Step 1: 移除 render.rs 中的 debug 日志

删除 paint() 中 `// #region agent log` ... `// #endregion` 块（line 725-742）。

### Step 2: 移除 view.rs 中的 debug 日志

删除 canvas paint callback 中 `// #region agent log` ... `// #endregion` 块（line 1012-1020）。

### Step 3: 验证编译

```bash
RUSTUP_TOOLCHAIN=stable cargo check -p gpui-terminal
```

---

## Task 4: 缓存 cell metrics

**Files:**
- Modify: `vendor/gpui-terminal/src/view.rs`

### 问题

每次 paint 都执行：
```rust
let mut measured_renderer = renderer.clone();
measured_renderer.measure_cell(window);
```

cell 尺寸只在字体/大小变化时改变。

### Step 1: 添加缓存字段到 TerminalView

在 `TerminalView` struct 中添加：

```rust
/// Cached cell dimensions to avoid re-measuring on every paint.
/// Reset when config changes (font/size).
cached_cell_width: Arc<Mutex<Option<Pixels>>>,
cached_cell_height: Arc<Mutex<Option<Pixels>>>,
```

### Step 2: 在 update_config 时清除缓存

在 `update_config()` 末尾加：

```rust
// Invalidate cached cell dimensions
if let Ok(mut w) = self.cached_cell_width.lock() { *w = None; }
if let Ok(mut h) = self.cached_cell_height.lock() { *h = None; }
```

### Step 3: 在 canvas paint 中使用缓存

将 canvas callback 中的 measure_cell 改为：

```rust
let mut measured_renderer = renderer.clone();

// Use cached cell dimensions if available
let (cached_w, cached_h) = {
    let w = cached_cell_width.lock().ok().and_then(|g| *g);
    let h = cached_cell_height.lock().ok().and_then(|g| *g);
    (w, h)
};

if let (Some(w), Some(h)) = (cached_w, cached_h) {
    measured_renderer.cell_width = w;
    measured_renderer.cell_height = h;
} else {
    measured_renderer.measure_cell(window);
    // Cache the result
    if let Ok(mut g) = cached_cell_width.lock() { *g = Some(measured_renderer.cell_width); }
    if let Ok(mut g) = cached_cell_height.lock() { *g = Some(measured_renderer.cell_height); }
}
```

注意：需要将 `cached_cell_width` 和 `cached_cell_height` clone 到 canvas closure 中。

---

## 实施顺序

| 优先级 | Task | 预估 | 收益 |
|--------|------|------|------|
| **P0** | Task 1: Direct PTY write | 30-45 min | 输入延迟 **-90%** |
| **P0** | Task 2: Batched text rendering | 30-45 min | 渲染 draw calls **-10~50x** |
| **P1** | Task 3: 清理 debug 日志 | 5 min | 消除渲染热路径 I/O |
| **P1** | Task 4: 缓存 cell metrics | 15 min | paint 开销 **-10~20%** |

**并行策略:** Task 1 (tmux_control_mode.rs) 和 Task 2 (render.rs) 修改不同文件，可用 `subagent-driven-development` 并行执行。Task 3 和 Task 4 修改 gpui-terminal，应在 Task 2 之后顺序执行。

---

## 验收标准

1. **输入流畅度:** 快速连打 30+ 字符/秒无感知卡顿
2. **方向键:** vim 中 `hjkl` 移动、fzf 中方向键选择流畅
3. **渲染正确性:** 颜色、粗体、斜体、下划线、box-drawing 均正常
4. **Fallback 工作:** 当 pane TTY 不可用时（极端情况）自动回退到 send-keys
5. **session 恢复:** 关闭 GUI 重新打开，输入仍正常（TTY 缓存自动重建）
6. **split pane:** ⌘D 分屏后新 pane 输入正常（TTY 缓存按需填充）
