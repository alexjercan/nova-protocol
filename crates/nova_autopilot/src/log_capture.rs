//! Test-only `tracing` capture, so a test can assert on what a driver LOGGED.
//!
//! A warn-and-continue path has no return value to assert against: the whole
//! contract is the line it emits. Without capture such a test passes with the
//! warn deleted, which is coverage that is not there (review R2.4).

use std::sync::{Arc, Mutex};

/// A `tracing` sink that keeps every formatted line in memory.
#[derive(Clone, Default)]
pub(crate) struct LogBuf(Arc<Mutex<Vec<u8>>>);

impl LogBuf {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

impl std::io::Write for LogBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogBuf {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Run `body` with every log line captured into the returned string.
pub(crate) fn capturing_logs<T>(body: impl FnOnce() -> T) -> (T, String) {
    let logs = LogBuf::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(logs.clone())
        .with_ansi(false)
        .finish();
    let out = tracing::subscriber::with_default(subscriber, body);
    (out, logs.text())
}
