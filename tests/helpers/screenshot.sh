#!/bin/bash
# tests/helpers/screenshot.sh
# Take screenshot of pmux window or full screen

set -e

OUTPUT=${1:-"/tmp/pmux_screenshot_$(date +%s).png"}
WINDOW_ONLY=${2:-"true"}

# Find pmux window
WINDOW_ID=$(osascript -e 'tell application "System Events" to get id of window 1 of process "pmux"' 2>/dev/null || echo "")

if [ "$WINDOW_ONLY" = "true" ] && [ -n "$WINDOW_ID" ]; then
    # Screenshot specific window
    screencapture -l "$WINDOW_ID" -x "$OUTPUT"
else
    # Full screen screenshot
    screencapture -x "$OUTPUT"
fi

echo "$OUTPUT"