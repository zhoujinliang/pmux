#!/bin/bash
# 测试5: Claude Code TUI - 光标位置测试，特别是输入 / 后光标在斜杠之后

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../regression/lib/test_utils.sh"

echo "================================"
echo "Claude Code TUI Cursor Test"
echo "================================"
echo ""

log_info "Step 1: Start pmux"
cat > "$HOME/.config/pmux/state.json" << 'EOF'
{
  "workspaces": ["/Users/matt.chow/workspace/saas-mono"],
  "active_workspace_index": 0
}
EOF

stop_pmux
sleep 1
start_pmux || exit 1
sleep 5
activate_window
sleep 1

log_info "Step 2: Launch claude"
send_keystroke "claude"
sleep 0.5
send_keycode 36  # Return
sleep 5

# 检查是否进入 TUI
if ! pgrep -f "target/debug/pmux" > /dev/null; then
    log_error "✗ Crashed launching claude"
    add_report_result "Launch Claude" "FAIL"
    stop_pmux
    exit 1
fi

log_info "✓ Claude launched (or attempted)"
add_report_result "Launch Claude" "PASS"

# 如果 claude 命令不存在，我们会停留在 shell，这也是可接受的
# 但我们假设测试环境有 claude 或类似的 TUI 应用

log_info "Step 3: Test TUI input - type 'hello'"
send_keystroke "hello"
sleep 0.5

# 截图记录光标位置
take_screenshot "claude_tui_hello"

log_info "Step 4: Clear input (Ctrl+C)"
osascript_cmd 'tell application "System Events" to tell process "pmux" to key down control'
osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "c"'
osascript_cmd 'tell application "System Events" to tell process "pmux" to key up control'
sleep 1

log_info "Step 5: Test slash command - type '/'"
send_keystroke "/"
sleep 0.5

# 截图记录光标位置（应该在斜杠之后）
take_screenshot "claude_tui_slash"

log_info "Step 6: Type command name after slash"
send_keystroke "clear"
sleep 0.5

# 截图
take_screenshot "claude_tui_slash_clear"

log_info "Step 7: Exit TUI (if in Claude, q to quit)"
# 先尝试 q
send_keystroke "q"
sleep 0.5

# 如果还在，尝试 Ctrl+C
osascript_cmd 'tell application "System Events" to tell process "pmux" to key down control'
osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "c"'
osascript_cmd 'tell application "System Events" to tell process "pmux" to key up control'
sleep 1

# 再尝试 :q (vim style)
send_keystroke ":q"
send_keycode 36
sleep 1

# 最后尝试 exit
send_keystroke "exit"
send_keycode 36
sleep 1

if pgrep -f "target/debug/pmux" > /dev/null; then
    log_info "✓ Back to shell"
    add_report_result "Exit TUI" "PASS"
else
    log_error "✗ Crashed exiting TUI"
    add_report_result "Exit TUI" "FAIL"
fi

stop_pmux

echo ""
echo "================================"
echo "Claude Code TUI Test Complete"
echo "================================"
echo ""
echo "Manual verification required:"
echo "1. Check 'claude_tui_hello' screenshot - cursor should be after 'hello'"
echo "2. Check 'claude_tui_slash' screenshot - cursor should be after '/'"
echo "3. Check 'claude_tui_slash_clear' screenshot - cursor should be after 'clear'"
echo ""
echo "If claude command not found, test still passes if no crash."
echo ""

exit 0
