//! Tmux backend integration test. Run with: tmux kill-server; cargo test tmux_integration -- --ignored --nocapture
//! Requires tmux and only runs on unix.

#![cfg(unix)]

use pmux::runtime::AgentRuntime;
use std::process::Command;
use std::time::{Duration, Instant};

fn tmux_available() -> bool {
    Command::new("tmux").arg("-V").output().is_ok_and(|o| o.status.success())
}

#[test]
#[ignore]
fn test_tmux_pipe_pane_and_input() {
    if !tmux_available() {
        eprintln!("tmux not available, skip");
        return;
    }

    let session = std::env::var("PMUX_TMUX_TEST_SESSION")
        .unwrap_or_else(|_| format!("pmux-integration-{}", std::process::id()));
    let test_dir = std::env::var("PMUX_TMUX_TEST_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let test_dir = std::path::Path::new(&test_dir);

    // Clean session
    let _ = Command::new("tmux").args(["kill-session", "-t", &session]).output();

    let rt = pmux::runtime::backends::TmuxRuntime::new(&session, "main", Some(test_dir));
    let pane_id = rt
        .primary_pane_id()
        .unwrap_or_else(|| panic!("no primary pane"));

    let rx = rt
        .subscribe_output(&pane_id)
        .unwrap_or_else(|| panic!("subscribe_output failed"));

    // Focus pane to prewarm PTY cache
    let _ = rt.focus_pane(&pane_id);

    // Send echo command
    let marker = "PMUX_TEST_MARKER_OK";
    rt.send_input(&pane_id, format!("echo {}\r", marker).as_bytes())
        .expect("send_input");

    // Read from pipe-pane until we see marker or timeout (5s)
    let mut all = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(chunk) => {
                all.extend_from_slice(&chunk);
                let s = String::from_utf8_lossy(&all);
                if s.contains(marker) {
                    let _ = Command::new("tmux").args(["kill-session", "-t", &session]).output();
                    return; // PASS
                }
            }
            Err(flume::RecvTimeoutError::Timeout) => continue,
            Err(flume::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = Command::new("tmux").args(["kill-session", "-t", &session]).output();
    panic!(
        "timeout: pipe-pane did not receive '{}'. Received {} bytes: {:?}",
        marker,
        all.len(),
        String::from_utf8_lossy(&all[..all.len().min(500)])
    );
}
