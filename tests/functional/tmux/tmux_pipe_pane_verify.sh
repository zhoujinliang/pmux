#!/bin/bash
# 验证 pmux 的 pipe-pane 显示：启动 pmux，用 tmux send-keys 直接发命令到 session，
# 截屏 OCR 验证 pmux 的 terminal 是否显示 README。
# 若通过：pipe-pane 正常，问题在 pmux 键盘输入路径。
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../regression/lib/test_utils.sh"

echo "========================================"
echo "Tmux Pipe-Pane Display Verification"
echo "========================================"

command -v tmux &>/dev/null || { echo "skip: no tmux"; exit 0; }
command -v tesseract &>/dev/null || { echo "skip: no tesseract"; exit 0; }

(cd "$PMUX_ROOT" && RUSTUP_TOOLCHAIN=stable cargo build 2>&1) || exit 1

TEST_REPO="/tmp/pmux-pipe-verify-$$"
mkdir -p "$TEST_REPO"
cd "$TEST_REPO"
git init -q
git config user.email "t@t.local"
git config user.name "T"
touch README
git add README
git commit -q -m "init"
cd - >/dev/null

CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/Library/Application Support}/pmux"
mkdir -p "$CONFIG_DIR"
CONFIG_FILE="$CONFIG_DIR/config.json"
[ -f "$CONFIG_FILE" ] && cp "$CONFIG_FILE" "$CONFIG_FILE.bak.$$"
cat > "$CONFIG_FILE" << EOF
{"workspace_paths": ["$TEST_REPO"], "active_workspace_index": 0, "backend": "local"}
EOF

cleanup() {
    rm -rf "$TEST_REPO"
    [ -f "${CONFIG_FILE}.bak.$$" ] && mv "${CONFIG_FILE}.bak.$$" "$CONFIG_FILE"
    stop_pmux 2>/dev/null || true
}
trap cleanup EXIT

tmux kill-server 2>/dev/null || true
sleep 0.5

SESSION="pmux-pmux-pipe-verify-$$"
export PMUX_BACKEND=tmux
start_pmux || exit 1
sleep 8

# 用 tmux send-keys 直接发到 pmux 创建的 session（绕过 pmux 键盘）
tmux send-keys -t "$SESSION:main" "cd $TEST_REPO" Enter
sleep 1
tmux send-keys -t "$SESSION:main" "ls" Enter
sleep 2

activate_window
sleep 0.5
SCREENSHOT=$(take_screenshot "tmux_pipe_verify")
[ ! -f "$SCREENSHOT" ] && { echo "FAIL: screenshot failed"; exit 1; }

OCR_RESULT=$(python3 "$SCRIPT_DIR/../../regression/lib/image_analysis.py" ocr "$SCREENSHOT" 2>/dev/null) || { echo "FAIL: OCR"; exit 1; }
OCR_TEXT=$(echo "$OCR_RESULT" | grep "^TEXT:" | cut -d':' -f2- | tr -d '\n')

if ! echo "$OCR_TEXT" | grep -qi "README"; then
    echo "FAIL: README not in screenshot (pipe-pane or display issue)"
    echo "OCR: ${OCR_TEXT:0:300}"
    exit 1
fi

echo "PASS: pipe-pane displays tmux output"
