use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug)]
pub(super) struct BoundedCapture {
    pub bytes: Vec<u8>,
    pub max_bytes: usize,
    pub truncated: bool,
}

impl BoundedCapture {
    pub(super) fn new(max_bytes: usize) -> Self {
        Self {
            bytes: Vec::new(),
            max_bytes,
            truncated: false,
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        if chunk.is_empty() {
            return;
        }
        if self.max_bytes == 0 {
            self.truncated = true;
            return;
        }
        if chunk.len() >= self.max_bytes {
            self.bytes.clear();
            self.bytes
                .extend_from_slice(&chunk[chunk.len() - self.max_bytes..]);
            self.truncated = true;
            return;
        }

        let next_len = self.bytes.len() + chunk.len();
        if next_len > self.max_bytes {
            let excess = next_len - self.max_bytes;
            self.bytes.drain(..excess);
            self.truncated = true;
        }
        self.bytes.extend_from_slice(chunk);
    }
}

pub(super) fn spawn_capture_thread(
    mut reader: impl Read + Send + 'static,
    capture: Arc<Mutex<BoundedCapture>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = [0_u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let mut guard = capture
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    guard.push(&buf[..n]);
                }
                Err(err) => {
                    log::debug!(
                        "managed subprocess reader exiting after read error: {}",
                        err
                    );
                    break;
                }
            }
        }
    })
}

pub(super) fn join_capture_thread(handle: thread::JoinHandle<()>) {
    if let Err(err) = handle.join() {
        log::debug!("managed subprocess capture thread panicked: {:?}", err);
    }
}

pub(super) fn unwrap_capture(capture: Arc<Mutex<BoundedCapture>>) -> BoundedCapture {
    match Arc::try_unwrap(capture) {
        Ok(mutex) => mutex
            .into_inner()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
        Err(shared) => {
            let mut guard = shared
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            BoundedCapture {
                bytes: std::mem::take(&mut guard.bytes),
                max_bytes: guard.max_bytes,
                truncated: guard.truncated,
            }
        }
    }
}
