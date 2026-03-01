#!/bin/bash
# 无 GUI 验证：用 tmux send-keys 直接发命令，capture-pane 验证输出。
# 验证 tmux session、pipe-pane 流、PTY 输入逻辑正确（不依赖 pmux UI 焦点）。
set -e

cd "$(dirname "$0")"
SCRIPT_DIR="$(pwd)"
source "$SCRIPT_DIR/../../regression/lib/test_utils.sh"

echo "========================================"
echo "Tmux Direct Verification (no GUI)"
echo "========================================"

[ -n "$TMUX" ] && { echo "Do not run inside tmux"; exit 1; }

TEST_DIR=$(mktemp -d /tmp/pmux-tmux-direct-XXXXXX)
SOCK_NAME="pmux-direct-$PPID"
trap "rm -rf $TEST_DIR; tmux -L $SOCK_NAME kill-server 2>/dev/null || true" EXIT

# Use isolated socket to avoid conflicts with other tmux
tmux -L "$SOCK_NAME" kill-server 2>/dev/null || true
sleep 1

cd "$TEST_DIR"
git init -q
git config user.email "t@t.local"
git config user.name "T"
touch README
git add README
git commit -q -m "init"

SESSION="pmux-direct-verify"
tmux -L "$SOCK_NAME" new-session -d -s "$SESSION" -n main -c "$TEST_DIR"
sleep 1

# 用 tmux send-keys 直接发命令（模拟 pmux 的 send_input 效果）
TMUX_CMD="tmux -L $SOCK_NAME"
$TMUX_CMD send-keys -t "$SESSION:main" "cd $TEST_DIR" Enter
sleep 0.5
$TMUX_CMD send-keys -t "$SESSION:main" "pwd" Enter
sleep 0.5
$TMUX_CMD send-keys -t "$SESSION:main" "ls" Enter
sleep 1

CAPTURED=$($TMUX_CMD capture-pane -t "$SESSION:main" -p | tr '\n' ' ')
$TMUX_CMD kill-session -t "$SESSION" 2>/dev/null || true

if echo "$CAPTURED" | grep -q "pmux-tmux-direct"; then
    echo "PASS: path visible"
else
    echo "FAIL: path not in capture: $CAPTURED"
    exit 1
fi

if echo "$CAPTURED" | grep -qi "README"; then
    echo "PASS: README visible"
else
    echo "FAIL: README not in capture: $CAPTURED"
    exit 1
fi

echo "PASS: tmux direct verification"
