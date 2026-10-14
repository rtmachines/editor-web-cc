//! The cookies of the web layer (`api.md` sections 7.1 and 8, design D12).
//!
//! The `cookie` crate builds and parses the cookies, so no code writes an attribute list by hand.

use axum::http::header::COOKIE;
use axum::http::header::InvalidHeaderValue;
use axum::http::{HeaderMap, HeaderValue};
use cookie::time::Duration;
use cookie::{Cookie, SameSite};
use recipe_domain::identity::SessionToken;

use crate::i18n::{LANG_COOKIE, Lang};

/// The name of the session cookie.
pub(crate) const SESSION_COOKIE: &str = "__Host-recipe_session";
/// The name of the pre-session CSRF cookie.
pub(crate) const CSRF_COOKIE: &str = "__Host-recipe_csrf";
/// The text of the cookie that ends the session, for the case that the builder fails.
const CLEAR_SESSION_FALLBACK: &str =
    "__Host-recipe_session=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0";

/// Return the value of the first cookie called `name` in the `Cookie` headers of a request.
///
/// A header that is not text, or a part that is not a cookie, is skipped.
pub(crate) fn request_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|line| line.to_str().ok())
        .flat_map(Cookie::split_parse)
        .filter_map(Result::ok)
        .find(|cookie| cookie.name() == name)
        .map(|cookie| cookie.value().to_owned())
}

/// Build the `Set-Cookie` value of a new session (`api.md` section 7.1).
///
/// The cookie holds the 32 hex characters of the token and nothing else. It has `HttpOnly`,
/// `Secure`, `SameSite=Strict`, and `Path=/`. It has no `Domain`, as the prefix `__Host-` demands,
/// and no `Max-Age`, because the server enforces both timeouts.
///
/// # Errors
///
/// Returns the error of `HeaderValue` when the text is not a valid header. A hex text always is.
pub(crate) fn session_cookie(token: &SessionToken) -> Result<HeaderValue, InvalidHeaderValue> {
    let cookie = Cookie::build((SESSION_COOKIE, token.to_hex()))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .build();
    HeaderValue::from_str(&cookie.to_string())
}

/// Build the `Set-Cookie` value that ends the session cookie with `Max-Age=0`.
pub(crate) fn clear_session_cookie() -> HeaderValue {
    let cookie = Cookie::build((SESSION_COOKIE, ""))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(Duration::ZERO)
        .build();
    HeaderValue::from_str(&cookie.to_string())
        .unwrap_or_else(|_invalid| HeaderValue::from_static(CLEAR_SESSION_FALLBACK))
}

/// Build the `Set-Cookie` value of the language cookie `recipe_lang` (`api.md` section 8).
///
/// The cookie holds the code of the language. It has `Secure`, `SameSite=Strict`, `Path=/`, and
/// `Max-Age` of `max_age`. Returns `None` when `max_age` does not fit the cookie or the text is not
/// a valid header. The caller then sets no cookie.
pub(crate) fn lang_cookie(lang: Lang, max_age: std::time::Duration) -> Option<HeaderValue> {
    let max_age = Duration::try_from(max_age).ok()?;
    let cookie = Cookie::build((LANG_COOKIE, lang.as_str()))
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(max_age)
        .build();
    HeaderValue::from_str(&cookie.to_string()).ok()
}
