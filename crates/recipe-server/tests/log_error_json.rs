#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Test of the JSON shape of the log line of an error (ADR-0129).
//!
//! The CI flood gate `deploy/ci/load_flood.py` reads `fields.error_code` and `fields.stage` from
//! this line. The test sends a request through the real router of `recipe-web` and captures the
//! line of the real `log_error` with the real subscriber `recipe_server::logging::subscriber`.

use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::Request;
use recipe_app::blocking::AppSettings;
use recipe_app::context::AppContext;
use recipe_app::identity::{BlockingHasher, HashGate, IdentityContext, IdentitySettings};
use recipe_app::memory::{MemoryClock, MemoryStore};
use recipe_domain::identity::KeyRing;
use recipe_server::logging::{build_filter, subscriber};
use recipe_web::{WebSettings, WebState, router};
use serde_json::Value;
use time::OffsetDateTime;
use tower::Service;
use tracing::Dispatch;
use tracing::subscriber::NoSubscriber;

/// The 32-byte key as 64 hex characters. Both key rings of the test use it.
const KEY: &str = "0101010101010101010101010101010101010101010101010101010101010101";
/// The header `Authorization: Basic` for the login `anna` and a valid password.
const BASIC_HEADER: &str = "Basic YW5uYTpjb3JyZWN0LWhvcnNlLWJhdHRlcnk=";
/// An export route that needs the Basic check.
const EXPORT_PATH: &str = "/export/v1/machines/M1/recipes";
/// The wait for an Argon2 permit. The test clock is paused, so this time is virtual.
const ACQUIRE_TIMEOUT: Duration = Duration::from_millis(10);

/// A registered dispatch that no thread installs. It lives until the test process ends.
///
/// It solves one problem of `tracing-core` 0.1.36 (tokio-rs/tracing#3611). With one live
/// dispatch, a callsite takes its interest from the thread that hits it first. A test thread
/// without a subscriber then caches `Interest::never` for all threads.
static KEEPALIVE: LazyLock<Dispatch> = LazyLock::new(|| Dispatch::new(NoSubscriber::default()));

/// A writer that sends each write to a channel, so the test reads the log output without a lock.
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

/// Build the real router over memory adapters. The Basic gate has no permit, so a Basic check
/// ends with the timeout `E-WEB-009` after `ACQUIRE_TIMEOUT`.
fn router_without_basic_permits() -> Router {
    let start = OffsetDateTime::from_unix_timestamp(1_000_000).unwrap();
    let clock = MemoryClock::new(start);
    let store = MemoryStore::with_clock(clock.clone());
    let settings = IdentitySettings {
        argon2_acquire_timeout: ACQUIRE_TIMEOUT,
        export_basic_permits: 0,
        ..IdentitySettings::default()
    };
    let app = AppContext::new(
        Arc::new(store.clone()),
        Arc::new(store.sink()),
        Arc::new(clock),
        AppSettings::default(),
    );
    let gate = HashGate::new(
        settings.argon2_permits,
        settings.argon2_acquire_timeout,
        Arc::new(BlockingHasher::new()),
    );
    let identity = IdentityContext::new(
        app,
        Arc::new(store.clone()),
        Arc::new(store),
        gate,
        KeyRing::parse(KEY).unwrap(),
        settings,
    );
    let state = WebState::new(
        identity,
        WebSettings::default(),
        KeyRing::parse(KEY).unwrap(),
    )
    .unwrap();
    let peer = SocketAddr::new(Ipv4Addr::new(192, 0, 2, 1).into(), 40_000);
    router(state).layer(MockConnectInfo(peer))
}

/// Send one `GET` request to the export route and return the log line of the error.
///
/// The line is the one JSON line that the subscriber of the server wrote while the router
/// answered. `authorization` is the value of the header `Authorization`, or `None` for no header.
async fn error_line_of(authorization: Option<&str>) -> Value {
    let (sender, receiver) = mpsc::channel();
    let filter = build_filter(Some("info")).unwrap();
    let production = subscriber(filter, move || ChannelWriter(sender.clone()));
    LazyLock::force(&KEEPALIVE);
    let guard = tracing::subscriber::set_default(production);

    let mut builder = Request::builder().uri(EXPORT_PATH);
    if let Some(value) = authorization {
        builder = builder.header("authorization", value);
    }
    // The router is always ready, so the call needs no `poll_ready`.
    let mut app = router_without_basic_permits();
    app.call(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    drop(guard);

    let bytes: Vec<u8> = receiver.try_iter().flatten().collect();
    let text = String::from_utf8(bytes).unwrap();
    let lines: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .filter(|line: &Value| line["fields"]["message"] == "request failed")
        .collect();
    assert_eq!(lines.len(), 1, "one error line expected: {text}");
    lines.into_iter().next().unwrap()
}

#[tokio::test(start_paused = true)]
async fn a_timeout_line_has_the_fields_error_code_and_stage_that_the_flood_gate_reads() {
    let line = error_line_of(Some(BASIC_HEADER)).await;

    let fields = &line["fields"];
    assert_eq!(fields["error_code"], "E-WEB-009", "{line}");
    assert_eq!(fields["stage"], "password hash permit", "{line}");
    // The stage has its own field. The field `chain` must not repeat it.
    let chain = fields["chain"].as_str().expect("chain is a string");
    assert!(!chain.contains("password hash permit"), "{line}");
    assert!(!chain.contains("stage"), "{line}");
}

#[tokio::test(start_paused = true)]
async fn an_error_line_that_is_no_timeout_has_no_field_stage() {
    let line = error_line_of(None).await;

    let fields = line["fields"].as_object().expect("fields is an object");
    assert_eq!(fields["error_code"], "E-WEB-012", "{line}");
    assert!(!fields.contains_key("stage"), "{line}");
}
