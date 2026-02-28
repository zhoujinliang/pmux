//! PTY reader thread - blocking I/O with 64KB buffer.
//!
//! Spawns a dedicated thread that reads from the PTY master fd and sends
//! byte chunks over a flume channel. Used by TerminalEngine for processing.
//! Supports clean shutdown via `PtyReaderHandle::shutdown()`.

use std::os::fd::RawFd;
use std::thread;

use libc::{c_void, read};

/// Spawn a blocking PTY reader thread with shutdown support.
///
/// Reads from `master_fd` in a loop using a 65536-byte buffer. Each read
/// chunk is sent as a whole `Vec<u8>` over the channel. Exits when:
/// - `shutdown_rx` receives a signal or is disconnected
/// - `read()` returns <= 0 (EOF or error)
/// - The receiver is dropped (`tx.send()` fails)
///
/// Uses non-blocking `try_recv()` for shutdown check before each read;
/// does not significantly impact performance.
pub fn spawn_pty_reader(
    master_fd: RawFd,
    tx: flume::Sender<Vec<u8>>,
    shutdown_rx: flume::Receiver<()>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        const BUF_SIZE: usize = 65536;
        let mut buf = vec![0u8; BUF_SIZE];

        loop {
            // Non-blocking shutdown check (cooperative; thread exits after next read)
            match shutdown_rx.try_recv() {
                Ok(()) | Err(flume::TryRecvError::Disconnected) => break,
                Err(flume::TryRecvError::Empty) => {}
            }

            // Blocking read - this is what we want for efficiency
            let n = unsafe {
                read(
                    master_fd,
                    buf.as_mut_ptr() as *mut c_void,
                    BUF_SIZE,
                )
            };

            if n <= 0 {
                break; // EOF or error
            }

            let bytes = buf[..n as usize].to_vec();
            if tx.send(bytes).is_err() {
                break; // Receiver dropped
            }
        }
    })
}

/// Handle for a PTY reader thread with clean shutdown.
pub struct PtyReaderHandle {
    pub thread: thread::JoinHandle<()>,
    pub shutdown_tx: flume::Sender<()>,
}

impl PtyReaderHandle {
    /// Signal the reader to stop and wait for the thread to finish.
    pub fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        self.thread.join().ok();
    }
}

/// Spawn a PTY reader and return a handle for clean shutdown.
pub fn spawn_pty_reader_with_handle(
    master_fd: RawFd,
    tx: flume::Sender<Vec<u8>>,
) -> PtyReaderHandle {
    let (shutdown_tx, shutdown_rx) = flume::bounded(1);
    let thread = spawn_pty_reader(master_fd, tx, shutdown_rx);
    PtyReaderHandle { thread, shutdown_tx }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_pty_reader_with_handle_shutdown() {
        // Create a channel and spawn with invalid fd (-1) - reader will exit immediately
        let (tx, rx) = flume::bounded(1);
        let handle = spawn_pty_reader_with_handle(-1, tx);
        drop(rx); // Drop receiver so any send would fail
        handle.shutdown();
    }
}
