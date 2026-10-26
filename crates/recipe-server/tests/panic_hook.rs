#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Test T21 of the change `operations-compose`: the panic hook (`reliability.md` section 2.4).
//!
//! A panic hook is global state of the process. This file therefore holds exactly one test, and
//! the test binary runs no other panic. The test installs the hook, starts a panic inside an
//! active span, and reads the JSON line that the hook writes.

use std::io;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};

use recipe_app::operations::AlertCounters;
use recipe_server::{logging, panic_hook};

/// A writer that sends each write to a channel, so a test reads the log output without a lock.
struct ChannelWriter(Sender<Vec<u8>>);

impl io::Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf.to_vec()).map_err(io::Error::other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Start a panic with a known message. `clippy::panic` also applies in tests, so `expect` does it.
///
/// The function stores the line of the panic in `line`, so the test needs no typed line number.
/// `Result::expect` is `#[track_caller]`, so the location is the call on the line after `line!()`.
fn explode(line: &AtomicU32) {
    let too_large = u32::from(u8::MAX).checked_add(1).unwrap();
    line.store(line!().checked_add(1).unwrap(), Ordering::SeqCst);
    u8::try_from(too_large).expect("the test panic");
}

/// The text that the JSON line holds for `file` and `line`: the file name and the line number.
fn location_prefix(file: &str, line: u32) -> String {
    // The JSON line escapes each backslash of a Windows path.
    format!("\"location\":\"{}:{line}:", file.replace('\\', "\\\\"))
}

#[test]
fn a_panic_gives_one_json_error_line_with_the_request_id_and_counts_once() {
    static EARLIER_HOOK_RAN: AtomicBool = AtomicBool::new(false);
    std::panic::set_hook(Box::new(|_| EARLIER_HOOK_RAN.store(true, Ordering::SeqCst)));
    let counters = AlertCounters::default();
    panic_hook::install(counters.clone());
    let (sender, receiver) = mpsc::channel();
    let filter = logging::build_filter(None).unwrap();
    let subscriber = logging::subscriber(filter, move || ChannelWriter(sender.clone()));
    let panic_line = AtomicU32::new(0);

    let outcome = tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("http.request", request_id = "0192-test-request");
        let _entered = span.enter();
        std::panic::catch_unwind(|| explode(&panic_line))
    });
    // A failed assertion below then goes through the default hook.
    drop(std::panic::take_hook());

    assert!(outcome.is_err(), "the panic must reach the caller");
    assert!(
        !EARLIER_HOOK_RAN.load(Ordering::SeqCst),
        "the hook must not call the earlier hook"
    );
    let bytes: Vec<u8> = receiver.try_iter().flatten().collect();
    let text = String::from_utf8(bytes).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1, "{text}");
    let line = lines[0];
    assert!(line.starts_with('{') && line.ends_with('}'), "{line}");
    assert!(line.contains("\"level\":\"ERROR\""), "{line}");
    assert!(line.contains("the test panic"), "{line}");
    let expected = location_prefix(file!(), panic_line.load(Ordering::SeqCst));
    assert!(line.contains(&expected), "{expected} not in {line}");
    assert!(
        line.contains("\"request_id\":\"0192-test-request\""),
        "{line}"
    );
    assert!(line.contains("\"span\":{"), "{line}");
    assert_eq!(counters.panics(), 1);
    assert_eq!(counters.export_hash_mismatches(), 0);
}
