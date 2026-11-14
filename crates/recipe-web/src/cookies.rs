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

/// The name of the session cookie with `session.secure` true.
const SESSION_COOKIE_SECURE: &str = "__Host-recipe_session";
/// The name of the session cookie with `session.secure` false. A browser refuses `__Host-` without `Secure`.
const SESSION_COOKIE_PLAIN: &str = "recipe_session";
/// The name of the pre-session CSRF cookie with `session.secure` true.
const CSRF_COOKIE_SECURE: &str = "__Host-recipe_csrf";
/// The name of the pre-session CSRF cookie with `session.secure` false.
const CSRF_COOKIE_PLAIN: &str = "recipe_csrf";
/// The text of the cookie that ends the session with `secure` true, for the case that the builder fails.
const CLEAR_SESSION_FALLBACK_SECURE: &str =
    "__Host-recipe_session=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0";
/// The text of the cookie that ends the session with `secure` false, for the case that the builder fails.
const CLEAR_SESSION_FALLBACK_PLAIN: &str =
    "recipe_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";

/// Return the name of the session cookie for the value of `session.secure` (ADR-0171).
///
/// The function solves one problem: every writer and reader of the cookie must use one name in
/// one process.
pub(crate) const fn session_cookie_name(secure: bool) -> &'static str {
    if secure {
        SESSION_COOKIE_SECURE
    } else {
        SESSION_COOKIE_PLAIN
    }
}

/// Return the name of the pre-session CSRF cookie for the value of `session.secure` (ADR-0171).
pub(crate) const fn csrf_cookie_name(secure: bool) -> &'static str {
    if secure {
        CSRF_COOKIE_SECURE
    } else {
        CSRF_COOKIE_PLAIN
    }
}

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
/// `SameSite=Strict`, and `Path=/`. It has `Secure` and the prefix `__Host-` only when `secure` is
/// true (ADR-0171). It has no `Domain`, as the prefix demands, and no `Max-Age`, because the
/// server enforces both timeouts.
///
/// # Errors
///
/// Returns the error of `HeaderValue` when the text is not a valid header. A hex text always is.
pub(crate) fn session_cookie(
    token: &SessionToken,
    secure: bool,
) -> Result<HeaderValue, InvalidHeaderValue> {
    let cookie = Cookie::build((session_cookie_name(secure), token.to_hex()))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .path("/")
        .build();
    HeaderValue::from_str(&cookie.to_string())
}

/// Build the `Set-Cookie` value that ends the session cookie with `Max-Age=0`.
pub(crate) fn clear_session_cookie(secure: bool) -> HeaderValue {
    let cookie = Cookie::build((session_cookie_name(secure), ""))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(Duration::ZERO)
        .build();
    HeaderValue::from_str(&cookie.to_string()).unwrap_or_else(|_invalid| {
        HeaderValue::from_static(if secure {
            CLEAR_SESSION_FALLBACK_SECURE
        } else {
            CLEAR_SESSION_FALLBACK_PLAIN
        })
    })
}

/// Build the `Set-Cookie` value of the language cookie `recipe_lang` (`api.md` section 8).
///
/// The cookie holds the code of the language. It has `SameSite=Strict`, `Path=/`, and `Max-Age` of
/// `max_age`. It has `Secure` only when `secure` is true (ADR-0171). Returns `None` when `max_age` does not fit the cookie or the text is not
/// a valid header. The caller then sets no cookie.
pub(crate) fn lang_cookie(
    lang: Lang,
    max_age: std::time::Duration,
    secure: bool,
) -> Option<HeaderValue> {
    let max_age = Duration::try_from(max_age).ok()?;
    let cookie = Cookie::build((LANG_COOKIE, lang.as_str()))
        .secure(secure)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(max_age)
        .build();
    HeaderValue::from_str(&cookie.to_string()).ok()
}

#[cfg(test)]
mod tests;
