//! The vendored static files (`api.md` sections 2.1 and 9, design D13).
//!
//! The binary embeds each file with `include_bytes!`, so no file system access happens at run time.

use axum::extract::State;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::response::IntoResponse;

use crate::state::WebState;

/// The version of the vendored HTMX file. It is the newest HTMX 2 release at vendoring time.
pub const HTMX_VERSION: &str = "2.0.11";

/// The SHA-256 of `static/htmx.min.js` in lower-case hex. A test hashes the embedded bytes.
pub const HTMX_SHA256: &str = "d6fdc75f204e6bdefa99b69bf1e6d4ac69b8a364f77929f45c13476b4000f717";

/// The HTMX file as downloaded from `unpkg.com/htmx.org@2.0.11/dist/htmx.min.js`, unchanged.
pub(crate) const HTMX_BYTES: &[u8] = include_bytes!("../static/htmx.min.js");

/// The media type of a script.
const JAVASCRIPT: &str = "application/javascript";

/// Serve `GET /static/htmx.min.js`.
///
/// The route needs no session. `Cache-Control` is `public, max-age` with `static.cache_max_age`.
pub(crate) async fn htmx(State(state): State<WebState>) -> impl IntoResponse {
    let max_age = state.settings().static_cache_max_age.as_secs();
    (
        [
            (CONTENT_TYPE, JAVASCRIPT.to_owned()),
            (CACHE_CONTROL, format!("public, max-age={max_age}")),
        ],
        HTMX_BYTES,
    )
}

#[cfg(test)]
mod tests;
