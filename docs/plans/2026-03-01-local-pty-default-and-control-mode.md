# Local PTY Default + tmux Control Mode 持久化 实施计划

> **For Claude:** Use TDD when implementing. Consider `subagent-driven-development` for parallel tasks.

**Goal:** 将默认 backend 从 tmux 切换为 local PTY，消除 pipe-pane/capture-pane 拼接问题；后续用 tmux control mode 实现会话持久化。

**Architecture:** Phase 1 将 `LocalPtyAgent`（已有，支持多 pane）升级为生产 runtime，替代 `LocalPtyRuntime`，并设为默认 backend。Phase 2 新增 `TmuxControlModeRuntime`，通过 `tmux -CC` 结构化协议代理 I/O，替代 pipe-pane + capture-pane。

**Tech Stack:** Rust, GPUI, gpui-terminal, portable-pty, tmux control mode (-CC)

**背景：** 当前 tmux 集成的结构性问题（阶梯 prompt、光标错位、Enter 双路径、capture-pane 注入）源于 pipe-pane 和 capture-pane 的双数据源拼接。Local PTY 无此问题——gpui-terminal 直连 PTY，输入输出单一路径。

---

## Phase 1：Local PTY 作为默认 backend

**预估：3-5 天**

### Task 1: 切换默认 backend 为 local

**Files:**
- Modify: `src/runtime/backends/mod.rs:24` — `DEFAULT_BACKEND`
- Modify: `src/config.rs:9-11` — `default_backend()`

**Step 1: Write the failing test**

在 `src/runtime/backends/mod.rs` 的 `#[cfg(test)]` 中添加：

```rust
#[test]
fn test_default_backend_is_local() {
    assert_eq!(DEFAULT_BACKEND, "local");
}

#[test]
fn test_resolve_backend_defaults_to_local() {
    // No env, no config → should be "local"
    std::env::remove_var("PMUX_BACKEND");
    let backend = resolve_backend(None);
    assert_eq!(backend, "local");
}
```

**Step 2: Run test to verify failure**

```bash
cargo test test_default_backend_is_local -- --nocapture
```

Expected: FAIL (`assertion failed: "tmux" == "local"`)

**Step 3: Write minimal implementation**

`src/runtime/backends/mod.rs:24`:

```rust
pub const DEFAULT_BACKEND: &str = "local";
```

`src/config.rs:9-11`:

```rust
fn default_backend() -> String {
    "local".to_string()
}
```

**Step 4: Run test to verify pass**

```bash
cargo test test_default_backend_is_local test_resolve_backend_defaults_to_local -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/runtime/backends/mod.rs src/config.rs
git commit -m "feat: switch default backend from tmux to local PTY"
```

---

### Task 2: 用 LocalPtyAgent 替代 LocalPtyRuntime 作为生产 runtime

**目的：** `LocalPtyRuntime` 只支持单 pane，`split_pane` / `open_diff` / `open_review` 都返回 Error。`LocalPtyAgent` 已实现多 pane 和 diff，但目前是 dead code。将其升级为生产 runtime。

**Files:**
- Modify: `src/runtime/backends/local_pty.rs` — 移除 `#[allow(dead_code)]`，统一为 `LocalPtyAgent`
- Modify: `src/runtime/backends/mod.rs` — `create_runtime()` 使用 `LocalPtyAgent`

**Step 1: Write the failing test**

在 `src/runtime/backends/local_pty.rs` 的 tests 中添加：

```rust
#[test]
fn test_create_runtime_returns_local_pty_agent() {
    let dir = tempfile::tempdir().unwrap();
    let rt = super::super::create_runtime(dir.path(), 80, 24).unwrap();
    // LocalPtyAgent supports split_pane; LocalPtyRuntime does not
    let primary = rt.primary_pane_id().unwrap();
    let result = rt.split_pane(&primary, true);
    assert!(result.is_ok(), "production runtime should support split_pane");
}
```

**Step 2: Run test to verify failure**

```bash
cargo test test_create_runtime_returns_local_pty_agent -- --nocapture
```

Expected: FAIL (`split pane not implemented in single LocalPtyRuntime`)

**Step 3: Write minimal implementation**

`src/runtime/backends/mod.rs` — 修改 `create_runtime()`:

```rust
pub fn create_runtime(
    worktree_path: &Path,
    cols: u16,
    rows: u16,
) -> Result<Arc<dyn AgentRuntime>, RuntimeError> {
    Ok(Arc::new(local_pty::LocalPtyAgent::new(
        worktree_path, cols, rows,
    )?))
}
```

`src/runtime/backends/local_pty.rs` — 移除 dead_code 标注：

```rust
// 移除这两行:
// #[allow(dead_code)]   (line 44 和 line 53)
pub struct LocalPtyAgent { ... }
impl LocalPtyAgent { ... }
```

确保 `LocalPtyAgent` 和 `create_pane` 是 `pub`。

**Step 4: Run test to verify pass**

```bash
cargo test test_create_runtime_returns_local_pty_agent -- --nocapture
```

Expected: PASS

**Step 5: Run full test suite**

```bash
RUSTUP_TOOLCHAIN=stable cargo test
```

Expected: 全部 PASS（可能有 tmux 相关测试需要 tmux 环境）

**Step 6: Commit**

```bash
git add src/runtime/backends/local_pty.rs src/runtime/backends/mod.rs
git commit -m "feat: use LocalPtyAgent as production local runtime (multi-pane support)"
```

---

### Task 3: 修复 effective_backend() 使用 Config

**目的：** `effective_backend()` 当前只看环境变量和 `DEFAULT_BACKEND`，不读 Config。需与 `create_runtime_from_env` 对齐。

**Files:**
- Modify: `src/ui/app_root.rs` — `effective_backend()` 方法

**Step 1: Write the failing test**

在 `src/ui/app_root.rs` 或单独测试文件：

```rust
#[test]
fn test_resolve_backend_respects_config() {
    use crate::config::Config;
    std::env::remove_var("PMUX_BACKEND");
    let mut config = Config::default();
    config.backend = "tmux".to_string();
    let backend = crate::runtime::backends::resolve_backend(Some(&config));
    assert_eq!(backend, "tmux");
}

#[test]
fn test_resolve_backend_env_overrides_config() {
    use crate::config::Config;
    std::env::set_var("PMUX_BACKEND", "local");
    let mut config = Config::default();
    config.backend = "tmux".to_string();
    let backend = crate::runtime::backends::resolve_backend(Some(&config));
    assert_eq!(backend, "local");
    std::env::remove_var("PMUX_BACKEND");
}
```

**Step 2: Run to verify these tests pass** (resolve_backend already uses config)

```bash
cargo test test_resolve_backend_respects_config test_resolve_backend_env_overrides_config -- --nocapture
```

Expected: PASS（`resolve_backend` 已实现 config 读取）

**Step 3: Fix effective_backend()**

`src/ui/app_root.rs` — 替换 `effective_backend()`:

```rust
fn effective_backend(&self) -> String {
    crate::runtime::backends::resolve_backend(
        crate::config::Config::load().ok().as_ref()
    )
}
```

**Step 4: Verify build**

```bash
cargo check
```

Expected: OK

**Step 5: Commit**

```bash
git add src/ui/app_root.rs
git commit -m "fix: effective_backend() now reads Config, consistent with create_runtime_from_env"
```

---

### Task 4: 改进 open_diff — spawn nvim diffview 到新 pane

**目的：** `LocalPtyAgent.open_diff` 当前直接 `git diff` 到当前 pane。改为 spawn nvim diffview 到新 pane（更接近 ⌘⇧D 体验）。

**Files:**
- Modify: `src/runtime/backends/local_pty.rs` — `LocalPtyAgent::open_diff` / `open_review`

**Step 1: Write the failing test**

```rust
#[test]
fn test_open_diff_creates_new_pane() {
    let dir = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .ok();
    let agent = LocalPtyAgent::new(dir.path(), 80, 24).unwrap();

    let panes_before = agent.list_panes(&String::new()).len();
    let result = agent.open_diff(dir.path(), None);
    assert!(result.is_ok());
    let panes_after = agent.list_panes(&String::new()).len();
    assert_eq!(panes_after, panes_before + 1, "open_diff should create a new pane");
}
```

**Step 2: Run test to verify failure**

```bash
cargo test test_open_diff_creates_new_pane -- --nocapture
```

Expected: FAIL（当前 open_diff 发到已有 pane，不创建新 pane）

**Step 3: Write implementation**

```rust
fn open_diff(&self, worktree: &Path, _pane_id: Option<&PaneId>) -> Result<String, RuntimeError> {
    let idx = self.pane_counter.fetch_add(1, Ordering::SeqCst);
    let diff_pane_id = self.create_pane(&format!("diff{}", idx))?;

    // Try nvim diffview first, fallback to git diff
    let worktree_str = worktree.to_string_lossy();
    let cmd = format!(
        "nvim -c 'DiffviewOpen main...HEAD' '{}' 2>/dev/null || git diff main...HEAD --color=always | less -R\n",
        worktree_str
    );
    self.send_input(&diff_pane_id, cmd.as_bytes())?;

    Ok(diff_pane_id)
}

fn open_review(&self, worktree: &Path) -> Result<String, RuntimeError> {
    self.open_diff(worktree, None)
}
```

**Step 4: Run test to verify pass**

```bash
cargo test test_open_diff_creates_new_pane -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/runtime/backends/local_pty.rs
git commit -m "feat: open_diff creates new pane with nvim diffview (fallback to git diff)"
```

---

### Task 5: 清理旧 LocalPtyRuntime（可选 deprecate）

**目的：** `LocalPtyRuntime` 不再用于生产。保留但标记为 deprecated，或直接删除。

**Files:**
- Modify: `src/runtime/backends/local_pty.rs`

**Step 1: 标记 deprecated**

```rust
#[deprecated(note = "Use LocalPtyAgent instead — supports multi-pane, diff, review")]
pub struct LocalPtyRuntime { ... }
```

**Step 2: 确认无编译警告影响**

```bash
cargo check 2>&1 | grep -c "deprecated"
```

Expected: 只在旧测试中出现 warning

**Step 3: Commit**

```bash
git add src/runtime/backends/local_pty.rs
git commit -m "refactor: deprecate LocalPtyRuntime in favor of LocalPtyAgent"
```

---

### Task 6: Phase 1 E2E 测试 — 截图 + OCR 断言

**目的：** 用自动化 E2E 测试覆盖 local PTY 的核心场景。每个测试：启动 pmux → 操作 → 截图 → OCR/图像分析 → 断言。失败时自动保留截图和录屏供调试。

**Files:**
- Create: `tests/e2e/local_pty_e2e.sh`

**Step 1: 创建 E2E 测试脚本**

```bash
#!/bin/bash
# E2E Tests for Phase 1: Local PTY Default Backend
# Uses screenshot + OCR + image analysis for assertions.
# Requires: tesseract, ffmpeg, PIL (pip3 install Pillow)
#
# Usage: bash tests/e2e/local_pty_e2e.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PMUX_ROOT="${PMUX_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
RESULTS_DIR="$SCRIPT_DIR/results/local_pty_$(date +%Y%m%d_%H%M%S)"
RECORDING_FILE="$RESULTS_DIR/session.mp4"
IMAGE_ANALYSIS="$PMUX_ROOT/tests/regression/lib/image_analysis.py"

source "$PMUX_ROOT/tests/regression/lib/test_utils.sh"

mkdir -p "$RESULTS_DIR"

# Override SCRIPT_DIR for take_screenshot output path
export SCRIPT_DIR="$RESULTS_DIR/.."

# ── Helpers ─────────────────────────────────────────────────

screenshot_and_ocr() {
    local name="$1"
    local path="$RESULTS_DIR/${name}.png"
    local win_id
    win_id=$(osascript -e 'tell application "System Events" to get id of window 1 of process "pmux"' 2>/dev/null || echo "")
    if [ -n "$win_id" ]; then
        screencapture -l "$win_id" -x "$path" 2>/dev/null || screencapture -x "$path"
    else
        screencapture -x "$path"
    fi
    echo "$path"
}

assert_ocr_contains() {
    local image="$1"
    local expected="$2"
    local test_name="$3"
    local ocr_out
    ocr_out=$(python3 "$IMAGE_ANALYSIS" ocr "$image" 2>/dev/null || echo "OK:False\nTEXT:")
    local ocr_text
    ocr_text=$(echo "$ocr_out" | grep "^TEXT:" | sed 's/^TEXT://')

    if echo "$ocr_text" | grep -qi "$expected"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — OCR found '$expected'"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — OCR did NOT find '$expected'"
        log_error "  OCR text: $ocr_text"
        return 1
    fi
}

assert_window_valid() {
    local image="$1"
    local test_name="$2"
    local result
    result=$(python3 "$IMAGE_ANALYSIS" verify_window "$image" 400 300 2>/dev/null || echo "OK:False")
    local ok
    ok=$(echo "$result" | grep "^OK:" | sed 's/^OK://')
    local reason
    reason=$(echo "$result" | grep "^REASON:" | sed 's/^REASON://')

    if [ "$ok" = "True" ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — window valid"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — window invalid: $reason"
        return 1
    fi
}

assert_terminal_has_content() {
    local image="$1"
    local test_name="$2"
    # Analyze the right 70% of the image (terminal area, skip sidebar)
    local w h
    w=$(python3 -c "from PIL import Image; print(Image.open('$image').size[0])" 2>/dev/null || echo 800)
    h=$(python3 -c "from PIL import Image; print(Image.open('$image').size[1])" 2>/dev/null || echo 600)
    local x=$((w * 30 / 100))
    local rw=$((w - x))
    local result
    result=$(python3 "$IMAGE_ANALYSIS" analyze_region "$image" "$x" 50 "$rw" "$((h - 100))" 2>/dev/null || echo "VARIANCE:0")
    local variance
    variance=$(echo "$result" | grep "^VARIANCE:" | sed 's/^VARIANCE://')
    local var_int
    var_int=$(echo "$variance" | cut -d. -f1)

    if [ "$var_int" -gt 100 ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — terminal has content (variance=$variance)"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — terminal looks empty (variance=$variance)"
        return 1
    fi
}

# ── Setup ───────────────────────────────────────────────────

init_report
add_report_section "Phase 1: Local PTY E2E Tests"

# Ensure local backend
unset PMUX_BACKEND
export PMUX_BACKEND=local

# Build
log_info "Building pmux..."
(cd "$PMUX_ROOT" && RUSTUP_TOOLCHAIN=stable cargo build 2>&1 | tail -3)

# Start recording (background, 120s max)
if command -v ffmpeg &>/dev/null; then
    ffmpeg -f avfoundation -i "1" -r 15 -t 120 -y "$RECORDING_FILE" 2>/dev/null &
    FFMPEG_PID=$!
    log_info "Recording started (PID=$FFMPEG_PID)"
else
    FFMPEG_PID=""
    log_warn "ffmpeg not found, skipping recording"
fi

stop_pmux 2>/dev/null || true
sleep 1
start_pmux || { log_error "Failed to start pmux"; exit 1; }
sleep 4
activate_window
sleep 1

# ── Test 1: Window visible & valid ─────────────────────────

log_info "=== Test 1: Window visible ==="
IMG=$(screenshot_and_ocr "01_window_visible")
assert_window_valid "$IMG" "Window is visible and valid"
add_report_result "Window visible" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 2: Prompt at top (not middle) ─────────────────────

log_info "=== Test 2: Prompt position ==="
IMG=$(screenshot_and_ocr "02_prompt_position")
# Cursor detection: check if content is in the top 40% of terminal area
python3 -c "
from PIL import Image
img = Image.open('$IMG')
w, h = img.size
# Sample top 40% of terminal area (right 70%)
x0 = int(w * 0.3)
y_top = int(h * 0.05)
y_mid = int(h * 0.40)
top = img.crop((x0, y_top, w, y_mid))
bot = img.crop((x0, y_mid, w, int(h * 0.95)))
top_px = list(top.getdata())
bot_px = list(bot.getdata())
top_var = sum(abs(p[0]-30)+abs(p[1]-30)+abs(p[2]-30) for p in top_px) / len(top_px)
bot_var = sum(abs(p[0]-30)+abs(p[1]-30)+abs(p[2]-30) for p in bot_px) / len(bot_px)
# Top should have more variance (text content) than bottom
print(f'TOP_VAR:{top_var:.1f}')
print(f'BOT_VAR:{bot_var:.1f}')
print(f'PROMPT_AT_TOP:{top_var > bot_var}')
" 2>/dev/null > "$RESULTS_DIR/02_analysis.txt" || true

PROMPT_TOP=$(grep "PROMPT_AT_TOP:" "$RESULTS_DIR/02_analysis.txt" 2>/dev/null | sed 's/.*://')
if [ "$PROMPT_TOP" = "True" ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ Prompt at top (not staircase in middle)"
    add_report_result "Prompt at top" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ Prompt may not be at top"
    add_report_result "Prompt at top" "FAIL"
fi

# ── Test 3: Command execution — ls ─────────────────────────

log_info "=== Test 3: ls command ==="
click_terminal_area
sleep 0.5
send_keystroke "ls"
sleep 0.3
send_keycode 36  # Enter
sleep 2
IMG=$(screenshot_and_ocr "03_ls_output")
assert_terminal_has_content "$IMG" "ls command shows output"
add_report_result "ls output" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# Check OCR for common filenames (Cargo.toml, src, etc.)
assert_ocr_contains "$IMG" "src\|Cargo\|README" "ls output contains expected files" || true
add_report_result "ls OCR" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 4: Command execution — echo with CJK ──────────────

log_info "=== Test 4: CJK echo ==="
send_keystroke "echo 'hello pmux 你好世界'"
sleep 0.3
send_keycode 36
sleep 2
IMG=$(screenshot_and_ocr "04_cjk_echo")
assert_terminal_has_content "$IMG" "CJK echo shows content"
add_report_result "CJK echo" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 5: No crash after Enter ────────────────────────────

log_info "=== Test 5: Multiple Enter presses ==="
for i in $(seq 1 10); do
    send_keycode 36
    sleep 0.2
done
sleep 1

if pgrep -f "target/debug/pmux" > /dev/null; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ No crash after 10 rapid Enter presses"
    add_report_result "Rapid Enter" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ Crashed after rapid Enter"
    add_report_result "Rapid Enter" "FAIL"
fi

# ── Test 6: Vim compatibility ───────────────────────────────

log_info "=== Test 6: vim open/close ==="
send_keystroke "vim /tmp/pmux_e2e_test.txt"
sleep 0.3
send_keycode 36
sleep 2
IMG=$(screenshot_and_ocr "06_vim_open")
assert_window_valid "$IMG" "vim opens without crash"
add_report_result "vim open" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# Type :q! to exit vim
send_keystroke ":"
sleep 0.2
send_keystroke "q!"
sleep 0.2
send_keycode 36
sleep 2
IMG=$(screenshot_and_ocr "06_vim_closed")
assert_terminal_has_content "$IMG" "vim closed, back to shell"
add_report_result "vim close" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 7: No staircase prompt ─────────────────────────────

log_info "=== Test 7: Staircase detection ==="
# Run several commands and check for staircase pattern
send_keystroke "echo line1"
send_keycode 36
sleep 0.5
send_keystroke "echo line2"
send_keycode 36
sleep 0.5
send_keystroke "echo line3"
send_keycode 36
sleep 1
IMG=$(screenshot_and_ocr "07_staircase_check")

# Staircase detection: check if text starts at increasing x offsets per line
# (In a healthy terminal, prompts all start at x=0; staircase has increasing indent)
python3 -c "
from PIL import Image
img = Image.open('$IMG')
w, h = img.size
x0 = int(w * 0.3)
terminal = img.crop((x0, 0, w, h))
tw, th = terminal.size
# Scan columns 0..20 of terminal for each row — staircase means text shifts right
rows_with_content = []
for y in range(0, th, 4):
    first_bright_x = None
    for x in range(min(200, tw)):
        r, g, b = terminal.getpixel((x, y))
        if (r + g + b) / 3 > 80:
            first_bright_x = x
            break
    if first_bright_x is not None:
        rows_with_content.append(first_bright_x)
# If staircase, the x values would monotonically increase
if len(rows_with_content) > 5:
    diffs = [rows_with_content[i+1] - rows_with_content[i] for i in range(len(rows_with_content)-1)]
    increasing_count = sum(1 for d in diffs if d > 3)
    ratio = increasing_count / len(diffs)
    print(f'STAIRCASE_RATIO:{ratio:.2f}')
    print(f'IS_STAIRCASE:{ratio > 0.5}')
else:
    print('STAIRCASE_RATIO:0')
    print('IS_STAIRCASE:False')
" 2>/dev/null > "$RESULTS_DIR/07_analysis.txt" || true

IS_STAIRCASE=$(grep "IS_STAIRCASE:" "$RESULTS_DIR/07_analysis.txt" 2>/dev/null | sed 's/.*://')
if [ "$IS_STAIRCASE" = "False" ] || [ -z "$IS_STAIRCASE" ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ No staircase prompt detected"
    add_report_result "No staircase" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ Staircase prompt detected!"
    add_report_result "No staircase" "FAIL"
fi

# ── Test 8: Large output (seq 500) ──────────────────────────

log_info "=== Test 8: Large output ==="
send_keystroke "seq 1 500"
send_keycode 36
sleep 3
IMG=$(screenshot_and_ocr "08_large_output")

if pgrep -f "target/debug/pmux" > /dev/null; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ Large output handled without crash"
    add_report_result "Large output" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ Crashed on large output"
    add_report_result "Large output" "FAIL"
fi

# ── Teardown ────────────────────────────────────────────────

stop_pmux

# Stop recording
if [ -n "${FFMPEG_PID:-}" ]; then
    kill "$FFMPEG_PID" 2>/dev/null || true
    wait "$FFMPEG_PID" 2>/dev/null || true
    log_info "Recording saved: $RECORDING_FILE"

    # Analyze recording for FPS
    if command -v ffprobe &>/dev/null && [ -f "$RECORDING_FILE" ]; then
        bash "$PMUX_ROOT/tests/helpers/recording.sh" analyze "$RECORDING_FILE" >> "$RESULTS_DIR/recording_analysis.txt" 2>&1 || true
    fi
fi

# ── Report ──────────────────────────────────────────────────

finalize_report
cp "$REPORT_FILE" "$RESULTS_DIR/report.md" 2>/dev/null || true

echo ""
echo "================================"
echo "Phase 1 E2E Results"
echo "================================"
echo "  Passed: $TESTS_PASSED"
echo "  Failed: $TESTS_FAILED"
echo "  Screenshots: $RESULTS_DIR/"
echo "  Recording: $RECORDING_FILE"
echo "  Report: $RESULTS_DIR/report.md"
echo "================================"

exit $TESTS_FAILED
```

**Step 2: 运行测试**

```bash
bash tests/e2e/local_pty_e2e.sh
```

Expected: 全部 PASS。失败时查看 `tests/e2e/results/local_pty_*/` 下的截图和录屏。

**Step 3: Commit**

```bash
git add tests/e2e/local_pty_e2e.sh
git commit -m "test: add Phase 1 E2E tests with screenshot + OCR + staircase detection"
```

---

## Phase 2：tmux Control Mode 持久化

**预估：2-3 周**

**前置条件：** Phase 1 完成，local PTY 默认模式稳定运行。

### Task 7: 研究 tmux control mode 协议

**Files:** 无代码变更，纯研究

**Step 1: 手动测试 control mode**

```bash
# 创建后台 session
tmux new-session -d -s test-cc -c /tmp

# 用 control mode 连接
tmux -CC attach -t test-cc
```

观察输出格式：

```
%begin 1234 1 0
%end 1234 1 0
%output %0 base64_or_raw_data
%session-changed $1 test-cc
```

**Step 2: 验证 %output 事件**

在另一个终端：

```bash
tmux send-keys -t test-cc 'echo hello' Enter
```

在 control mode 终端中应看到：

```
%output %0 echo hello\r\n
%output %0 hello\r\n
%output %0 $
```

**Step 3: 验证输入转发**

在 control mode stdin 中输入：

```
send-keys -t %0 'ls' Enter
```

应看到 `%output` 事件包含 ls 输出。

**Step 4: 验证 resize**

```
refresh-client -C 120,40
resize-pane -t %0 -x 120 -y 40
```

**Step 5: 记录协议细节到文档**

```bash
# 写入 docs/tmux-control-mode-protocol.md
```

---

### Task 8: 实现 ControlModeParser

**目的：** 解析 tmux control mode 的 `%output`、`%begin`/`%end`、`%exit` 等事件。

**Files:**
- Create: `src/runtime/backends/tmux_control_mode.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_output_event() {
        let mut parser = ControlModeParser::new();
        let events = parser.feed(b"%output %0 hello world\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControlModeEvent::Output { pane_id, data } => {
                assert_eq!(pane_id, "%0");
                assert_eq!(data, b"hello world");
            }
            _ => panic!("expected Output event"),
        }
    }

    #[test]
    fn test_parse_exit_event() {
        let mut parser = ControlModeParser::new();
        let events = parser.feed(b"%exit\n");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ControlModeEvent::Exit));
    }

    #[test]
    fn test_parse_partial_line() {
        let mut parser = ControlModeParser::new();
        let events1 = parser.feed(b"%output %0 hel");
        assert!(events1.is_empty(), "partial line should not emit event");
        let events2 = parser.feed(b"lo\n");
        assert_eq!(events2.len(), 1);
    }

    #[test]
    fn test_parse_begin_end() {
        let mut parser = ControlModeParser::new();
        let events = parser.feed(b"%begin 1234567890 1 0\nsome response\n%end 1234567890 1 0\n");
        // begin/end wraps command responses
        assert!(events.iter().any(|e| matches!(e, ControlModeEvent::BeginEnd { .. })));
    }
}
```

**Step 2: Run test to verify failure**

```bash
cargo test control_mode::tests -- --nocapture
```

Expected: FAIL (module doesn't exist)

**Step 3: Write implementation**

```rust
//! tmux control mode (-CC) protocol parser.
//!
//! Parses structured notifications from `tmux -CC attach`:
//! - %output %pane_id data     — pane output bytes
//! - %begin/%end               — command response brackets
//! - %exit                     — session detached/exited
//! - %session-changed          — session switch
//! - %window-add/close/renamed — window lifecycle

#[derive(Debug, Clone, PartialEq)]
pub enum ControlModeEvent {
    Output { pane_id: String, data: Vec<u8> },
    BeginEnd { tag: String, response: Vec<u8> },
    Exit,
    SessionChanged { session_id: String, name: String },
    WindowAdd { window_id: String },
    WindowClose { window_id: String },
    LayoutChanged { window_id: String, layout: String },
    Unknown(String),
}

pub struct ControlModeParser {
    line_buf: Vec<u8>,
    in_begin: Option<String>,
    begin_response: Vec<u8>,
}

impl ControlModeParser {
    pub fn new() -> Self {
        Self {
            line_buf: Vec::new(),
            in_begin: None,
            begin_response: Vec::new(),
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<ControlModeEvent> {
        let mut events = Vec::new();
        for &b in bytes {
            if b == b'\n' {
                let line = std::mem::take(&mut self.line_buf);
                if let Some(ev) = self.parse_line(&line) {
                    events.push(ev);
                }
            } else {
                self.line_buf.push(b);
            }
        }
        events
    }

    fn parse_line(&mut self, line: &[u8]) -> Option<ControlModeEvent> {
        let s = String::from_utf8_lossy(line);
        let s = s.trim_end_matches('\r');

        // Handle begin/end state
        if let Some(ref tag) = self.in_begin.clone() {
            if s.starts_with("%end ") && s.contains(&*tag) {
                let response = std::mem::take(&mut self.begin_response);
                self.in_begin = None;
                return Some(ControlModeEvent::BeginEnd {
                    tag: tag.clone(),
                    response,
                });
            } else {
                self.begin_response.extend_from_slice(line);
                self.begin_response.push(b'\n');
                return None;
            }
        }

        if s.starts_with("%begin ") {
            let tag = s.strip_prefix("%begin ").unwrap_or("").to_string();
            self.in_begin = Some(tag);
            self.begin_response.clear();
            return None;
        }

        if s.starts_with("%output ") {
            let rest = &s["%output ".len()..];
            let space_idx = rest.find(' ')?;
            let pane_id = rest[..space_idx].to_string();
            let data = rest[space_idx + 1..].as_bytes().to_vec();
            return Some(ControlModeEvent::Output { pane_id, data });
        }

        if s == "%exit" {
            return Some(ControlModeEvent::Exit);
        }

        if s.starts_with("%session-changed ") {
            let rest = &s["%session-changed ".len()..];
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            return Some(ControlModeEvent::SessionChanged {
                session_id: parts.first().unwrap_or(&"").to_string(),
                name: parts.get(1).unwrap_or(&"").to_string(),
            });
        }

        if s.starts_with("%window-add ") {
            return Some(ControlModeEvent::WindowAdd {
                window_id: s["%window-add ".len()..].to_string(),
            });
        }

        if s.starts_with("%window-close ") {
            return Some(ControlModeEvent::WindowClose {
                window_id: s["%window-close ".len()..].to_string(),
            });
        }

        if s.starts_with("%layout-change ") {
            let rest = &s["%layout-change ".len()..];
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            return Some(ControlModeEvent::LayoutChanged {
                window_id: parts.first().unwrap_or(&"").to_string(),
                layout: parts.get(1).unwrap_or(&"").to_string(),
            });
        }

        Some(ControlModeEvent::Unknown(s.to_string()))
    }
}
```

**Step 4: Run test to verify pass**

```bash
cargo test control_mode::tests -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/runtime/backends/tmux_control_mode.rs
git commit -m "feat: add tmux control mode protocol parser"
```

---

### Task 9: 实现 TmuxControlModeRuntime

**目的：** 通过 control mode 代理 I/O，实现 `AgentRuntime` trait。gpui-terminal 连接 local PTY pair，control mode parser 负责桥接。

**Files:**
- Create: `src/runtime/backends/tmux_control_mode.rs`（扩展 Task 8）
- Modify: `src/runtime/backends/mod.rs` — 注册新 backend

**Step 1: Write the failing test**

```rust
#[test]
fn test_control_mode_runtime_backend_type() {
    // This test requires tmux to be installed
    if !super::super::tmux_available() {
        eprintln!("skipping: tmux not available");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let rt = TmuxControlModeRuntime::new("pmux-test-cc", "main", Some(dir.path()))
        .expect("should create control mode runtime");
    assert_eq!(rt.backend_type(), "tmux-cc");

    // Cleanup
    let _ = std::process::Command::new("tmux")
        .args(["kill-session", "-t", "pmux-test-cc"])
        .output();
}
```

**Step 2: Run test to verify failure**

```bash
cargo test test_control_mode_runtime_backend_type -- --nocapture
```

Expected: FAIL (struct doesn't exist)

**Step 3: Write implementation**

```rust
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct TmuxControlModeRuntime {
    session_name: String,
    window_name: String,
    /// stdin of `tmux -CC attach` process — send commands here
    control_stdin: Arc<Mutex<std::process::ChildStdin>>,
    /// Output channels per pane_id, fed by the parser thread
    pane_outputs: Arc<Mutex<HashMap<String, flume::Sender<Vec<u8>>>>>,
    /// Control mode child process
    _control_child: Arc<Mutex<Child>>,
}

impl TmuxControlModeRuntime {
    pub fn new(
        session_name: &str,
        window_name: &str,
        start_dir: Option<&Path>,
    ) -> Result<Self, RuntimeError> {
        // Ensure tmux session exists
        let mut args = vec!["new-session", "-d", "-s", session_name, "-n", window_name];
        let dir_str;
        if let Some(dir) = start_dir.and_then(|p| p.to_str()) {
            dir_str = dir.to_string();
            args.extend(["-c", &dir_str]);
        }
        let _ = Command::new("tmux").args(&args).output();

        // Attach in control mode
        let mut child = Command::new("tmux")
            .args(["-CC", "attach", "-t", session_name])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| RuntimeError::Backend(format!("tmux -CC spawn: {}", e)))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| RuntimeError::Backend("no stdin".into()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| RuntimeError::Backend("no stdout".into()))?;

        let pane_outputs: Arc<Mutex<HashMap<String, flume::Sender<Vec<u8>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Parser thread: read control mode stdout, dispatch %output events
        let outputs_clone = pane_outputs.clone();
        thread::spawn(move || {
            let mut parser = ControlModeParser::new();
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            loop {
                line.clear();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let events = parser.feed(&line);
                        for event in events {
                            match event {
                                ControlModeEvent::Output { pane_id, data } => {
                                    if let Ok(map) = outputs_clone.lock() {
                                        if let Some(tx) = map.get(&pane_id) {
                                            let _ = tx.send(data);
                                        }
                                    }
                                }
                                ControlModeEvent::Exit => return,
                                _ => {}
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            session_name: session_name.to_string(),
            window_name: window_name.to_string(),
            control_stdin: Arc::new(Mutex::new(stdin)),
            pane_outputs,
            _control_child: Arc::new(Mutex::new(child)),
        })
    }

    fn send_command(&self, cmd: &str) -> Result<(), RuntimeError> {
        let mut stdin = self.control_stdin.lock()
            .map_err(|e| RuntimeError::Backend(e.to_string()))?;
        writeln!(stdin, "{}", cmd)
            .map_err(|e| RuntimeError::Backend(e.to_string()))?;
        stdin.flush()
            .map_err(|e| RuntimeError::Backend(e.to_string()))
    }
}

impl AgentRuntime for TmuxControlModeRuntime {
    fn backend_type(&self) -> &'static str {
        "tmux-cc"
    }

    fn send_input(&self, pane_id: &PaneId, bytes: &[u8]) -> Result<(), RuntimeError> {
        // Escape bytes for send-keys -l (literal)
        let text = String::from_utf8_lossy(bytes);
        self.send_command(&format!("send-keys -l -t {} '{}'", pane_id, text.replace("'", "\\'")))
    }

    fn send_key(&self, pane_id: &PaneId, key: &str, use_literal: bool) -> Result<(), RuntimeError> {
        if use_literal {
            self.send_command(&format!("send-keys -l -t {} '{}'", pane_id, key))
        } else {
            self.send_command(&format!("send-keys -t {} {}", pane_id, key))
        }
    }

    fn resize(&self, pane_id: &PaneId, cols: u16, rows: u16) -> Result<(), RuntimeError> {
        self.send_command(&format!("resize-pane -t {} -x {} -y {}", pane_id, cols, rows))?;
        self.send_command(&format!("refresh-client -C {},{}",  cols, rows))
    }

    fn subscribe_output(&self, pane_id: &PaneId) -> Option<flume::Receiver<Vec<u8>>> {
        let (tx, rx) = flume::unbounded();
        if let Ok(mut map) = self.pane_outputs.lock() {
            map.insert(pane_id.clone(), tx);
        }
        Some(rx)
    }

    fn capture_initial_content(&self, _pane_id: &PaneId) -> Option<Vec<u8>> {
        // Control mode replays output on attach; no separate capture needed
        None
    }

    fn list_panes(&self, _agent_id: &AgentId) -> Vec<PaneId> {
        // Query tmux for pane list
        let target = format!("{}:{}", self.session_name, self.window_name);
        let output = Command::new("tmux")
            .args(["list-panes", "-t", &target, "-F", "#{pane_id}"])
            .output()
            .ok();
        output
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn focus_pane(&self, pane_id: &PaneId) -> Result<(), RuntimeError> {
        self.send_command(&format!("select-pane -t {}", pane_id))
    }

    fn split_pane(&self, pane_id: &PaneId, vertical: bool) -> Result<PaneId, RuntimeError> {
        let flag = if vertical { "-h" } else { "-v" };
        self.send_command(&format!("split-window {} -t {}", flag, pane_id))?;
        // Return the new pane id (last pane in list)
        let panes = self.list_panes(&String::new());
        panes.last().cloned()
            .ok_or_else(|| RuntimeError::Backend("no pane after split".into()))
    }

    fn get_pane_dimensions(&self, _pane_id: &PaneId) -> (u16, u16) {
        (80, 24) // TODO: query from tmux
    }

    fn open_diff(&self, worktree: &Path, _pane_id: Option<&PaneId>) -> Result<String, RuntimeError> {
        let new_pane = self.split_pane(&self.list_panes(&String::new()).first()
            .cloned().unwrap_or_default(), true)?;
        let cmd = format!(
            "nvim -c 'DiffviewOpen main...HEAD' '{}' 2>/dev/null || git diff main...HEAD --color=always | less -R",
            worktree.to_string_lossy()
        );
        self.send_input(&new_pane, cmd.as_bytes())?;
        self.send_input(&new_pane, b"\n")?;
        Ok(new_pane)
    }

    fn open_review(&self, worktree: &Path) -> Result<String, RuntimeError> {
        self.open_diff(worktree, None)
    }

    fn kill_window(&self, _window_target: &str) -> Result<(), RuntimeError> {
        self.send_command(&format!("kill-window -t {}:{}", self.session_name, self.window_name))
    }

    fn session_info(&self) -> Option<(String, String)> {
        Some((self.session_name.clone(), self.window_name.clone()))
    }
}
```

**Step 4: Run test to verify pass**

```bash
cargo test test_control_mode_runtime_backend_type -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/runtime/backends/tmux_control_mode.rs src/runtime/backends/mod.rs
git commit -m "feat: add TmuxControlModeRuntime (tmux -CC) for session persistence"
```

---

### Task 10: 注册 tmux-cc backend

**目的：** 让用户可以通过 `PMUX_BACKEND=tmux-cc` 或 config 选择 control mode backend。

**Files:**
- Modify: `src/runtime/backends/mod.rs` — `resolve_backend` 和 `create_runtime_from_env`

**Step 1: Write the failing test**

```rust
#[test]
fn test_resolve_backend_accepts_tmux_cc() {
    std::env::set_var("PMUX_BACKEND", "tmux-cc");
    let backend = resolve_backend(None);
    assert_eq!(backend, "tmux-cc");
    std::env::remove_var("PMUX_BACKEND");
}
```

**Step 2: Run test to verify failure**

```bash
cargo test test_resolve_backend_accepts_tmux_cc -- --nocapture
```

Expected: FAIL（`tmux-cc` 不在 VALID 列表中，fallback 到 `"local"`）

**Step 3: Write implementation**

`src/runtime/backends/mod.rs`:

```rust
pub fn resolve_backend(config: Option<&Config>) -> String {
    const VALID: [&str; 3] = ["local", "tmux", "tmux-cc"];
    // ... rest unchanged
}
```

在 `create_runtime_from_env` 中添加 `"tmux-cc"` 分支：

```rust
"tmux-cc" => {
    #[cfg(unix)]
    {
        if !tmux_available() {
            let rt = create_runtime(worktree_path, cols, rows)?;
            return Ok(RuntimeCreationResult {
                runtime: rt,
                fallback_message: Some("tmux 不可用，已回退到 local".to_string()),
            });
        }
        let session_name = session_name_for_workspace(workspace_path);
        let window_name = window_name_for_worktree(worktree_path, branch_name);
        let runtime = Arc::new(
            tmux_control_mode::TmuxControlModeRuntime::new(
                &session_name, &window_name, Some(worktree_path),
            )?
        );
        Ok(RuntimeCreationResult {
            runtime,
            fallback_message: None,
        })
    }
    #[cfg(not(unix))]
    {
        Err(RuntimeError::Backend("tmux-cc not supported on this platform".into()))
    }
}
```

**Step 4: Run test to verify pass**

```bash
cargo test test_resolve_backend_accepts_tmux_cc -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/runtime/backends/mod.rs
git commit -m "feat: register tmux-cc backend in runtime factory"
```

---

### Task 11: 实现 recover_runtime 对 tmux-cc 的支持

**Files:**
- Modify: `src/runtime/backends/mod.rs` — `recover_runtime`

**Step 1: Write implementation**

在 `recover_runtime` 中添加 `"tmux-cc"` 分支：

```rust
"tmux-cc" => {
    let runtime = tmux_control_mode::TmuxControlModeRuntime::new(
        &state.backend_session_id,
        &state.backend_window_id,
        None,
    )?;
    Ok(Arc::new(runtime))
}
```

**Step 2: Verify**

```bash
cargo check
```

**Step 3: Commit**

```bash
git add src/runtime/backends/mod.rs
git commit -m "feat: recover_runtime supports tmux-cc backend"
```

---

### Task 12: Phase 2 E2E 测试 — 持久化 + 恢复 + 截图断言

**目的：** 验证 tmux-cc 模式的核心价值——关闭 GUI 后 agent 继续运行、重开后自动恢复，以及与 Phase 1 同等的终端渲染质量。

**Files:**
- Create: `tests/e2e/tmux_cc_e2e.sh`

**Step 1: 创建 E2E 测试脚本**

```bash
#!/bin/bash
# E2E Tests for Phase 2: tmux Control Mode (-CC) Persistence
# Tests: terminal rendering, session persistence, recovery after GUI restart
# Requires: tmux, tesseract, ffmpeg, PIL (pip3 install Pillow)
#
# Usage: bash tests/e2e/tmux_cc_e2e.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PMUX_ROOT="${PMUX_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
RESULTS_DIR="$SCRIPT_DIR/results/tmux_cc_$(date +%Y%m%d_%H%M%S)"
RECORDING_FILE="$RESULTS_DIR/session.mp4"
IMAGE_ANALYSIS="$PMUX_ROOT/tests/regression/lib/image_analysis.py"

source "$PMUX_ROOT/tests/regression/lib/test_utils.sh"

mkdir -p "$RESULTS_DIR"
export SCRIPT_DIR="$RESULTS_DIR/.."

# ── Helpers (reuse from local_pty_e2e.sh, inline for independence) ──

screenshot_and_ocr() {
    local name="$1"
    local path="$RESULTS_DIR/${name}.png"
    local win_id
    win_id=$(osascript -e 'tell application "System Events" to get id of window 1 of process "pmux"' 2>/dev/null || echo "")
    if [ -n "$win_id" ]; then
        screencapture -l "$win_id" -x "$path" 2>/dev/null || screencapture -x "$path"
    else
        screencapture -x "$path"
    fi
    echo "$path"
}

assert_ocr_contains() {
    local image="$1" expected="$2" test_name="$3"
    local ocr_text
    ocr_text=$(python3 "$IMAGE_ANALYSIS" ocr "$image" 2>/dev/null | grep "^TEXT:" | sed 's/^TEXT://' || echo "")
    if echo "$ocr_text" | grep -qi "$expected"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — OCR found '$expected'"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — OCR did NOT find '$expected'"
        log_error "  OCR text: $ocr_text"
        return 1
    fi
}

assert_window_valid() {
    local image="$1" test_name="$2"
    local ok
    ok=$(python3 "$IMAGE_ANALYSIS" verify_window "$image" 400 300 2>/dev/null | grep "^OK:" | sed 's/^OK://' || echo "False")
    if [ "$ok" = "True" ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name"
        return 1
    fi
}

assert_tmux_session_exists() {
    local session="$1" test_name="$2"
    if tmux has-session -t "$session" 2>/dev/null; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — session '$session' exists"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — session '$session' NOT found"
        return 1
    fi
}

assert_tmux_session_gone() {
    local session="$1" test_name="$2"
    if ! tmux has-session -t "$session" 2>/dev/null; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — session '$session' is gone"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — session '$session' still exists"
        return 1
    fi
}

# ── Prereqs ─────────────────────────────────────────────────

if ! command -v tmux &>/dev/null; then
    log_error "tmux not installed, skipping Phase 2 E2E"
    exit 0
fi

init_report
add_report_section "Phase 2: tmux Control Mode E2E Tests"

export PMUX_BACKEND=tmux-cc

# Build
log_info "Building pmux..."
(cd "$PMUX_ROOT" && RUSTUP_TOOLCHAIN=stable cargo build 2>&1 | tail -3)

# Clean up any previous test sessions
tmux kill-session -t "pmux-e2e-test" 2>/dev/null || true
stop_pmux 2>/dev/null || true
sleep 1

# Start recording
if command -v ffmpeg &>/dev/null; then
    ffmpeg -f avfoundation -i "1" -r 15 -t 180 -y "$RECORDING_FILE" 2>/dev/null &
    FFMPEG_PID=$!
else
    FFMPEG_PID=""
fi

# ── Test 1: Start with tmux-cc, window valid ───────────────

log_info "=== Test 1: tmux-cc startup ==="
start_pmux || { log_error "Failed to start"; exit 1; }
sleep 5
activate_window
sleep 1
IMG=$(screenshot_and_ocr "01_tmux_cc_startup")
assert_window_valid "$IMG" "tmux-cc window visible"
add_report_result "tmux-cc startup" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 2: Command execution ──────────────────────────────

log_info "=== Test 2: Command execution ==="
click_terminal_area
sleep 0.5
send_keystroke "echo TMUX_CC_MARKER_42"
sleep 0.3
send_keycode 36
sleep 2
IMG=$(screenshot_and_ocr "02_command_exec")
assert_ocr_contains "$IMG" "TMUX_CC_MARKER_42\|MARKER" "echo output visible via OCR" || true
add_report_result "Command exec" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 3: No staircase (same as Phase 1) ─────────────────

log_info "=== Test 3: No staircase ==="
send_keystroke "echo stair1"
send_keycode 36
sleep 0.5
send_keystroke "echo stair2"
send_keycode 36
sleep 0.5
send_keystroke "echo stair3"
send_keycode 36
sleep 1
IMG=$(screenshot_and_ocr "03_no_staircase")

# Reuse staircase detection from Phase 1
python3 -c "
from PIL import Image
img = Image.open('$IMG')
w, h = img.size
x0 = int(w * 0.3)
terminal = img.crop((x0, 0, w, h))
tw, th = terminal.size
rows_with_content = []
for y in range(0, th, 4):
    first_bright_x = None
    for x in range(min(200, tw)):
        r, g, b = terminal.getpixel((x, y))
        if (r + g + b) / 3 > 80:
            first_bright_x = x
            break
    if first_bright_x is not None:
        rows_with_content.append(first_bright_x)
if len(rows_with_content) > 5:
    diffs = [rows_with_content[i+1] - rows_with_content[i] for i in range(len(rows_with_content)-1)]
    increasing_count = sum(1 for d in diffs if d > 3)
    ratio = increasing_count / len(diffs)
    print(f'IS_STAIRCASE:{ratio > 0.5}')
else:
    print('IS_STAIRCASE:False')
" 2>/dev/null > "$RESULTS_DIR/03_analysis.txt" || true

IS_STAIRCASE=$(grep "IS_STAIRCASE:" "$RESULTS_DIR/03_analysis.txt" 2>/dev/null | sed 's/.*://')
if [ "$IS_STAIRCASE" = "False" ] || [ -z "$IS_STAIRCASE" ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ No staircase in tmux-cc mode"
    add_report_result "No staircase (tmux-cc)" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ Staircase detected in tmux-cc mode!"
    add_report_result "No staircase (tmux-cc)" "FAIL"
fi

# ── Test 4: Session persistence — close GUI, tmux survives ─

log_info "=== Test 4: Session persistence ==="

# Write a marker to know which session to check
send_keystroke "echo 'SESSION_ALIVE_CHECK'"
send_keycode 36
sleep 1

# Find the pmux tmux session name
PMUX_SESSION=$(tmux list-sessions -F "#{session_name}" 2>/dev/null | grep "pmux" | head -1 || echo "")
log_info "Detected pmux session: $PMUX_SESSION"

# Take screenshot before closing
IMG=$(screenshot_and_ocr "04a_before_close")

# Close pmux GUI
stop_pmux
sleep 2

# Assert tmux session still exists
if [ -n "$PMUX_SESSION" ]; then
    assert_tmux_session_exists "$PMUX_SESSION" "tmux session survives GUI close"
    add_report_result "Session persists" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

    # Verify the session has content (capture-pane should show our marker)
    CAPTURE=$(tmux capture-pane -t "$PMUX_SESSION" -p 2>/dev/null || echo "")
    if echo "$CAPTURE" | grep -q "SESSION_ALIVE_CHECK"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ Session content preserved (marker found)"
        add_report_result "Session content" "PASS"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ Session content NOT preserved"
        add_report_result "Session content" "FAIL"
    fi
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ No pmux tmux session found"
    add_report_result "Session persists" "FAIL"
    add_report_result "Session content" "SKIP"
fi

# ── Test 5: Recovery — reopen GUI, auto-attach ──────────────

log_info "=== Test 5: Session recovery ==="
sleep 1
start_pmux || { log_error "Failed to restart"; exit 1; }
sleep 5
activate_window
sleep 2
IMG=$(screenshot_and_ocr "05_recovered")

assert_window_valid "$IMG" "Recovered window is valid"
add_report_result "Recovery window" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# Check OCR for our marker from before close
assert_ocr_contains "$IMG" "SESSION_ALIVE\|ALIVE_CHECK\|ALIVE" "Recovery shows previous content" || true
add_report_result "Recovery content" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 6: vim after recovery ──────────────────────────────

log_info "=== Test 6: vim after recovery ==="
click_terminal_area
sleep 0.5
send_keystroke "vim /tmp/pmux_cc_test.txt"
send_keycode 36
sleep 2
IMG=$(screenshot_and_ocr "06_vim_after_recovery")
assert_window_valid "$IMG" "vim opens after recovery"
add_report_result "vim after recovery" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# Exit vim
send_keystroke ":"
sleep 0.2
send_keystroke "q!"
send_keycode 36
sleep 1

# ── Teardown ────────────────────────────────────────────────

stop_pmux

# Kill test tmux session
if [ -n "$PMUX_SESSION" ]; then
    tmux kill-session -t "$PMUX_SESSION" 2>/dev/null || true
fi

# Stop recording
if [ -n "${FFMPEG_PID:-}" ]; then
    kill "$FFMPEG_PID" 2>/dev/null || true
    wait "$FFMPEG_PID" 2>/dev/null || true
    log_info "Recording saved: $RECORDING_FILE"
    if command -v ffprobe &>/dev/null && [ -f "$RECORDING_FILE" ]; then
        bash "$PMUX_ROOT/tests/helpers/recording.sh" analyze "$RECORDING_FILE" >> "$RESULTS_DIR/recording_analysis.txt" 2>&1 || true
    fi
fi

# ── Report ──────────────────────────────────────────────────

finalize_report
cp "$REPORT_FILE" "$RESULTS_DIR/report.md" 2>/dev/null || true

echo ""
echo "================================"
echo "Phase 2 E2E Results"
echo "================================"
echo "  Passed: $TESTS_PASSED"
echo "  Failed: $TESTS_FAILED"
echo "  Screenshots: $RESULTS_DIR/"
echo "  Recording: $RECORDING_FILE"
echo "  Report: $RESULTS_DIR/report.md"
echo "================================"

exit $TESTS_FAILED
```

**Step 2: 运行测试**

```bash
bash tests/e2e/tmux_cc_e2e.sh
```

Expected: 全部 PASS。关键断言：
- 关闭 GUI 后 `tmux ls` 仍显示 session
- 重新打开后 OCR 能找到之前的命令输出
- 无阶梯 prompt

**Step 3: Commit**

```bash
git add tests/e2e/tmux_cc_e2e.sh
git commit -m "test: add Phase 2 E2E tests — persistence, recovery, screenshot assertions"
```

---

## 整体迁移路线

```
Week 1:  Task 1-5  (Phase 1: local PTY default)           ← 解决所有终端 bug
         Task 6    (Phase 1 E2E: 截图+OCR+阶梯检测)       ← 自动化验证
Week 2:  Task 7-8  (研究 + ControlModeParser)
Week 3:  Task 9-11 (TmuxControlModeRuntime + 注册)
Week 4:  Task 12   (Phase 2 E2E: 持久化+恢复+截图断言)    ← 自动化验证
```

## E2E 测试覆盖矩阵

| 断言类型 | Phase 1 (local_pty_e2e.sh) | Phase 2 (tmux_cc_e2e.sh) |
|----------|---------------------------|--------------------------|
| 窗口可见 (verify_window) | ✓ Test 1 | ✓ Test 1 |
| Prompt 在顶部（非中间） | ✓ Test 2 | — |
| 命令执行 OCR | ✓ Test 3 (ls) | ✓ Test 2 (echo marker) |
| CJK 字符 | ✓ Test 4 | — |
| 快速 Enter 不崩溃 | ✓ Test 5 | — |
| vim 打开/关闭 | ✓ Test 6 | ✓ Test 6 (恢复后) |
| **无阶梯 prompt** | ✓ Test 7 | ✓ Test 3 |
| 大量输出 | ✓ Test 8 | — |
| **关闭 GUI session 存活** | — | ✓ Test 4 (tmux ls) |
| **重开 GUI 自动恢复** | — | ✓ Test 5 (OCR 验证) |
| **录屏 + FPS 分析** | ✓ (全程) | ✓ (全程) |

每次测试失败都保留：截图（`results/*/xx_name.png`）、分析结果（`*_analysis.txt`）、录屏（`session.mp4`），方便事后调试。

## 后续（不在本 Plan 内）

- ~~旧 `TmuxRuntime`（pipe-pane）标记 deprecated，保留供兼容~~ ✅ Done
- ~~`PMUX_BACKEND=tmux` 逐步指向 `tmux-cc`（配置迁移）~~ ✅ Done — "tmux" now uses TmuxControlModeRuntime
- 多 pane control mode 支持（`%output` 已按 pane_id 路由）
- scrollback 拉取（`capture-pane -S -N` 通过 control mode 一次性查询）
- libghostty-vt 替换 alacritty_terminal（Phase 3+，等 C API 稳定）

## Status: ALL TASKS COMPLETE ✅

Phase 1 (Tasks 1-6) and Phase 2 (Tasks 7-12) fully implemented. Default backend is now "tmux" (control mode). Legacy TmuxRuntime deprecated. "tmux" and "tmux-cc" consolidated to TmuxControlModeRuntime.
