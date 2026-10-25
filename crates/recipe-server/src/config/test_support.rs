//! Helpers that the tests of the configuration modules share.

use std::io;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

use tracing::Dispatch;
use tracing::subscriber::{DefaultGuard, NoSubscriber};

use crate::logging;

static NEXT_NUMBER: AtomicU32 = AtomicU32::new(0);

/// A registered dispatch that no thread installs. It lives until the test process ends.
///
/// It solves one problem of `tracing-core` 0.1.36 (tokio-rs/tracing#3611). With one live
/// dispatch, a callsite takes its interest from the thread that hits it first. A test thread
/// without a subscriber then caches `Interest::never` for all threads.
static KEEPALIVE: LazyLock<Dispatch> = LazyLock::new(|| Dispatch::new(NoSubscriber::default()));

/// A writer that sends each write to a channel.
struct ChannelWriter(Sender<Vec<u8>>);

impl io::Write for ChannelWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        // A closed channel means that the test is over. The log line is then not needed.
        let _ = self.0.send(buffer.to_vec());
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The JSON log lines of the current thread. The drop of the capture removes the subscriber.
pub(crate) struct LogCapture {
    lines: Receiver<Vec<u8>>,
    _guard: DefaultGuard,
}

impl LogCapture {
    /// Install a JSON subscriber at level `info` for the current thread.
    pub(crate) fn start() -> Self {
        let (sender, lines) = channel();
        let filter = logging::build_filter(Some("info")).expect("a valid filter");
        let subscriber = logging::subscriber(filter, move || ChannelWriter(sender.clone()));
        LazyLock::force(&KEEPALIVE);
        Self {
            lines,
            _guard: tracing::subscriber::set_default(subscriber),
        }
    }

    /// Every line written so far.
    pub(crate) fn lines(&self) -> Vec<String> {
        let mut text = Vec::new();
        while let Ok(chunk) = self.lines.try_recv() {
            text.extend(chunk);
        }
        String::from_utf8(text)
            .expect("log lines are UTF-8")
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

/// A temporary directory of one test. The drop removes it.
pub(crate) struct Sandbox(PathBuf);

impl Sandbox {
    pub(crate) fn new() -> Self {
        let number = NEXT_NUMBER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "recipe-server-config-{}-{number}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    pub(crate) fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path(name);
        std::fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // A leftover directory in the temporary folder does no harm.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
