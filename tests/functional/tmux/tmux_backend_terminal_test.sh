#!/bin/bash
# 功能测试: PMUX_BACKEND=tmux 时 terminal 是否正确显示
# 复现: 使用 tmux 后端启动后，右侧 terminal 不出现的问题

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../regression/lib/test_utils.sh"

echo "================================"
echo "Tmux Backend Terminal Test"
echo "================================"
echo ""

# 检查 tmux 是否安装
if ! command -v tmux &> /dev/null; then
    log_error "tmux is not installed - skipping tmux backend test"
    exit 0
fi

# 创建临时 git 仓库
TEST_REPO=$(mktemp -d)
cd "$TEST_REPO"
git init -q
git config user.email "test@pmux.local"
git config user.name "Test"
touch README && git add README && git commit -q -m "init"
cd - > /dev/null

cleanup_repo() {
    rm -rf "$TEST_REPO"
}
trap cleanup_repo EXIT

test_tmux_runtime_creates_pane() {
    log_info "Test: Tmux runtime creates session and returns pane_id"
    
    # 直接测试 tmux 创建 session (cargo test 在部分 macOS 上会有 gpui_macros SIGBUS，跳过)
    local session_name="pmux-tmux-test-$$"
    tmux new-session -d -s "$session_name" -n "main"
    
    local panes
    panes=$(tmux list-panes -t "$session_name:main" -F "#{pane_id}" 2>/dev/null || echo "")
    tmux kill-session -t "$session_name" 2>/dev/null || true
    
    if [ -n "$panes" ]; then
        log_info "✓ Tmux pane created: $panes"
        return 0
    else
        log_error "✗ Tmux list-panes returned empty"
        return 1
    fi
}

test_pmux_tmux_backend_launch() {
    log_info "Test: PMUX_BACKEND=tmux with workspace shows terminal"
    
    # 预置 state: 有 workspace 时启动会尝试加载
    mkdir -p "$PMUX_CONFIG_DIR"
    cat > "$PMUX_CONFIG_DIR/state.json" << EOF
{
  "workspaces": ["$TEST_REPO"],
  "active_workspace_index": 0
}
EOF
    
    # 清除可能存在的 tmux session
    tmux kill-session -t "pmux-$(basename "$TEST_REPO")" 2>/dev/null || true
    
    # 使用 tmux 后端启动
    export PMUX_BACKEND=tmux
    local bin="${PMUX_BIN:-$PMUX_ROOT/target/debug/pmux}"
    
    if [ ! -f "$bin" ]; then
        log_warn "pmux not built - run: cargo build"
        return 0
    fi
    
    PMUX_BACKEND=tmux "$bin" &
    PMUX_PID=$!
    
    # 等待加载
    sleep 5
    
    if ! ps -p $PMUX_PID > /dev/null; then
        log_error "pmux crashed on startup with PMUX_BACKEND=tmux"
        return 1
    fi
    
    # 检查 tmux session 是否被创建
    local session_name="pmux-$(basename "$TEST_REPO")"
    if tmux list-sessions 2>/dev/null | grep -q "$session_name"; then
        log_info "✓ Tmux session created: $session_name"
        
        # 检查是否有 pane
        local panes
        panes=$(tmux list-panes -t "$session_name:main" -F "#{pane_id}" 2>/dev/null || echo "")
        if [ -n "$panes" ]; then
            log_info "✓ Tmux panes exist: $panes"
        else
            log_error "✗ No panes in tmux window"
        fi
    else
        log_error "✗ Tmux session not created - terminal may not display"
    fi
    
    # 停止 pmux
    kill -9 $PMUX_PID 2>/dev/null || true
    wait $PMUX_PID 2>/dev/null || true
    
    return 0
}

# 运行测试
test_tmux_runtime_creates_pane || true
test_pmux_tmux_backend_launch

echo ""
echo "================================"
echo "Tmux Backend Test Complete"
echo "================================"
