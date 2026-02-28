//! Direct PTY writer for nonblocking writes.
//!
//! Writes bytes directly to the PTY master fd using libc::write.
//! Used for keyboard input (same-thread, no async hop).

use std::os::fd::RawFd;

use libc::{c_void, write};

/// Direct PTY writer for nonblocking writes.
#[derive(Clone)]
pub struct PtyWriter {
    master_fd: RawFd,
}

impl PtyWriter {
    pub fn new(master_fd: RawFd) -> Self {
        Self { master_fd }
    }

    /// Write bytes directly to PTY using libc::write.
    /// Returns number of bytes written.
    pub fn write(&self, bytes: &[u8]) -> std::io::Result<usize> {
        let n = unsafe {
            write(self.master_fd, bytes.as_ptr() as *const c_void, bytes.len())
        };
        if n < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(n as usize)
        }
    }

    /// Write all bytes (retry on partial write).
    pub fn write_all(&self, bytes: &[u8]) -> std::io::Result<()> {
        let mut written = 0;
        while written < bytes.len() {
            match self.write(&bytes[written..]) {
                Ok(0) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "write returned 0",
                    ))
                }
                Ok(n) => written += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

unsafe impl Send for PtyWriter {}
unsafe impl Sync for PtyWriter {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_writer_creation() {
        // Use a dummy fd (0 is stdin, but we're just testing creation)
        let writer = PtyWriter::new(0);
        // Just verify it doesn't panic
        drop(writer);
    }
}
