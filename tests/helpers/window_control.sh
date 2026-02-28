#!/bin/bash
# tests/helpers/window_control.sh
# Control pmux window

set -e

ACTION=${1:-"help"}
shift 2>/dev/null || true

case "$ACTION" in
    "resize")
        WIDTH=${1:-800}
        HEIGHT=${2:-600}
        osascript -e "tell application \"pmux\" to set bounds of front window to {100, 100, $((100 + WIDTH)), $((100 + HEIGHT))}"
        echo "Window resized to ${WIDTH}x${HEIGHT}"
        ;;
    "move")
        X=${1:-100}
        Y=${2:-100}
        osascript -e "tell application \"pmux\" to set position of front window to {$X, $Y}"
        echo "Window moved to ($X, $Y)"
        ;;
    "focus")
        osascript -e 'tell application "pmux" to activate'
        echo "Window focused"
        ;;
    "close")
        osascript -e 'tell application "pmux" to quit'
        echo "Application closed"
        ;;
    "bounds")
        BOUNDS=$(osascript -e 'tell application "pmux" to get bounds of front window' 2>/dev/null || echo "unknown")
        echo "Window bounds: $BOUNDS"
        ;;
    "exists")
        EXISTS=$(osascript -e 'tell application "System Events" to get name of processes' 2>/dev/null | grep -c "pmux" || echo "0")
        if [ "$EXISTS" -gt 0 ]; then
            echo "pmux is running"
            exit 0
        else
            echo "pmux is not running"
            exit 1
        fi
        ;;
    *)
        echo "Usage: $0 {resize|move|focus|close|bounds|exists}"
        echo ""
        echo "  resize <width> <height>  - Resize window"
        echo "  move <x> <y>             - Move window"
        echo "  focus                    - Focus window"
        echo "  close                    - Close application"
        echo "  bounds                   - Get window bounds"
        echo "  exists                   - Check if running"
        ;;
esac