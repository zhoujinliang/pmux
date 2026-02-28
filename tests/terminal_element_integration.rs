//! Integration tests for TerminalView + TerminalElement.
//! Verifies TerminalBuffer data path unchanged after TerminalElement switch.

use pmux::terminal::{spawn_pty_reader_with_handle, TerminalEngine};
use pmux::ui::terminal_view::{TerminalBuffer, DEFAULT_ROW_CACHE_SIZE};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

fn make_engine_with_bytes(bytes: &[u8]) -> Arc<TerminalEngine> {
    let (tx, rx) = flume::unbounded();
    let engine = Arc::new(TerminalEngine::new(80, 24, rx));
    tx.send(bytes.to_vec()).unwrap();
    drop(tx);
    engine.advance_bytes();
    engine
}

#[test]
fn test_terminal_buffer_content_unchanged_after_terminal_element_switch() {
    // Verify TerminalBuffer data path; no render, just content extraction
    let engine = make_engine_with_bytes(b"hello\r\n");
    let buf =
        TerminalBuffer::new_term_with_cache_size(engine.clone(), DEFAULT_ROW_CACHE_SIZE);
    let content = buf.content_for_status_detection();
    assert!(content.unwrap().contains("hello"));
}

/// PTY + background resize thread + main thread cat/yes.
/// Verifies no panic, no deadlock under concurrent resize + heavy output.
#[test]
#[ignore = "Requires PTY; slow; run with: cargo test test_concurrent_resize_heavy_output -- --ignored"]
fn test_concurrent_resize_heavy_output() {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
    let mut cmd = CommandBuilder::new(shell);
    cmd.cwd(temp_dir.path());
    let _child = pair.slave.spawn_command(cmd).expect("spawn shell");

    let (tx, rx) = flume::unbounded();
    let engine = Arc::new(TerminalEngine::new(80, 24, rx));

    let fd = pair.master.as_raw_fd().expect("PTY master raw fd");
    let handle = spawn_pty_reader_with_handle(fd, tx);

    let mut writer = pair.master.take_writer().expect("take_writer");
    writer.write_all(b"yes 2>/dev/null | head -5000\n").expect("write");
    writer.flush().expect("flush");

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let engine_clone = engine.clone();
    let resize_thread = thread::spawn(move || {
        for i in 0..100 {
            if stop_clone.load(Ordering::Relaxed) {
                break;
            }
            let cols = 80 + (i % 20) as usize;
            let rows = 24 + (i % 10) as usize;
            engine_clone.resize(cols, rows);
            thread::sleep(Duration::from_millis(5));
        }
    });

    for _ in 0..100 {
        engine.advance_bytes();
        let _ = engine.try_renderable_content(
            |_content, _display_iter, _screen_lines, _cols, _display_offset| (),
        );
        thread::sleep(Duration::from_millis(2));
    }

    stop.store(true, Ordering::Relaxed);
    resize_thread.join().unwrap();

    drop(writer);
    drop(pair);
    handle.shutdown();
}

/// yes-like output flood; verify no panic when engine is stressed.
#[test]
#[ignore = "Requires PTY; slow; run with: cargo test test_pty_flood_gpu_stall -- --ignored"]
fn test_pty_flood_gpu_stall() {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
    let mut cmd = CommandBuilder::new(shell);
    cmd.cwd(temp_dir.path());
    let _child = pair.slave.spawn_command(cmd).expect("spawn shell");

    let (tx, rx) = flume::unbounded();
    let engine = Arc::new(TerminalEngine::new(80, 24, rx));

    let fd = pair.master.as_raw_fd().expect("PTY master raw fd");
    let handle = spawn_pty_reader_with_handle(fd, tx);

    let mut writer = pair.master.take_writer().expect("take_writer");
    writer.write_all(b"yes 2>/dev/null | head -10000\n").expect("write");
    writer.flush().expect("flush");

    for _ in 0..500 {
        engine.advance_bytes();
        let _ = engine.try_renderable_content(
            |_content, _display_iter, _screen_lines, _cols, _display_offset| (),
        );
    }

    drop(writer);
    drop(pair);
    handle.shutdown();
}
