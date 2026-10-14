//! Request pipeline layers (design D11, steps 1 to 3).
//!
//! `wrap` puts the request id outside, the trace layer inside it, and the security headers inside
//! the trace layer. A layer that is added later runs earlier, so the request id exists for every
//! answer, also for an early rejection. The layers of steps 4 to 9 sit in `routes::assemble`.

use axum::Router;
use axum::extract::{Request, State};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::http::header::{CACHE_CONTROL, CONTENT_SECURITY_POLICY, REFERRER_POLICY, USER_AGENT};
use axum::http::{Extensions, HeaderName, HeaderValue};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::Response;
use recipe_app::identity::random_nonce;
use recipe_domain::audit::{RequestId, UserAgent};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing::Span;
use uuid::Uuid;

use crate::settings::WebSettings;

/// The value of `Content-Security-Policy` (`api.md` section 9).
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";
/// The header that carries the request id.
const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
/// The header that HTMX sets on each request it sends.
pub(crate) const HX_REQUEST: &str = "hx-request";
/// The header that stops content sniffing.
const X_CONTENT_TYPE_OPTIONS: HeaderName = HeaderName::from_static("x-content-type-options");
/// The header that forces HTTPS.
const STRICT_TRANSPORT_SECURITY: HeaderName = HeaderName::from_static("strict-transport-security");
/// Number of bytes of a UUID v7 after the 48 bit timestamp, as `Builder` takes them.
const TAIL_BYTES: usize = 10;
/// Number of nanoseconds in one millisecond.
const NANOS_PER_MILLI: u32 = 1_000_000;
/// The value of `Strict-Transport-Security` when the setting gives no valid text.
const HSTS_FALLBACK: &str = "max-age=31536000; includeSubDomains";

/// Give each request a new UUID v7 and return it in `X-Request-Id`.
///
/// The function ignores an `X-Request-Id` header of the client, so a client cannot choose the
/// text that the log and the audit entries hold. Handlers read the id from the request
/// extension [`RequestId`].
async fn request_id(
    State(counter): State<Arc<AtomicU64>>,
    mut request: Request,
    next: Next,
) -> Response {
    let (millis, nanos) = clock_parts();
    let sequence = counter.fetch_add(1, Ordering::Relaxed);
    let random = random_nonce().ok();
    let id = RequestId::from(build_request_id(millis, random, nanos, sequence));
    request.extensions_mut().insert(id);
    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&id.to_string()) {
        response.headers_mut().insert(X_REQUEST_ID, value);
    }
    response
}

/// Return the milliseconds since 1970 and the nanoseconds inside the current millisecond.
///
/// A clock before 1970 gives 0 and 0.
fn clock_parts() -> (u64, u32) {
    let Ok(elapsed) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return (0, 0);
    };
    let millis = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    (millis, elapsed.subsec_nanos() % NANOS_PER_MILLI)
}

/// Build a UUID v7 from the clock and ten more bytes.
///
/// The function solves one problem: `Uuid::now_v7` panics when the operating system gives no
/// random bytes, and the request id must exist for every answer. The ten bytes come from
/// `random` when the read worked. Else they hold `nanos` and the low six bytes of `sequence`.
/// The sequence differs for each call, so two ids differ also when the clock value repeats.
fn build_request_id(millis: u64, random: Option<[u8; 16]>, nanos: u32, sequence: u64) -> Uuid {
    let mut tail = [0_u8; TAIL_BYTES];
    let spare = nanos
        .to_be_bytes()
        .into_iter()
        .chain(sequence.to_be_bytes().into_iter().skip(2));
    match random {
        Some(bytes) => fill(&mut tail, bytes),
        None => fill(&mut tail, spare),
    }
    uuid::Builder::from_unix_timestamp_millis(millis, &tail).into_uuid()
}

/// Copy the first bytes of `source` into `target`.
fn fill(target: &mut [u8], source: impl IntoIterator<Item = u8>) {
    for (slot, byte) in target.iter_mut().zip(source) {
        *slot = byte;
    }
}

/// Return the id that the layer `request_id` stored in `extensions`.
///
/// A request that did not pass that layer gets the nil id. Only a test builds such a request.
pub(crate) fn request_id_in(extensions: &Extensions) -> RequestId {
    extensions
        .get::<RequestId>()
        .copied()
        .unwrap_or_else(|| RequestId::from(Uuid::nil()))
}

/// Return the id that the layer `request_id` stored in `request`.
///
/// A request that did not pass that layer gets the nil id. Only a test builds such a request.
pub(crate) fn request_id_of(request: &Request) -> RequestId {
    request_id_in(request.extensions())
}

/// Create the span of a request (`reliability.md` section 3.2).
///
/// The span holds the request id, the method, and the `User-Agent` cut to 256 characters. The
/// layers that learn the route, the client address, the session, and the answer record the other
/// fields later. The span holds no header besides `User-Agent`, no query, and no body.
fn make_span(request: &Request) -> Span {
    let agent = request
        .headers()
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|text| UserAgent::from(text).as_str().to_owned());
    tracing::info_span!(
        "http_request",
        request_id = %request_id_of(request),
        method = %request.method(),
        user_agent = agent.as_deref(),
        route = tracing::field::Empty,
        client_ip = tracing::field::Empty,
        session_ref = tracing::field::Empty,
        user_id = tracing::field::Empty,
        role = tracing::field::Empty,
        status = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

/// Record the status and the latency, then write the one `INFO` line of the request.
fn finish_span(response: &Response, latency: Duration, span: &Span) {
    let millis = u64::try_from(latency.as_millis()).unwrap_or(u64::MAX);
    span.record("status", response.status().as_u16());
    span.record("latency_ms", millis);
    tracing::info!("request finished");
}

/// Return the value of `Strict-Transport-Security` for `settings`.
fn hsts_value(settings: &WebSettings) -> HeaderValue {
    let text = format!(
        "max-age={}; includeSubDomains",
        settings.hsts_max_age.as_secs()
    );
    HeaderValue::from_str(&text).unwrap_or_else(|_invalid| HeaderValue::from_static(HSTS_FALLBACK))
}

/// Add the request id and the security headers of `api.md` section 9 to every answer of `router`.
///
/// `Cache-Control: no-store` is the default. A handler that sets its own value, as the static
/// route does, keeps it.
pub(crate) fn wrap(router: Router, settings: &WebSettings) -> Router {
    router
        .layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            STRICT_TRANSPORT_SECURITY,
            hsts_value(settings),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            REFERRER_POLICY,
            HeaderValue::from_static("same-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CSP),
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_span)
                .on_request(())
                .on_response(finish_span)
                .on_eos(())
                .on_failure(()),
        )
        .layer(from_fn_with_state(Arc::new(AtomicU64::new(0)), request_id))
}

#[cfg(test)]
mod tests;
