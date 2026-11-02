//! The parser of the `Authorization: Basic` header (`api.md` section 7.7, ADR-0109 (ma)).

use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use recipe_app::identity::BasicCredentials;
use recipe_domain::identity::{Password, TypedLogin};

use crate::error::WebError;

/// The longest `Authorization` header in bytes (`export.basic_header_max`, `reliability.md` 7.11).
pub(crate) const BASIC_HEADER_MAX: usize = 1024;

/// The scheme of the header. The parser compares it without a difference of case.
const SCHEME: &str = "Basic";

/// Parse the `Authorization` header of a Basic route.
///
/// The function solves one problem: the header comes from a client and holds a password, so one
/// place checks it before any hash work. It checks the count and the length first, then the
/// scheme, the standard Base64 with padding, UTF-8, and the first `:`. The login and the password
/// then pass through their value objects, and a control character in either part is a fault.
/// The function logs nothing. Each fault gives the one error `MissingBasicCredentials`.
///
/// # Errors
///
/// Returns [`WebError::MissingBasicCredentials`] for every case of ADR-0109 (ma), and when the
/// request has no `Authorization` header.
pub(crate) fn parse_basic(headers: &HeaderMap) -> Result<BasicCredentials, WebError> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let (Some(value), None) = (values.next(), values.next()) else {
        return Err(WebError::MissingBasicCredentials);
    };
    if value.len() > BASIC_HEADER_MAX {
        return Err(WebError::MissingBasicCredentials);
    }
    let text = value
        .to_str()
        .map_err(|_not_visible_ascii| WebError::MissingBasicCredentials)?;
    let (scheme, token) = text
        .split_once(' ')
        .ok_or(WebError::MissingBasicCredentials)?;
    if !scheme.eq_ignore_ascii_case(SCHEME) {
        return Err(WebError::MissingBasicCredentials);
    }
    let decoded = STANDARD
        .decode(token)
        .map_err(|_bad_base64| WebError::MissingBasicCredentials)?;
    let pair = String::from_utf8(decoded).map_err(|_not_utf8| WebError::MissingBasicCredentials)?;
    let (typed_login, typed_password) = pair
        .split_once(':')
        .ok_or(WebError::MissingBasicCredentials)?;
    if typed_password.chars().any(char::is_control) {
        return Err(WebError::MissingBasicCredentials);
    }
    let login =
        TypedLogin::parse(typed_login).map_err(|_bad_login| WebError::MissingBasicCredentials)?;
    let password = Password::parse(typed_password)
        .map_err(|_bad_password| WebError::MissingBasicCredentials)?;
    Ok(BasicCredentials { login, password })
}

#[cfg(test)]
mod tests;
