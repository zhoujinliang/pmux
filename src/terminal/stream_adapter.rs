//! Stream adapters for gpui-terminal: RuntimeReader, RuntimeWriter, tee_output.

use std::io::{Read, Result as IoResult, Write};
use std::sync::Arc;

use crate::runtime::{AgentRuntime, PaneId};

/// Wraps flume::Receiver<Vec<u8>> as std::io::Read for gpui-terminal.
pub struct RuntimeReader {
    rx: flume::Receiver<Vec<u8>>,
    buf: Vec<u8>,
    pos: usize,
}

impl RuntimeReader {
    pub fn new(rx: flume::Receiver<Vec<u8>>) -> Self {
        Self {
            rx,
            buf: Vec::new(),
            pos: 0,
        }
    }
}

impl Read for RuntimeReader {
    fn read(&mut self, out: &mut [u8]) -> IoResult<usize> {
        while self.pos >= self.buf.len() {
            match self.rx.recv() {
                Ok(chunk) => {
                    self.buf = chunk;
                    self.pos = 0;
                }
                Err(_) => return Ok(0),
            }
        }
        let n = (self.buf.len() - self.pos).min(out.len());
        out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// Forwards std::io::Write to runtime.send_input for gpui-terminal.
pub struct RuntimeWriter {
    runtime: Arc<dyn AgentRuntime>,
    pane_id: PaneId,
}

impl RuntimeWriter {
    pub fn new(runtime: Arc<dyn AgentRuntime>, pane_id: PaneId) -> Self {
        Self { runtime, pane_id }
    }
}

impl Write for RuntimeWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.runtime
            .send_input(&self.pane_id, buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

/// Spawns a thread that forwards each chunk from `rx` to two new receivers.
/// Use one for gpui-terminal, one for ContentExtractor.
pub fn tee_output(rx: flume::Receiver<Vec<u8>>) -> (flume::Receiver<Vec<u8>>, flume::Receiver<Vec<u8>>) {
    let (tx1, rx1) = flume::unbounded();
    let (tx2, rx2) = flume::unbounded();
    std::thread::spawn(move || {
        // #region agent log
        let mut chunk_count: u64 = 0;
        let mut total_bytes: u64 = 0;
        // #endregion
        while let Ok(chunk) = rx.recv() {
            // #region agent log
            chunk_count += 1;
            total_bytes += chunk.len() as u64;
            if chunk_count <= 3 || (chunk_count % 50 == 0) {
                crate::debug_log::dbg_session_log(
                    "stream_adapter.rs:tee_output",
                    "chunk forwarded",
                    &serde_json::json!({
                        "chunk_count": chunk_count,
                        "chunk_len": chunk.len(),
                        "total_bytes": total_bytes,
                        "preview": String::from_utf8_lossy(&chunk[..chunk.len().min(80)]).to_string()
                    }),
                    "H_output_flow",
                );
            }
            // #endregion
            let _ = tx1.send(chunk.clone());
            let _ = tx2.send(chunk);
        }
    });
    (rx1, rx2)
}

// Tests live in tests/stream_adapter_test.rs to avoid gpui_macros SIGBUS
// during lib test compilation on some macOS setups.
