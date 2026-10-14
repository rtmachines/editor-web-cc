//! The vendored static files (`api.md` sections 2.1 and 9, design D3).
//!
//! The binary embeds each file with `include_bytes!`, so no file system access happens at run time.

use axum::extract::State;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use axum::routing::{MethodRouter, get};

use crate::state::WebState;

/// The version of the vendored HTMX file. It is the newest HTMX 2 release at vendoring time.
pub const HTMX_VERSION: &str = "2.0.11";

/// The SHA-256 of `static/htmx.min.js` in lower-case hex. A test hashes the embedded bytes.
pub const HTMX_SHA256: &str = "d6fdc75f204e6bdefa99b69bf1e6d4ac69b8a364f77929f45c13476b4000f717";

/// The HTMX file as downloaded from `unpkg.com/htmx.org@2.0.11/dist/htmx.min.js`, unchanged.
pub(crate) const HTMX_BYTES: &[u8] = include_bytes!("../static/htmx.min.js");

/// The style sheet of every page (`design-system.md`).
pub(crate) const APP_CSS_BYTES: &[u8] = include_bytes!("../static/app.css");

/// The script that copies the new `row_version` into the forms (`api.md` section 9).
pub(crate) const ROW_VERSION_BYTES: &[u8] = include_bytes!("../static/row-version.js");

/// The script that opens and closes the groups of a tree (`design-system.md` section 10.8).
pub(crate) const TREE_BYTES: &[u8] = include_bytes!("../static/tree.js");

/// The media type of a script.
const JAVASCRIPT: &str = "application/javascript";

/// The media type of the style sheet.
const CSS: &str = "text/css; charset=utf-8";

/// The media type of a font file.
const WOFF2: &str = "font/woff2";

/// One embedded font file (design D3).
///
/// The type solves one problem: the route table needs a fixed path, the bytes, and the pinned
/// hash of each font in one place, so a changed file fails a test.
pub(crate) struct FontFile {
    /// The route path of the file, `/static/fonts/` and the file name.
    pub(crate) path: &'static str,
    /// The embedded bytes.
    pub(crate) bytes: &'static [u8],
}

/// Build one `FontFile` from a file name.
macro_rules! font {
    ($name:literal) => {
        FontFile {
            path: concat!("/static/fonts/", $name),
            bytes: include_bytes!(concat!("../static/fonts/", $name)),
        }
    };
}

/// The 8 font files of `design-system.md` section 5.
///
/// Source: the npm packages `@fontsource/atkinson-hyperlegible-next@5.3.0` and
/// `@fontsource/atkinson-hyperlegible-mono@5.3.0`, license OFL-1.1.
pub(crate) const FONTS: [FontFile; 8] = [
    font!("atkinson-hyperlegible-next-latin-400-normal.woff2"),
    font!("atkinson-hyperlegible-next-latin-ext-400-normal.woff2"),
    font!("atkinson-hyperlegible-next-latin-700-normal.woff2"),
    font!("atkinson-hyperlegible-next-latin-ext-700-normal.woff2"),
    font!("atkinson-hyperlegible-mono-latin-400-normal.woff2"),
    font!("atkinson-hyperlegible-mono-latin-ext-400-normal.woff2"),
    font!("atkinson-hyperlegible-mono-latin-700-normal.woff2"),
    font!("atkinson-hyperlegible-mono-latin-ext-700-normal.woff2"),
];

/// Build the answer for one static file: the media type, the cache rule, and the body.
fn asset(state: &WebState, media_type: &'static str, body: &'static [u8]) -> Response {
    let max_age = state.settings().static_cache_max_age.as_secs();
    (
        [
            (CONTENT_TYPE, media_type.to_owned()),
            (CACHE_CONTROL, format!("public, max-age={max_age}")),
        ],
        body,
    )
        .into_response()
}

/// One embedded script or style sheet.
///
/// The type solves one problem: the route table needs the path, the bytes, and the media type
/// of each file in one place, so no handler repeats the same few lines.
pub(crate) struct StaticFile {
    /// The route path of the file.
    pub(crate) path: &'static str,
    /// The embedded bytes.
    pub(crate) bytes: &'static [u8],
    /// The value of `Content-Type`.
    pub(crate) media_type: &'static str,
}

/// The four files of `api.md` section 9, in the order of the route table.
pub(crate) const STATIC_FILES: [StaticFile; 4] = [
    StaticFile {
        path: "/static/htmx.min.js",
        bytes: HTMX_BYTES,
        media_type: JAVASCRIPT,
    },
    StaticFile {
        path: "/static/app.css",
        bytes: APP_CSS_BYTES,
        media_type: CSS,
    },
    StaticFile {
        path: "/static/row-version.js",
        bytes: ROW_VERSION_BYTES,
        media_type: JAVASCRIPT,
    },
    StaticFile {
        path: "/static/tree.js",
        bytes: TREE_BYTES,
        media_type: JAVASCRIPT,
    },
];

/// Return the handler of one script or style sheet route.
///
/// The route needs no session. `Cache-Control` is `public, max-age` with `static.cache_max_age`.
pub(crate) fn static_handler(file: &'static StaticFile) -> MethodRouter<WebState> {
    get(
        move |State(state): State<WebState>| async move { asset(&state, file.media_type, file.bytes) },
    )
}

/// Return the handler of one font route.
///
/// The route needs no session. `Cache-Control` decides the caching, so the answer has no `ETag`.
pub(crate) fn font_handler(file: &'static FontFile) -> MethodRouter<WebState> {
    get(move |State(state): State<WebState>| async move { asset(&state, WOFF2, file.bytes) })
}

#[cfg(test)]
mod tests;
