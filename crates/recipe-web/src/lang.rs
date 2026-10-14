//! `POST /lang`: switch the UI language (`api.md` section 8, design D12 item o).
//!
//! The route sets the cookie `recipe_lang` and redirects. It writes no database row, so it works
//! before login and in read-only mode. It never reads the `Referer` header.

use axum::extract::rejection::FormRejection;
use axum::extract::{Form, State};
use axum::http::header::{LOCATION, SET_COOKIE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::cookies::lang_cookie;
use crate::error::{FieldReason, WebError};
use crate::i18n::Lang;
use crate::state::WebState;

/// The page that a redirect uses when `next` is missing or not a same-origin path.
const DEFAULT_NEXT: &str = "/recipes";

/// The fields of the form of `POST /lang`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LangForm {
    lang: String,
    #[serde(default)]
    next: String,
    /// The pre-session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_pre")]
    _csrf_pre: Option<String>,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// Return `next` when it is a same-origin relative path, else `None`.
///
/// The path starts with `/`, and its second character is neither `/` nor `\`. It holds no `\`
/// and no control or white-space character (ADR-0035).
fn same_origin_path(next: &str) -> Option<&str> {
    let mut chars = next.chars();
    let first = chars.next()?;
    let second = chars.next();
    let starts_well = first == '/' && !matches!(second, Some('/' | '\\'));
    let clean = next
        .chars()
        .all(|c| c != '\\' && !c.is_control() && !c.is_whitespace());
    (starts_well && clean).then_some(next)
}

/// Write every byte outside ASCII as `%XX`, so the `Location` header holds visible ASCII only.
fn percent_encode_non_ascii(path: &str) -> String {
    path.bytes()
        .fold(String::with_capacity(path.len()), |mut out, byte| {
            if byte.is_ascii() {
                out.push(char::from(byte));
            } else {
                out.push_str(&format!("%{byte:02X}"));
            }
            out
        })
}

/// Build the `Location` value. A text that is not a same-origin path goes to the default page.
fn location_of(next: &str) -> HeaderValue {
    same_origin_path(next)
        .and_then(|path| HeaderValue::from_str(&percent_encode_non_ascii(path)).ok())
        .unwrap_or_else(|| HeaderValue::from_static(DEFAULT_NEXT))
}

/// Handle `POST /lang`.
pub(crate) async fn switch(
    State(state): State<WebState>,
    form: Result<Form<LangForm>, FormRejection>,
) -> Result<Response, WebError> {
    let Form(form) = form?;
    let lang = Lang::parse(&form.lang).map_err(|_unsupported| WebError::BadRequest {
        field: "lang",
        reason: FieldReason::Lang,
    })?;
    let max_age = state.settings().lang_cookie_max_age;
    let mut response = StatusCode::SEE_OTHER.into_response();
    response
        .headers_mut()
        .insert(LOCATION, location_of(&form.next));
    if let Some(value) = lang_cookie(lang, max_age) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
    Ok(response)
}
