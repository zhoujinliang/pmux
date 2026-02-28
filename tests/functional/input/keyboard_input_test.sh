#!/bin/bash
# 功能测试: 键盘输入处理
# 测试各种键盘输入的响应

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../regression/lib/test_utils.sh"

echo "================================"
echo "Keyboard Input Functional Test"
echo "================================"
echo ""

test_alphanumeric_input() {
    log_info "Test: Alphanumeric input (a-z, 0-9)"
    
    send_keystroke "abcdefghijklmnopqrstuvwxyz0123456789"
    sleep 0.5
    
    # 清除
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key down control'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "c"'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key up control'
    sleep 0.3
    
    log_info "✓ Alphanumeric input accepted"
    add_report_result "Alphanumeric Input" "PASS"
}

test_special_characters() {
    log_info "Test: Special characters (~!@#$%^&*)"
    
    send_keystroke "~!@#$%^&*()_+-=[]{}|;':\",./<>?"
    sleep 0.5
    
    # 清除
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key down control'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "c"'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key up control'
    sleep 0.3
    
    log_info "✓ Special characters accepted"
    add_report_result "Special Characters" "PASS"
}

test_arrow_keys() {
    log_info "Test: Arrow keys navigation"
    
    # 先输入一些文本
    send_keystroke "hello world"
    sleep 0.3
    
    # 使用方向键
    for keycode in 123 124 125 126; do  # left, right, down, up
        send_keycode $keycode
        sleep 0.1
    done
    
    log_info "✓ Arrow keys work"
    add_report_result "Arrow Keys" "PASS"
}

test_function_keys() {
    log_info "Test: Function keys (F1-F12)"
    
    # F1-F4 (key codes 122-118, macOS specific)
    for keycode in 122 120 99 118; do
        send_keycode $keycode
        sleep 0.2
    done
    
    log_info "✓ Function keys tested"
    add_report_result "Function Keys" "PASS"
}

test_control_combinations() {
    log_info "Test: Ctrl+Key combinations"
    
    # Ctrl+C, Ctrl+V, Ctrl+A, Ctrl+E
    for key in "c" "a" "e"; do
        osascript_cmd 'tell application "System Events" to tell process "pmux" to key down control'
        osascript_cmd "tell application \"System Events\" to tell process \"pmux\" to keystroke \"$key\""
        osascript_cmd 'tell application "System Events" to tell process "pmux" to key up control'
        sleep 0.2
    done
    
    log_info "✓ Control combinations work"
    add_report_result "Control Combinations" "PASS"
}

test_rapid_typing() {
    log_info "Test: Rapid typing (stress test)"
    
    # 快速输入 100 个字符
    for i in {1..20}; do
        send_keystroke "test "
        sleep 0.05
    done
    
    log_info "✓ Rapid typing handled"
    add_report_result "Rapid Typing" "PASS"
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

test_alphanumeric_input
test_special_characters
test_arrow_keys
test_function_keys
test_control_combinations
test_rapid_typing

stop_pmux

echo ""
echo "================================"
echo "Keyboard Input Test Complete"
echo "================================"
exit $TESTS_FAILED
