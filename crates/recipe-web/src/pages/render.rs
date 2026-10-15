//! Helpers that every page handler shares.

use askama::Template;
use askama::filters::urlencode_strict;
use axum::http::header::{LOCATION, SET_COOKIE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use recipe_domain::audit::RequestId;
use recipe_domain::timestamp::Timestamp;

use crate::error::WebError;

/// Render a template. A failure is a defect of the server and gives `E-WEB-010`.
///
/// # Errors
///
/// Returns `WebError::Internal` when the template cannot render.
pub(crate) fn render<T: Template>(
    page: &T,
    request_id: RequestId,
) -> Result<Html<String>, WebError> {
    page.render()
        .map(Html)
        .map_err(|_failed| WebError::Internal { request_id })
}

/// Write an instant as `yyyy-mm-dd hh:mm UTC` (`design-system.md` section 5).
///
/// The function solves one problem: the user pages and the machine page must write a time in one
/// form, so one function owns that form.
pub(crate) fn instant(stamp: &Timestamp) -> String {
    let moment = stamp.as_offset_date_time().to_utc();
    let date = moment.date();
    let time = moment.time();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02} UTC",
        date.year(),
        u8::from(date.month()),
        date.day(),
        time.hour(),
        time.minute()
    )
}

/// Build a `303` answer with the `Location` header and the given `Set-Cookie` values.
pub(crate) fn see_other(
    location: &'static str,
    cookies: impl IntoIterator<Item = HeaderValue>,
) -> Response {
    redirect(HeaderValue::from_static(location), cookies)
}

/// Build a `303` answer to a path that the server made at run time, for example `/machines/M7`.
///
/// The function solves one problem: `see_other` takes a `&'static str`, so it cannot build the
/// address of a machine or a unit. This function checks the text instead. The location must be a
/// path of this site: one `/` first, no `\`, and ASCII only. It must also be a valid header value.
/// A location from a value object always passes. The check stops a later caller that passes text
/// of a user from sending the browser to another site.
///
/// # Errors
///
/// Returns `WebError::Internal` (`E-WEB-010`) with `request_id` when the location fails the check.
pub(crate) fn see_other_path(
    location: &str,
    request_id: RequestId,
    cookies: impl IntoIterator<Item = HeaderValue>,
) -> Result<Response, WebError> {
    let defect = || WebError::Internal { request_id };
    if !is_site_path(location) {
        return Err(defect());
    }
    let value = HeaderValue::from_str(location).map_err(|_invalid| defect())?;
    Ok(redirect(value, cookies))
}

/// Percent-encode a key or a value of a query string. Only `A-Z a-z 0-9 - . _ ~` stay as they are.
fn encode_query(text: &str) -> String {
    let Ok(encoded) = urlencode_strict(text);
    encoded.to_string()
}

/// Build the address of one page of a list, for example `/machines?q=a%26b&page=2`.
///
/// The function solves one problem: the search text of a list goes into the link to the next page.
/// A raw `&`, `#`, `+`, or `%` would change the meaning of the link. Each key and each value is
/// percent-encoded with the strict set of askama, so the link keeps the text whole. The `page`
/// number comes last. `path` comes from code, never from the request. The pager macro writes the
/// address into `href`, where askama escapes it for HTML.
pub(crate) fn page_url(path: &str, query: &[(&str, &str)], page: u32) -> String {
    let pairs: Vec<String> = query
        .iter()
        .map(|(key, value)| format!("{}={}", encode_query(key), encode_query(value)))
        .chain(std::iter::once(format!("page={page}")))
        .collect();
    format!("{path}?{}", pairs.join("&"))
}

/// Return true when `location` is a path of this site: one `/` first, no `\`, and ASCII only.
fn is_site_path(location: &str) -> bool {
    location.is_ascii()
        && location.starts_with('/')
        && !location.starts_with("//")
        && !location.contains('\\')
}

/// Build the `303` answer for a location that is a valid header value.
fn redirect(location: HeaderValue, cookies: impl IntoIterator<Item = HeaderValue>) -> Response {
    let mut response = StatusCode::SEE_OTHER.into_response();
    let headers = response.headers_mut();
    headers.insert(LOCATION, location);
    for cookie in cookies {
        headers.append(SET_COOKIE, cookie);
    }
    response
}

#[cfg(test)]
mod tests;
