//! The CSRF layer for every method except `GET`, `HEAD`, and `OPTIONS` (design D11 step 8, D12,
//! ADR-0035 e, s, t).
//!
//! The layer solves one problem: a new mutating route must not skip the check by accident. It
//! runs for every route in the table. It first checks the origin, then the token. A failed check
//! writes the security event `csrf_rejected` and gives `403 E-WEB-007`.

use axum::body::{Body, Bytes, to_bytes};
use axum::extract::{Request, State};
use axum::http::header::{CONTENT_TYPE, HOST, ORIGIN, USER_AGENT};
use axum::http::{HeaderMap, Method};
use axum::middleware::Next;
use axum::response::Response;
use recipe_app::auth::Operation;
use recipe_app::error::AppError;
use recipe_app::identity::Authenticated;
use recipe_domain::audit::{SecurityEvent, UserAgent};
use subtle::ConstantTimeEq as _;

use crate::client::ClientAddr;
use crate::cookies::{CSRF_COOKIE, request_cookie};
use crate::error::{ByteLimit, WebError};
use crate::limits::request_limit;
use crate::pipeline::request_id_of;
use crate::routes::Access;
use crate::state::WebState;
use crate::timeout::{is_body_timeout, timed_out};

mod form;
mod pre_session;

use pre_session::verify_pre_session;
pub use pre_session::{PreSession, issue_pre_session};

/// The header that carries the session token of HTMX requests.
const X_CSRF_TOKEN: &str = "x-csrf-token";
/// The header that a browser sets with the relation of the request to the site.
///
/// The CSRF layer and the guard of the export routes (ADR-0055) read it.
pub(crate) const SEC_FETCH_SITE: &str = "sec-fetch-site";
/// The value of `Sec-Fetch-Site` for a request from the same origin.
///
/// The CSRF layer and the guard of the export routes (ADR-0055) compare with it.
pub(crate) const SAME_ORIGIN: &str = "same-origin";
/// The scheme of an `Origin` that matches the site. The site runs behind HTTPS only.
const ORIGIN_PREFIX: &str = "https://";
/// The form field of the session token.
const SESSION_FIELD: &str = "csrf_token";
/// The form field of the pre-session token.
const PRE_SESSION_FIELD: &str = "csrf_pre";

/// Compare two byte strings in constant time. Strings of different length are not equal.
fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    left.ct_eq(right).into()
}

/// Tell whether the method changes state.
///
/// The function solves one problem: a list of mutating methods lets a later `PATCH` route or an
/// unusual method skip the checks. Only `GET`, `HEAD`, and `OPTIONS` are safe. Every other method
/// counts as mutating.
pub(crate) fn is_mutating(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// Which token a route accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenKind {
    /// The token of the session.
    Session,
    /// The signed double-submit token of the cookie `__Host-recipe_csrf`.
    PreSession,
    /// Either of the two.
    Either,
}

/// Return the token kind of a route (`api.md` section 7.3).
///
/// `POST /login` always uses the pre-session token. `POST /lang` accepts both. Every other route
/// needs the session token.
fn token_kind(access: Access) -> TokenKind {
    match access {
        Access::Lang => TokenKind::Either,
        Access::Operation(Operation::Login) => TokenKind::PreSession,
        Access::Operation(_) | Access::Static => TokenKind::Session,
    }
}

/// Tell whether the request comes from the site (ADR-0035 e).
///
/// `Sec-Fetch-Site: same-origin` passes. Otherwise `Origin` must equal `https://` and the `Host`
/// header. `Origin: null`, another scheme, another port, and a missing `Host` fail.
fn origin_matches(headers: &HeaderMap) -> bool {
    let fetched_from_site = headers
        .get(SEC_FETCH_SITE)
        .is_some_and(|value| value.as_bytes() == SAME_ORIGIN.as_bytes());
    if fetched_from_site {
        return true;
    }
    let (Some(origin), Some(host)) = (
        headers.get(ORIGIN).and_then(|value| value.to_str().ok()),
        headers.get(HOST).and_then(|value| value.to_str().ok()),
    ) else {
        return false;
    };
    !host.is_empty() && origin.strip_prefix(ORIGIN_PREFIX) == Some(host)
}

/// Read a request body up to `limit` bytes.
///
/// A body above the limit gives `E-WEB-003`. A body that breaks off before its end gives the same
/// answer, because the client cannot read an answer anyway. A body that misses its deadline gives
/// `E-WEB-009` (ADR-0058 (j)).
async fn buffer_body(body: Body, limit: usize) -> Result<Bytes, WebError> {
    to_bytes(body, limit).await.map_err(|error| {
        if is_body_timeout(&error) {
            timed_out()
        } else {
            WebError::PayloadTooLarge {
                limit: ByteLimit::from(limit),
            }
        }
    })
}

/// Return the value of the header `X-CSRF-Token`. A value that is not text counts as empty.
fn header_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(X_CSRF_TOKEN)
        .map(|value| value.to_str().unwrap_or_default().to_owned())
}

/// Tell whether the token of the request equals the token of its session.
///
/// The header `X-CSRF-Token` wins over the form field. The comparison runs in constant time.
fn session_token_matches(
    session: Option<&Authenticated>,
    headers: &HeaderMap,
    body: &[u8],
    is_form: bool,
) -> bool {
    let Some(session) = session else {
        return false;
    };
    let submitted = header_token(headers).or_else(|| {
        is_form
            .then(|| form::form_field(body, SESSION_FIELD))
            .flatten()
    });
    let Some(submitted) = submitted else {
        return false;
    };
    constant_time_equal(
        session.csrf_token().to_hex().as_bytes(),
        submitted.as_bytes(),
    )
}

/// Tell whether the request carries a valid pre-session token.
fn pre_session_matches(state: &WebState, headers: &HeaderMap, body: &[u8], is_form: bool) -> bool {
    let cookie = request_cookie(headers, CSRF_COOKIE);
    let field = is_form
        .then(|| form::form_field(body, PRE_SESSION_FIELD))
        .flatten();
    verify_pre_session(state.csrf_keys(), cookie.as_deref(), field.as_deref())
}

/// Build the event `csrf_rejected` of a request.
///
/// The event holds the request id, the client address, and the `User-Agent`. It holds no token.
/// A request without a client address is a defect of the pipeline and gives `E-WEB-010`. The
/// CSRF layer and the guard of the export routes (ADR-0055) call it.
pub(crate) fn rejection_event(request: &Request) -> Result<SecurityEvent, WebError> {
    let request_id = request_id_of(request);
    let Some(client) = request.extensions().get::<ClientAddr>().copied() else {
        return Err(WebError::Internal { request_id });
    };
    let agent = request
        .headers()
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(UserAgent::from);
    Ok(SecurityEvent::csrf_rejected(request_id, client.ip(), agent))
}

/// Write the event and return the error of the request: `E-WEB-007`.
///
/// The function takes the event, not the request, because a request body is not `Sync` and the
/// future must stay `Send`. When the write fails, the request gets the storage error (ADR-0035 t).
/// The CSRF layer and the guard of the export routes (ADR-0055) call it.
pub(crate) async fn reject(state: &WebState, event: Result<SecurityEvent, WebError>) -> WebError {
    let event = match event {
        Ok(event) => event,
        Err(error) => return error,
    };
    match state.security().record(event).await {
        Ok(()) => WebError::CsrfRejected,
        Err(error) => WebError::App(AppError::Storage(error)),
    }
}

/// Check the origin and the token of every mutating request (see [`is_mutating`]).
///
/// The layer buffers the body, which the body limit of step 5 bounds, and puts it back for the
/// handler. A request that matched no route goes on, because the fallback serves no data.
pub(crate) async fn csrf_layer(
    State(state): State<WebState>,
    request: Request,
    next: Next,
) -> Result<Response, WebError> {
    let access = request.extensions().get::<Access>().copied();
    let Some(access) = access.filter(|_listed| is_mutating(request.method())) else {
        return Ok(next.run(request).await);
    };
    if !origin_matches(request.headers()) {
        return Err(reject(&state, rejection_event(&request)).await);
    }
    let limit = request_limit(state.settings(), &request);
    let (parts, body) = request.into_parts();
    let body = buffer_body(body, limit).await?;
    let is_form = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(form::is_form_content_type);
    let session = parts
        .extensions
        .get::<Option<Authenticated>>()
        .and_then(Option::as_ref);
    let kind = token_kind(access);
    let by_session = session_token_matches(session, &parts.headers, &body, is_form);
    let by_cookie = pre_session_matches(&state, &parts.headers, &body, is_form);
    let accepted = match kind {
        TokenKind::Session => by_session,
        TokenKind::PreSession => by_cookie,
        TokenKind::Either => by_session | by_cookie,
    };
    let request = Request::from_parts(parts, Body::from(body));
    if !accepted {
        return Err(reject(&state, rejection_event(&request)).await);
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
mod limit_tests;
#[cfg(test)]
mod tests;
