//! The pre-session CSRF token (design D12, ADR-0035 s, `api.md` section 7.3).
//!
//! A signed double-submit cookie protects `POST /login` and `POST /lang` before a session exists.
//! The cookie `__Host-recipe_csrf` holds `hex(nonce) "." hex(HMAC-SHA256(nonce))`. The form field
//! `csrf_pre` holds the hex nonce. A browser keeps the cookie for its session only.

use std::fmt;

use axum::http::HeaderValue;
use cookie::{Cookie, SameSite};
use recipe_app::error::AppError;
use recipe_app::identity::{NONCE_BYTES, random_nonce};
use recipe_domain::identity::KeyRing;

use crate::cookies::CSRF_COOKIE;
use crate::hex::{from_lower_hex, to_hex};

/// Bytes in the HMAC-SHA256 tag.
const TAG_BYTES: usize = 32;
/// Text that `Debug` prints in place of the nonce.
const REDACTED: &str = "[redacted]";

/// A pre-session token that the server just issued.
///
/// The type solves one problem: `GET /login` must put the cookie in the response and the nonce
/// in the form, and it must not log either one. `Debug` redacts the nonce and the cookie.
pub struct PreSession {
    nonce: String,
    set_cookie: HeaderValue,
}

impl PreSession {
    /// Return the nonce for the form field `csrf_pre`, as 32 lower-case hex characters.
    #[must_use]
    pub fn nonce(&self) -> &str {
        &self.nonce
    }

    /// Return the `Set-Cookie` value of the cookie `__Host-recipe_csrf`.
    ///
    /// The cookie has `HttpOnly`, `Secure`, `SameSite=Strict`, and `Path=/`. It has no `Max-Age`
    /// and no `Domain`, as the prefix `__Host-` requires.
    #[must_use]
    pub const fn set_cookie(&self) -> &HeaderValue {
        &self.set_cookie
    }
}

impl fmt::Debug for PreSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreSession")
            .field("nonce", &REDACTED)
            .field("set_cookie", &REDACTED)
            .finish()
    }
}

/// Issue a new pre-session token signed under the first key of `keys`.
///
/// The nonce has 128 random bits from `recipe_app::identity::random_nonce`. It has no panic path.
///
/// # Errors
///
/// Returns `AppError::TaskFailed` with the stage `Token` (`E-WEB-011`) when the system gives no
/// random bytes.
pub fn issue_pre_session(keys: &KeyRing) -> Result<PreSession, AppError> {
    let nonce_bytes = random_nonce()?;
    let nonce = to_hex(&nonce_bytes);
    let tag = to_hex(&keys.sign(&nonce_bytes));
    let cookie = Cookie::build((CSRF_COOKIE, format!("{nonce}.{tag}")))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .build();
    // The text is ASCII, so the conversion cannot fail. An empty value would set no cookie, and
    // the next `POST` would be rejected.
    let set_cookie = HeaderValue::from_str(&cookie.to_string())
        .unwrap_or_else(|_invalid| HeaderValue::from_static(""));
    Ok(PreSession { nonce, set_cookie })
}

/// Tell whether the cookie value and the form field make a valid pre-session token.
///
/// The cookie must hold a nonce of 16 bytes and a tag that one key of `keys` made. The field must
/// equal the nonce text. Both parts use lower-case hex. The comparison of the nonce runs in
/// constant time, and `KeyRing::verify` checks the tag in constant time.
pub(crate) fn verify_pre_session(
    keys: &KeyRing,
    cookie_value: Option<&str>,
    field: Option<&str>,
) -> bool {
    let (Some(value), Some(field)) = (cookie_value, field) else {
        return false;
    };
    let Some((nonce_text, tag_text)) = value.split_once('.') else {
        return false;
    };
    let (Some(nonce), Some(tag)) = (
        from_lower_hex(nonce_text, NONCE_BYTES),
        from_lower_hex(tag_text, TAG_BYTES),
    ) else {
        return false;
    };
    let signed = keys.verify(&nonce, &tag);
    let same = super::constant_time_equal(nonce_text.as_bytes(), field.as_bytes());
    signed & same
}
