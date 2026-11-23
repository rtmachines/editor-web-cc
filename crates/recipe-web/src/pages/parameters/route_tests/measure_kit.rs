//! Shared kit of the measurement tests of the grid answers (tasks 5.1 to 5.10, design D6,
//! ADR-0223).
//!
//! The kit holds the pin of an answer, the check of an answer against its pin, the printed line,
//! and the loaded request that a render copy needs. Each measurement file imports it.

use std::io::Write as _;
use std::time::Duration;

use axum::body::Body;
use axum::extract::FromRequestParts as _;
use axum::http::{Method, Request, StatusCode};
use recipe_app::identity::authenticate;
use recipe_domain::identity::SessionToken;
use recipe_domain::recipe::{ParameterKey, ReferenceNumber};

use super::largest_form_tests::{KEY, REFERENCE};
use crate::client::ClientAddr;
use crate::pages::PageRequest;
use crate::pages::parameters::reply::{Ctx, Target};
use crate::state::WebState;
use crate::test_support::heap::{BodyDigest, Measured};
use crate::test_support::plant::World;
use crate::test_support::{PEER, request};

/// The slack of a pinned peak above its measured value: 1 MiB.
pub(super) const PEAK_SLACK: usize = 1_048_576;

/// The number of the seeded user of the ProcessEngineer session of `world` (`seed_session(2, ..)`).
pub(super) const ENGINEER_SESSION: u8 = 2;

/// What one answer must have.
pub(super) struct Pin {
    /// The name of the answer in the printed line.
    pub(super) name: &'static str,
    /// The length of the answer body in bytes.
    pub(super) len: usize,
    /// The SHA-256 of the answer body, with the request id replaced.
    pub(super) sha256: &'static str,
    /// The measured peak of the request. The pinned bound is this value plus [`PEAK_SLACK`].
    pub(super) request_peak: usize,
    /// The measured peak of the render interval. The bound is this value plus [`PEAK_SLACK`].
    pub(super) render_peak: usize,
}

/// The longest answer that a test reads: 128 MiB.
pub(super) const ANSWER_LIMIT: usize = 128 * 1024 * 1024;
/// The size of the four value bodies of "The sum of change A": 4 × 48 MiB (ADR-0211 (p)).
const FOUR_BODIES: usize = 201_326_592;
/// The number of refused forms that round 1 sends at the same time.
const ROUND_FORMS: usize = 4;

/// Return the path of the page of `BIG`.
pub(super) fn page_path() -> String {
    format!("/recipes/{REFERENCE}/versions/1/parameters/{KEY}")
}

/// Return the request headers of `session` with the language and the extra headers.
pub(super) fn headers_of<'a>(
    cookie: &'a str,
    lang: (&'a str, &'a str),
    extra: &[(&'a str, &'a str)],
) -> Vec<(&'a str, &'a str)> {
    let mut headers = vec![("cookie", cookie), lang];
    headers.extend_from_slice(extra);
    headers
}

/// Write one line to the standard error stream, which the test harness does not capture.
///
/// The lint `print_stderr` denies `eprintln!`, so the line goes through a bound handle.
pub(super) fn report(line: &str) {
    let mut stderr = std::io::stderr().lock();
    writeln!(stderr, "{line}").unwrap();
}

/// The result of the render interval: the answer as a length and a hash, the peak, and the time.
pub(super) struct Rendered {
    pub(super) len: usize,
    pub(super) sha256: String,
    pub(super) peak: usize,
    pub(super) time: Duration,
}

/// The request of a session and the target of `BIG`, as the handlers get them.
pub(super) struct Loaded {
    pub(super) request: PageRequest,
    pub(super) target: Target,
}

impl Loaded {
    /// Return the context of a handler.
    pub(super) fn ctx<'a>(&'a self, state: &'a WebState) -> Ctx<'a> {
        Ctx {
            state,
            request: &self.request,
            target: &self.target,
        }
    }

    /// Return the length and the SHA-256 of `body` with the request id replaced.
    pub(super) fn digest_of(&self, body: &str) -> (usize, String) {
        let mut digest = BodyDigest::new(&self.request.request_id().to_string());
        digest.push(body.as_bytes());
        digest.finish()
    }
}

/// Build the `GET` request of the session number `session` with `headers`, and the target of `BIG`.
///
/// Solves the problem that a render copy needs the context of a handler, which only the extractor
/// builds. The method and the path matter only for the layout, so a copy of a `POST` answer uses
/// [`loaded_of_post`].
pub(super) async fn loaded_of(world: &World, session: u8, headers: &[(&str, &str)]) -> Loaded {
    loaded_with(world, session, headers, (Method::GET, page_path())).await
}

/// Build the `POST` request of the route `/definition` of `BIG`, as [`loaded_of`] builds a `GET`.
///
/// The layout of a `POST` answer has no language switch, so the copy of such an answer needs the
/// method of the request.
pub(super) async fn loaded_of_post(world: &World, session: u8, headers: &[(&str, &str)]) -> Loaded {
    loaded_with(world, session, headers, (Method::POST, definition_path())).await
}

/// Build the request of `session` with `headers`, `method`, and `uri`, and the target of `BIG`.
async fn loaded_with(
    world: &World,
    session: u8,
    headers: &[(&str, &str)],
    (method, uri): (Method, String),
) -> Loaded {
    let state = &world.fixture.state;
    let token = SessionToken::from_bytes([session; 16]);
    let auth = authenticate(state.identity(), &token)
        .await
        .unwrap()
        .unwrap();
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let (mut parts, ()) = builder.body(()).unwrap().into_parts();
    parts.extensions.insert(ClientAddr::of(PEER.ip()));
    parts.extensions.insert(Some(auth));
    let request = PageRequest::from_request_parts(&mut parts, state)
        .await
        .map_err(|_rejection| "rejected")
        .unwrap();
    let target = Target {
        reference: ReferenceNumber::parse(REFERENCE).unwrap(),
        number: 1,
        key: Some(ParameterKey::from(KEY)),
    };
    Loaded { request, target }
}

/// Print the line of an answer with the status `200` and check it against `pin`.
///
/// See [`check_status`].
pub(super) fn check_answer(
    pin: &Pin,
    measured: &Measured,
    rendered: &Rendered,
    request_time: Duration,
) {
    check_status(pin, StatusCode::OK, measured, rendered, request_time);
}

/// Print the line of an answer and check it against `pin` and the expected `status`.
///
/// The router answer and the render copy must have the pinned length and hash, so a drift between
/// the copy and the handler fails the test. The request peak must be at least the render peak.
pub(super) fn check_status(
    pin: &Pin,
    status: StatusCode,
    measured: &Measured,
    rendered: &Rendered,
    request_time: Duration,
) {
    let allowance = measured.peak.checked_sub(rendered.peak);
    let shown = allowance.map_or_else(|| "none".to_owned(), |bytes| bytes.to_string());
    report(&format!(
        "{}: len {} sha256 {} | request peak {} | render peak {} | allowance {shown} | \
         render time {} ms | request time {} ms | render len {} sha256 {}",
        pin.name,
        measured.len,
        measured.sha256,
        measured.peak,
        rendered.peak,
        rendered.time.as_millis(),
        request_time.as_millis(),
        rendered.len,
        rendered.sha256
    ));
    assert_eq!(measured.status, status);
    assert_eq!(measured.len, pin.len, "{}", pin.name);
    assert_eq!(measured.sha256, pin.sha256, "{}", pin.name);
    assert_eq!(rendered.len, pin.len, "{}: render interval", pin.name);
    assert_eq!(rendered.sha256, pin.sha256, "{}: render interval", pin.name);
    assert!(
        allowance.is_some(),
        "{}: render peak {} is above request peak {}",
        pin.name,
        rendered.peak,
        measured.peak
    );
    let request_bound = pin.request_peak.checked_add(PEAK_SLACK).unwrap();
    let render_bound = pin.render_peak.checked_add(PEAK_SLACK).unwrap();
    assert!(
        measured.peak <= request_bound,
        "{}: request peak {} exceeds {request_bound}",
        pin.name,
        measured.peak
    );
    assert!(
        rendered.peak <= render_bound,
        "{}: render peak {} exceeds {render_bound}",
        pin.name,
        rendered.peak
    );
}

/// Return the path of the route `/definition` of `BIG`.
pub(super) fn definition_path() -> String {
    format!("{}/definition", page_path())
}

/// Build the `POST` of an edit form. The copy of the body happens here, inside the measured
/// interval.
pub(super) fn definition_request(
    headers: &[(&str, &str)],
    csrf: &str,
    body: &str,
) -> Request<Body> {
    let mut builder = request(Method::POST, &definition_path())
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("x-csrf-token", csrf);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    builder.body(Body::from(body.to_owned())).unwrap()
}

/// Return the number of the occurrences of `needle` in `text`.
pub(super) fn count(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

/// Return the sum of "The sum of change A": the four bodies and four times the render peak.
pub(super) fn change_a_sum(render_peak: usize) -> usize {
    render_peak
        .checked_mul(ROUND_FORMS)
        .and_then(|pages| pages.checked_add(FOUR_BODIES))
        .unwrap()
}
