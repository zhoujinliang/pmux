#!/bin/bash
# tests/helpers/recording.sh
# Screen recording utilities

set -e

RECORD_DIR="/tmp/pmux_recordings"
mkdir -p "$RECORD_DIR"

start_recording() {
    local output=${1:-"$RECORD_DIR/recording_$(date +%s).mp4"}
    local duration=${2:-10}
    
    if ! command -v ffmpeg &> /dev/null; then
        echo "Error: ffmpeg not installed"
        echo "Install with: brew install ffmpeg"
        exit 1
    fi
    
    echo "Starting recording..."
    echo "Output: $output"
    echo "Duration: ${duration}s"
    
    # Start ffmpeg recording
    ffmpeg -f avfoundation -i "1" -r 30 -t "$duration" -y "$output" 2>/dev/null &
    FFMPEG_PID=$!
    
    echo "PID: $FFMPEG_PID"
    echo "$FFMPEG_PID" > /tmp/pmux_recording.pid
}

stop_recording() {
    if [ -f /tmp/pmux_recording.pid ]; then
        PID=$(cat /tmp/pmux_recording.pid)
        kill "$PID" 2>/dev/null || true
        rm /tmp/pmux_recording.pid
        echo "Recording stopped"
    else
        echo "No recording in progress"
    fi
}

analyze_recording() {
    local video=${1:-""}
    
    if [ -z "$video" ] || [ ! -f "$video" ]; then
        echo "Error: Video file not found"
        exit 1
    fi
    
    if ! command -v ffprobe &> /dev/null; then
        echo "Error: ffprobe not installed"
        exit 1
    fi
    
    echo "=== Video Analysis ==="
    echo ""
    
    # Duration
    DURATION=$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$video")
    echo "Duration: ${DURATION}s"
    
    # Frame count
    FRAMES=$(ffprobe -v error -select_streams v:0 -count_packets -show_entries stream=nb_read_packets -of csv=p=0 "$video")
    echo "Frames: $FRAMES"
    
    # FPS
    FPS=$(echo "scale=1; $FRAMES / $DURATION" | bc 2>/dev/null || echo "N/A")
    echo "Average FPS: $FPS"
    
    # Resolution
    RESOLUTION=$(ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=p=0 "$video")
    echo "Resolution: ${RESOLUTION/,/x}"
    
    echo ""
    
    # Pass/Fail
    if [ "$FPS" != "N/A" ]; then
        FPS_INT=$(echo "$FPS" | cut -d. -f1)
        if [ "$FPS_INT" -ge 25 ]; then
            echo "✓ PASS: FPS >= 25"
        else
            echo "✗ FAIL: FPS < 25"
        fi
    fi
}

# Main
case "${1:-help}" in
    "start")
        start_recording "${2:-}" "${3:-10}"
        ;;
    "stop")
        stop_recording
        ;;
    "analyze")
        analyze_recording "${2:-}"
        ;;
    *)
        echo "Usage: $0 {start|stop|analyze}"
        echo ""
        echo "  start <output> <duration>  - Start recording"
        echo "  stop                       - Stop recording"
        echo "  analyze <video>            - Analyze recording"
        ;;
esac