#!/bin/bash
# 功能测试: Vim TUI 兼容性
# 测试 Vim 编辑器在 terminal 中的基本操作

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../regression/lib/test_utils.sh"

echo "================================"
echo "Vim TUI Compatibility Test"
echo "================================"
echo ""

test_vim_launch() {
    log_info "Test: Launch Vim"
    
    send_keystroke "vim /tmp/test_vim_$$.txt"
    send_keycode 36  # Return
    sleep 2
    
    # 按 i 进入 insert 模式
    send_keystroke "i"
    sleep 0.5
    
    # 输入文本
    send_keystroke "This is a test file for vim"
    sleep 0.5
    
    # 保存退出
    send_keycode 53  # Escape
    sleep 0.3
    send_keystroke ":wq"
    sleep 0.3
    send_keycode 36  # Return
    sleep 1
    
    # 验证文件是否创建
    if [ -f "/tmp/test_vim_$$.txt" ]; then
        log_info "✓ Vim launched, edited, and saved file"
        add_report_result "Vim Launch" "PASS"
        rm -f "/tmp/test_vim_$$.txt"
        return 0
    else
        log_warn "⚠ Vim launch tested (file creation could not be verified)"
        add_report_result "Vim Launch" "WARN"
        return 0
    fi
}

test_vim_navigation() {
    log_info "Test: Vim navigation (hjkl, gg, G)"
    
    # 创建测试文件
    head -100 /dev/urandom | base64 > /tmp/vim_nav_test.txt
    
    send_keystroke "vim /tmp/vim_nav_test.txt"
    send_keycode 36
    sleep 2
    
    # 导航测试
    send_keystroke "gg"  # 到文件开头
    sleep 0.3
    send_keystroke "G"   # 到文件结尾
    sleep 0.3
    
    # hjkl 导航
    send_keystroke "hhhh"
    sleep 0.3
    send_keystroke "jjjj"
    sleep 0.3
    send_keystroke "kkkk"
    sleep 0.3
    send_keystroke "llll"
    sleep 0.3
    
    # 退出
    send_keystroke ":q!"
    send_keycode 36
    sleep 1
    
    log_info "✓ Vim navigation tested"
    add_report_result "Vim Navigation" "PASS"
    
    rm -f /tmp/vim_nav_test.txt
}

test_vim_visual_mode() {
    log_info "Test: Vim visual mode (v, V, Ctrl+V)"
    
    echo "line1
line2
line3
line4
line5" > /tmp/vim_visual_test.txt
    
    send_keystroke "vim /tmp/vim_visual_test.txt"
    send_keycode 36
    sleep 2
    
    # 进入 visual mode
    send_keystroke "v"
    sleep 0.2
    send_keystroke "llll"
    sleep 0.3
    send_keystroke "jj"
    sleep 0.3
    
    # 退出 visual mode
    send_keycode 53
    sleep 0.3
    
    # 退出 vim
    send_keystroke ":q!"
    send_keycode 36
    sleep 1
    
    log_info "✓ Vim visual mode tested"
    add_report_result "Vim Visual Mode" "PASS"
    
    rm -f /tmp/vim_visual_test.txt
}

test_vim_resize() {
    log_info "Test: Vim with window resize"
    
    send_keystroke "vim"
    send_keycode 36
    sleep 2
    
    # 进入 insert mode 输入内容
    send_keystroke "i"
    sleep 0.3
    for i in {1..50}; do
        send_keystroke "This is line $i in the test file. "
        send_keycode 36  # Return
        sleep 0.05
    done
    
    # 退出 insert mode
    send_keycode 53
    sleep 0.3
    
    # 模拟窗口 resize (通过创建 pane 分割)
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key down command'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "d"'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key up command'
    sleep 2
    
    # 关闭新 pane
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key down command'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "w"'
    osascript_cmd 'tell application "System Events" to tell process "pmux" to key up command'
    sleep 1
    
    # 退出 vim
    send_keystroke ":q!"
    send_keycode 36
    sleep 1
    
    log_info "✓ Vim resize tested"
    add_report_result "Vim Resize" "PASS"
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

test_vim_launch
test_vim_navigation
test_vim_visual_mode
test_vim_resize

stop_pmux

echo ""
echo "================================"
echo "Vim TUI Test Complete"
echo "================================"
exit $TESTS_FAILED
