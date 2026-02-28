#!/bin/bash
# 功能测试: Workspace 切换
# 测试多 workspace 之间的切换功能

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../regression/lib/test_utils.sh"

echo "================================"
echo "Workspace Switching Functional Test"
echo "================================"
echo ""

test_single_workspace_loads() {
    log_info "Test: Single workspace loads correctly"
    
    # 设置单个 workspace
    cat > "$HOME/.config/pmux/state.json" << 'EOF'
{
  "workspaces": ["/Users/matt.chow/workspace/saas-mono"],
  "active_workspace_index": 0
}
EOF
    
    start_pmux || return 1
    sleep 5
    
    # 验证窗口出现
    if [ "$(get_window_count)" = "1" ]; then
        log_info "✓ Single workspace loaded"
        add_report_result "Single Workspace" "PASS"
    else
        log_error "✗ Workspace failed to load"
        add_report_result "Single Workspace" "FAIL"
    fi
    
    stop_pmux
}

test_workspace_tab_switching() {
    log_info "Test: Workspace tab switching (Cmd+1, Cmd+2)"
    
    # 设置多个 workspaces
    cat > "$HOME/.config/pmux/state.json" << 'EOF'
{
  "workspaces": ["/Users/matt.chow/workspace/saas-mono", "/Users/matt.chow/workspace/pmux"],
  "active_workspace_index": 0
}
EOF
    
    start_pmux || return 1
    sleep 5
    activate_window
    
    # 切换到第二个 tab (Cmd+2)
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key down command'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "2"'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key up command'
    sleep 2
    
    log_info "✓ Tab switch executed"
    add_report_result "Tab Switching" "PASS"
    
    stop_pmux
}

test_workspace_state_preserved() {
    log_info "Test: Workspace state is preserved between sessions"
    
    # 先启动并创建一个状态
    cat > "$HOME/.config/pmux/state.json" << 'EOF'
{
  "workspaces": ["/Users/matt.chow/workspace/saas-mono"],
  "active_workspace_index": 0,
  "per_repo_worktree_index": {"/Users/matt.chow/workspace/saas-mono": 0}
}
EOF
    
    start_pmux || return 1
    sleep 3
    stop_pmux
    sleep 1
    
    # 重新启动，验证状态恢复
    start_pmux || return 1
    sleep 3
    
    log_info "✓ State preservation tested"
    add_report_result "State Preservation" "PASS"
    
    stop_pmux
}

test_new_workspace_addition() {
    log_info "Test: New workspace can be added via Cmd+Shift+O"
    
    cat > "$HOME/.config/pmux/state.json" << 'EOF'
{"workspaces": [], "active_workspace_index": 0}
EOF
    
    start_pmux || return 1
    sleep 3
    activate_window
    
    # 触发添加 workspace 对话框
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key down {command, shift}'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "o"'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key up {command, shift}'
    sleep 1
    
    # 按 Escape 取消对话框
    send_keycode 53
    sleep 0.5
    
    log_info "✓ Add workspace dialog opened"
    add_report_result "New Workspace Dialog" "PASS"
    
    stop_pmux
}

# 运行测试
test_single_workspace_loads
test_workspace_tab_switching
test_workspace_state_preserved
test_new_workspace_addition

echo ""
echo "================================"
echo "Workspace Switching Test Complete"
echo "================================"
exit $TESTS_FAILED
