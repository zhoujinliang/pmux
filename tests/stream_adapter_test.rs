//! Stream adapter integration tests.
//! Run with: RUSTUP_TOOLCHAIN=stable cargo test stream_adapter_test
//!
//! These live in integration tests to avoid gpui_macros SIGBUS during lib test compilation.

use pmux::runtime::{AgentId, AgentRuntime, PaneId, RuntimeError};
use pmux::terminal::{RuntimeReader, RuntimeWriter, tee_output};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[test]
fn test_runtime_reader_reads_from_flume() {
    let (tx, rx) = flume::unbounded();
    tx.send(b"hello".to_vec()).unwrap();
    tx.send(b" world".to_vec()).unwrap();
    drop(tx);

    let mut reader = RuntimeReader::new(rx);
    let mut buf = [0u8; 32];
    let n = reader.read(&mut buf).unwrap();
    assert_eq!(n, 5);
    assert_eq!(&buf[..5], b"hello");

    let n2 = reader.read(&mut buf).unwrap();
    assert_eq!(n2, 6);
    assert_eq!(&buf[..6], b" world");
}

#[test]
fn test_runtime_writer_forwards_to_send_input() {
    struct MockRuntime {
        sent: AtomicU64,
    }
    impl AgentRuntime for MockRuntime {
        fn backend_type(&self) -> &'static str {
            "mock"
        }
        fn send_input(&self, _: &PaneId, bytes: &[u8]) -> Result<(), RuntimeError> {
            self.sent.fetch_add(bytes.len() as u64, Ordering::SeqCst);
            Ok(())
        }
        fn send_key(&self, _: &PaneId, _: &str, _: bool) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn resize(&self, _: &PaneId, _: u16, _: u16) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn subscribe_output(&self, _: &PaneId) -> Option<flume::Receiver<Vec<u8>>> {
            None
        }
        fn capture_initial_content(&self, _: &PaneId) -> Option<Vec<u8>> {
            None
        }
        fn list_panes(&self, _: &AgentId) -> Vec<PaneId> {
            vec![]
        }
        fn focus_pane(&self, _: &PaneId) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn split_pane(&self, _: &PaneId, _: bool) -> Result<PaneId, RuntimeError> {
            Err(RuntimeError::Backend("".into()))
        }
        fn get_pane_dimensions(&self, _: &PaneId) -> (u16, u16) {
            (80, 24)
        }
        fn open_diff(&self, _: &Path, _: Option<&PaneId>) -> Result<String, RuntimeError> {
            Err(RuntimeError::Backend("".into()))
        }
        fn open_review(&self, _: &Path) -> Result<String, RuntimeError> {
            Err(RuntimeError::Backend("".into()))
        }
        fn kill_window(&self, _: &str) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn session_info(&self) -> Option<(String, String)> {
            None
        }
    }

    let rt = Arc::new(MockRuntime {
        sent: AtomicU64::new(0),
    });
    let pane_id = PaneId::from("%0");
    let mut writer = RuntimeWriter::new(rt.clone(), pane_id.clone());
    writer.write_all(b"abc").unwrap();
    writer.flush().unwrap();
    assert_eq!(rt.sent.load(Ordering::SeqCst), 3);
}

#[test]
fn test_tee_pipe_fans_out_bytes() {
    let (tx, rx) = flume::unbounded();
    let (rx1, rx2) = tee_output(rx);
    tx.send(b"x".to_vec()).unwrap();
    tx.send(b"y".to_vec()).unwrap();
    drop(tx);

    // Give the tee thread time to forward (avoids flaky test on slow CI)
    std::thread::sleep(std::time::Duration::from_millis(50));

    let a: Vec<Vec<u8>> = rx1.try_iter().collect();
    let b: Vec<Vec<u8>> = rx2.try_iter().collect();
    assert_eq!(a, vec![b"x".to_vec(), b"y".to_vec()]);
    assert_eq!(b, vec![b"x".to_vec(), b"y".to_vec()]);
}
