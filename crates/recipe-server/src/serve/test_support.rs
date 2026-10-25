//! Helpers that the tests of the start share (design D14 row T26).
//!
//! The stub host is in [`stub`]. This file holds the log capture, the temporary directory, and the
//! factories of a checked configuration.

pub(super) mod stub;
mod watch;

use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use recipe_app::operations::ConfiguredSettings;
use recipe_app::ports::{Clock, TimeStateKey};
use recipe_domain::identity::KeyRing;
use recipe_domain::recipe::LangCode;
use recipe_domain::timestamp::Timestamp;
use recipe_web::CidrList;
use tokio::time::Instant;
use tracing::subscriber::{DefaultGuard, NoSubscriber};
use tracing::{Dispatch, Subscriber};

use super::plan::Plan;
use crate::logging;

/// The public port of the test plan.
pub(super) const PUBLIC_PORT: u16 = 18_080;
/// The internal port of the test plan.
pub(super) const INTERNAL_PORT: u16 = 18_081;
/// The first instant of the test clock.
pub(super) const START_TEXT: &str = "2026-10-24T12:00:00.000000Z";

/// The 18 checked values of a test start. The webhook and its CA file are not set.
pub(super) fn configured() -> ConfiguredSettings {
    ConfiguredSettings {
        db_host: "db".to_owned(),
        db_port: 5432,
        db_name: "recipe".to_owned(),
        db_app_user: "recipe_app".to_owned(),
        app_password_file: "/run/secrets/app_password".to_owned(),
        login_hmac_key_file: "/run/secrets/login_hmac_key".to_owned(),
        csrf_signing_key_file: "/run/secrets/csrf_signing_key".to_owned(),
        trusted_proxies: vec!["172.28.0.2/32".to_owned()],
        public_port: PUBLIC_PORT,
        internal_port: INTERNAL_PORT,
        separation_of_duties: true,
        webhook_url_file: None,
        webhook_ca_file: None,
        webhook_url: None,
        session_secure: true,
        idle_timeout: Duration::from_secs(1_800),
        absolute_timeout: Duration::from_secs(43_200),
        translation_langs: vec!["pl".to_owned()],
    }
}

/// A key ring with one key.
pub(super) fn key_ring() -> KeyRing {
    KeyRing::parse(&"ab".repeat(32)).unwrap()
}

/// The plan of a test start with the default numbers.
pub(super) fn plan() -> Plan {
    Plan::new(
        configured(),
        key_ring(),
        key_ring(),
        None,
        CidrList::default(),
        vec![LangCode::parse("pl").unwrap()],
    )
}

/// A clock that follows the clock of Tokio, from a fixed first instant.
#[derive(Debug)]
pub(super) struct TokioClock {
    first: Timestamp,
    started: Instant,
}

impl TokioClock {
    /// A clock that shows the UTC time `text` now. Call it inside the runtime of the test.
    pub(super) fn at(text: &str) -> Arc<Self> {
        Arc::new(Self {
            first: TimeStateKey::parse_text(text).unwrap(),
            started: Instant::now(),
        })
    }
}

impl Clock for TokioClock {
    fn now(&self) -> Timestamp {
        let elapsed = self.started.elapsed().try_into().unwrap();
        Timestamp::from(
            self.first
                .as_offset_date_time()
                .checked_add(elapsed)
                .unwrap(),
        )
    }
}

static NEXT_NUMBER: AtomicU32 = AtomicU32::new(0);

/// A temporary directory of one test. The drop removes it.
pub(super) struct Sandbox(PathBuf);

impl Sandbox {
    pub(super) fn new() -> Self {
        let number = NEXT_NUMBER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "recipe-server-serve-{}-{number}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(super) fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    pub(super) fn missing(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A registered dispatch that no thread installs. It lives until the test process ends.
///
/// It solves one problem of `tracing-core` 0.1.36 (tokio-rs/tracing#3611). With one live
/// dispatch, a callsite takes its interest from the thread that hits it first. A test thread
/// without a subscriber then caches `Interest::never` for all threads.
static KEEPALIVE: LazyLock<Dispatch> = LazyLock::new(|| Dispatch::new(NoSubscriber::default()));

fn set_test_default<S>(subscriber: S) -> DefaultGuard
where
    S: Subscriber + Send + Sync + 'static,
{
    LazyLock::force(&KEEPALIVE);
    tracing::subscriber::set_default(subscriber)
}

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

/// The log lines of the current thread.
pub(super) struct LogCapture {
    lines: Receiver<Vec<u8>>,
    _guard: DefaultGuard,
}

impl LogCapture {
    /// Install a JSON subscriber for the current thread until the capture drops.
    pub(super) fn start() -> Self {
        let (sender, lines) = std::sync::mpsc::channel();
        let filter = logging::build_filter(Some("info")).unwrap();
        let subscriber = logging::subscriber(filter, move || ChannelWriter(sender.clone()));
        Self {
            lines,
            _guard: set_test_default(subscriber),
        }
    }

    /// Every line written so far.
    pub(super) fn lines(&self) -> Vec<String> {
        let mut text = Vec::new();
        while let Ok(chunk) = self.lines.try_recv() {
            text.extend(chunk);
        }
        String::from_utf8(text)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// The lines of one level, for example `ERROR`.
    pub(super) fn at_level(&self, level: &str) -> Vec<String> {
        let marker = format!("\"level\":\"{level}\"");
        self.lines()
            .into_iter()
            .filter(|line| line.contains(&marker))
            .collect()
    }
}
