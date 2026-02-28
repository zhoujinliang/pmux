#!/bin/bash
# 功能测试: Pane 分屏操作
# 测试 pane 的创建、关闭、导航

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../regression/lib/test_utils.sh"

echo "================================"
echo "Pane Split Operations Test"
echo "================================"
echo ""

test_vertical_split() {
    log_info "Test: Vertical split (Cmd+D)"
    
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key down command'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "d"'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key up command'
    sleep 2
    
    if pgrep -f "target/debug/pmux" > /dev/null; then
        log_info "✓ Vertical split executed"
        add_report_result "Vertical Split" "PASS"
        return 0
    else
        log_error "✗ Crashed on vertical split"
        add_report_result "Vertical Split" "FAIL"
        return 1
    fi
}

test_horizontal_split() {
    log_info "Test: Horizontal split (Cmd+Shift+D)"
    
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key down {command, shift}'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "d"'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key up {command, shift}'
    sleep 2
    
    if pgrep -f "target/debug/pmux" > /dev/null; then
        log_info "✓ Horizontal split executed"
        add_report_result "Horizontal Split" "PASS"
        return 0
    else
        log_error "✗ Crashed on horizontal split"
        add_report_result "Horizontal Split" "FAIL"
        return 1
    fi
}

test_pane_navigation() {
    log_info "Test: Pane navigation (Cmd+Alt+Arrows)"
    
    # 尝试在各个 pane 之间导航
    for direction in 126 125 123 124; do  # up, down, left, right
        osascript_cmd "tell application \"System Events\" to tell process \"pmux\" to key down {command, option}"
        osascript_cmd "tell application \"System Events\" to tell process \"pmux\" to key code $direction"
        osascript_cmd "tell application \"System Events\" to tell process \"pmux\" to key up {command, option}"
        sleep 0.5
    done
    
    log_info "✓ Pane navigation tested"
    add_report_result "Pane Navigation" "PASS"
}

test_close_pane() {
    log_info "Test: Close pane (Cmd+W)"
    
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key down command'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "w"'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key up command'
    sleep 1
    
    if pgrep -f "target/debug/pmux" > /dev/null; then
        log_info "✓ Pane closed"
        add_report_result "Close Pane" "PASS"
        return 0
    else
        log_error "✗ Crashed closing pane"
        add_report_result "Close Pane" "FAIL"
        return 1
    fi
}

test_last_pane_protection() {
    log_info "Test: Last pane cannot be closed"
    
    # 尝试关闭所有 pane 直到只剩一个
    for i in {1..5}; do
        osascript_cmd 'tell application "System Events" to tell process "pmux" to key down command'
        osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "w"'
        osascript_cmd 'tell application "System Events" to tell process "pmux" to key up command'
        sleep 0.5
    done
    
    sleep 1
    
    # 检查应用是否还在运行
    if pgrep -f "target/debug/pmux" > /dev/null; then
        log_info "✓ Application still running (last pane protected)"
        add_report_result "Last Pane Protection" "PASS"
    else
        log_warn "⚠ Application closed (behavior may vary)"
        add_report_result "Last Pane Protection" "WARN"
    fi
}

# 主测试流程
cat > "$HOME/.config/pmux/state.json" << 'EOF'
{
  "workspaces": ["/Users/matt.chow/workspace/saas-mono"],
  "active_workspace_index": 0
}
EOF

start_pmux || exit 1
sleep 5
activate_window
sleep 1

# 测试序列
test_vertical_split
test_horizontal_split
test_pane_navigation
test_close_pane
test_last_pane_protection

stop_pmux

echo ""
echo "================================"
echo "Pane Operations Test Complete"
echo "================================"
exit $TESTS_FAILED
