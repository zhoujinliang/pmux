#!/bin/bash
# tests/helpers/keyboard.sh
# Send keyboard input to focused application

set -e

TYPE=${1:-"help"}
shift 2>/dev/null || true

case "$TYPE" in
    "text")
        TEXT=${1:-""}
        if [ -z "$TEXT" ]; then
            echo "Error: No text provided"
            exit 1
        fi
        osascript -e "tell application \"System Events\" to keystroke \"$TEXT\""
        ;;
    "enter")
        osascript -e 'tell application "System Events" to key code 36'
        ;;
    "tab")
        osascript -e 'tell application "System Events" to key code 48'
        ;;
    "escape")
        osascript -e 'tell application "System Events" to key code 53'
        ;;
    "backspace")
        osascript -e 'tell application "System Events" to key code 51'
        ;;
    "arrow")
        DIR=${1:-"right"}
        case "$DIR" in
            "up")    CODE=126 ;;
            "down")  CODE=125 ;;
            "left")  CODE=123 ;;
            "right") CODE=124 ;;
            *)       CODE=124 ;;
        esac
        osascript -e "tell application \"System Events\" to key code $CODE"
        ;;
    "key")
        KEY=${1:-""}
        case "$KEY" in
            "a") CODE=0 ;;
            "b") CODE=11 ;;
            "c") CODE=8 ;;
            "d") CODE=2 ;;
            "e") CODE=14 ;;
            "f") CODE=3 ;;
            "g") CODE=5 ;;
            "h") CODE=4 ;;
            "i") CODE=34 ;;
            "j") CODE=38 ;;
            "k") CODE=40 ;;
            "l") CODE=37 ;;
            "m") CODE=46 ;;
            "n") CODE=45 ;;
            "o") CODE=31 ;;
            "p") CODE=35 ;;
            "q") CODE=12 ;;
            "r") CODE=15 ;;
            "s") CODE=1 ;;
            "t") CODE=17 ;;
            "u") CODE=32 ;;
            "v") CODE=9 ;;
            "w") CODE=13 ;;
            "x") CODE=7 ;;
            "y") CODE=16 ;;
            "z") CODE=6 ;;
            "0") CODE=29 ;;
            "1") CODE=18 ;;
            "2") CODE=19 ;;
            "3") CODE=20 ;;
            "4") CODE=21 ;;
            "5") CODE=23 ;;
            "6") CODE=22 ;;
            "7") CODE=26 ;;
            "8") CODE=28 ;;
            "9") CODE=25 ;;
            "space") CODE=49 ;;
            *)       
                echo "Unknown key: $KEY"
                exit 1 
                ;;
        esac
        osascript -e "tell application \"System Events\" to key code $CODE"
        ;;
    "combo")
        MODIFIER=${1:-"command"}
        KEY=${2:-"a"}
        
        # Get key code
        case "$KEY" in
            "a") CODE=0 ;;
            "c") CODE=8 ;;
            "v") CODE=9 ;;
            "x") CODE=7 ;;
            "z") CODE=6 ;;
            "s") CODE=1 ;;
            "q") CODE=12 ;;
            "w") CODE=13 ;;
            *)   CODE=0 ;;
        esac
        
        # Send combo
        case "$MODIFIER" in
            "command") MOD="command down" ;;
            "shift")   MOD="shift down" ;;
            "option")  MOD="option down" ;;
            "control") MOD="control down" ;;
            *)         MOD="command down" ;;
        esac
        
        osascript -e "tell application \"System Events\" to key code $CODE using {$MOD}"
        ;;
    "ctrl")
        KEY=${1:-"c"}
        case "$KEY" in
            "c") osascript -e 'tell application "System Events" to keystroke "c" using {control down}' ;;
            "d") osascript -e 'tell application "System Events" to keystroke "d" using {control down}' ;;
            "z") osascript -e 'tell application "System Events" to keystroke "z" using {control down}' ;;
            "l") osascript -e 'tell application "System Events" to keystroke "l" using {control down}' ;;
            "a") osascript -e 'tell application "System Events" to keystroke "a" using {control down}' ;;
            "e") osascript -e 'tell application "System Events" to keystroke "e" using {control down}' ;;
            *)   echo "Unknown ctrl combo: $KEY" ;;
        esac
        ;;
    *)
        echo "Usage: $0 {text|enter|tab|escape|backspace|arrow|key|combo|ctrl}"
        echo ""
        echo "  text <string>        - Type text"
        echo "  enter                - Press Enter"
        echo "  tab                  - Press Tab"
        echo "  escape               - Press Escape"
        echo "  backspace            - Press Backspace"
        echo "  arrow <dir>          - Press arrow (up/down/left/right)"
        echo "  key <key>            - Press single key (a-z, 0-9, space)"
        echo "  combo <mod> <key>    - Press modifier+key (command/shift/option/control)"
        echo "  ctrl <key>           - Press Ctrl+key"
        ;;
esac