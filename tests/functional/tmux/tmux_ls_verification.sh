#!/bin/bash
# 验证: PMUX_BACKEND=tmux 时 pmux 创建 session、pipe-pane 显示、cd/pwd/ls 可执行
#
# 验证方式: 启动 pmux，用 tmux send-keys 发 cd/pwd/ls，tmux capture-pane 断言路径和 README
# 依赖: tmux

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../regression/lib/test_utils.sh"

echo "================================"
echo "Tmux Backend - ls Verification"
echo "================================"

# 检查 tmux
if ! command -v tmux &> /dev/null; then
    log_error "tmux not installed - skip"
    exit 0
fi

# 构建 pmux
log_info "Building pmux..."
(cd "$PMUX_ROOT" && RUSTUP_TOOLCHAIN=stable cargo build 2>&1) || {
    log_error "Build failed"
    exit 1
}

# 创建临时 git 仓库
TEST_REPO="/tmp/pmux-tmux-verify-$$"
mkdir -p "$TEST_REPO"
cd "$TEST_REPO"
git init -q
git config user.email "t@t.local"
git config user.name "T"
touch README
git add README
git commit -q -m "init"
cd - > /dev/null

RESULT_FILE="${PMUX_ROOT:-/Users/matt.chow/workspace/pmux}/tests/functional/tmux/tmux_ls_verify_result.txt"

cleanup() {
    rm -rf "$TEST_REPO"
    [ -n "$CONFIG_FILE" ] && [ -f "${CONFIG_FILE}.bak.$$" ] && mv "${CONFIG_FILE}.bak.$$" "$CONFIG_FILE"
    stop_pmux 2>/dev/null || true
}
trap cleanup EXIT

write_result() {
    local status="$1"
    local detail="${2:-}"
    mkdir -p "$(dirname "$RESULT_FILE")"
    echo "$status" > "$RESULT_FILE"
    echo "timestamp: $(date -Iseconds 2>/dev/null || date)" >> "$RESULT_FILE"
    [ -n "$detail" ] && echo "detail: $detail" >> "$RESULT_FILE"
}

# 预置 config
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/Library/Application Support}/pmux"
mkdir -p "$CONFIG_DIR"
CONFIG_FILE="$CONFIG_DIR/config.json"
[ -f "$CONFIG_FILE" ] && cp "$CONFIG_FILE" "$CONFIG_FILE.bak.$$"
cat > "$CONFIG_FILE" << EOF
{
  "workspace_paths": ["$TEST_REPO"],
  "active_workspace_index": 0,
  "backend": "local"
}
EOF

SESSION="pmux-pmux-tmux-verify-$$"
# 清理残留 session，避免 "ls" 等旧内容、光标错位
tmux kill-server 2>/dev/null || true
sleep 0.5

# 启动 pmux
export PMUX_BACKEND=tmux
log_info "Starting pmux with PMUX_BACKEND=tmux..."
start_pmux || exit 1

sleep 8

# 用 tmux send-keys 直接发命令（osascript 焦点不稳定），验证 pipe-pane 显示
log_info "Sending 'cd', 'pwd', 'ls' via tmux send-keys..."
tmux send-keys -t "$SESSION:main" "cd $TEST_REPO" Enter
sleep 1
tmux send-keys -t "$SESSION:main" "pwd" Enter
sleep 1
tmux send-keys -t "$SESSION:main" "ls" Enter
sleep 3

activate_window
sleep 0.5

# 检查 pmux 未崩溃
if ! ps -p $PMUX_PID > /dev/null 2>&1; then
    log_error "FAIL: pmux crashed"
    write_result "FAIL" "pmux crashed"
    exit 1
fi

# 检查 tmux session 存在
if ! tmux list-sessions 2>/dev/null | grep -q "$SESSION"; then
    log_error "FAIL: Tmux session not created (expected: $SESSION)"
    tmux list-sessions 2>/dev/null || true
    write_result "FAIL" "tmux session not created"
    exit 1
fi

# 用 tmux capture-pane 验证（可靠，不依赖 OCR）
CAPTURED=$(tmux capture-pane -t "$SESSION:main" -p | tr '\n' ' ')
log_info "Pane content (excerpt): ${CAPTURED:0:200}..."

# 断言 1: 必须包含路径
if ! echo "$CAPTURED" | grep -q "pmux-tmux-verify"; then
    log_error "FAIL: pane does not show path (expected 'pmux-tmux-verify')"
    write_result "FAIL" "pane does not show path"
    exit 1
fi

# 断言 2: 必须包含 README (ls 输出)
if ! echo "$CAPTURED" | grep -qi "README"; then
    log_error "FAIL: pane does not show 'README' from ls output"
    log_info "Captured: $CAPTURED"
    write_result "FAIL" "pane does not show README"
    exit 1
fi

log_info "✓ Verification passed: tmux pane shows path and README"
write_result "PASS" "tmux capture-pane verified"
exit 0
