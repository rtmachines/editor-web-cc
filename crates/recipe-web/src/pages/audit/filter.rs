//! The parsers of the filter values of the audit pages (ADR-0051 (d) and (h), design D6).
//!
//! The module solves one problem: `/audit` and `/security-events` read the same kinds of values:
//! a date, a UUID, an address, and a login. One parser for each kind gives one answer to a bad
//! value: `422 E-WEB-008` with the name of the field. The parser keeps no typed text in the error.
//! `recipe-web` has no dependency on `time` outside the tests, so `recipe-app` owns the days
//! (`UtcDay`, `day_range`). The reasons are the nearest ones of `FieldReason`. A reason of its own
//! for a date, a choice, and an identifier is a follow-up (see the report of task 11.3).

use std::net::IpAddr;

use recipe_app::audit::{UtcDay, day_range};
use recipe_app::ports::TimeRange;
use uuid::Uuid;

use crate::error::{FieldReason, WebError};
use crate::pages::recipes::fields::FieldLines;
use crate::pages::render::encode_query;

/// The length of a UUID in the hyphenated form.
const HYPHENATED_UUID_LENGTH: usize = 36;
/// The reason of a value that is not in a fixed list, not a UUID, or not a date.
pub(crate) const BAD_VALUE: FieldReason = FieldReason::Query;

/// The path of the audit list. The pagers, the filter form, and the entry links use it.
pub(crate) const AUDIT_PATH: &str = "/audit";

/// Read an IP address in the text form for the field `client_ip`.
///
/// # Errors
///
/// Returns `422 E-WEB-008` with the field `client_ip` for any other text.
pub(crate) fn address(text: &str) -> Result<IpAddr, WebError> {
    text.parse().map_err(|_reason| WebError::BadRequest {
        field: "client_ip",
        reason: FieldReason::Forwarded,
    })
}

/// Return `None` for an absent or empty field. An empty field means no filter.
pub(crate) fn present(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.is_empty())
}

/// Build the error of a value that is not in a fixed list.
pub(crate) const fn not_listed(field: &'static str) -> WebError {
    WebError::BadRequest {
        field,
        reason: BAD_VALUE,
    }
}

/// Read the field `text` with `parse`. An empty field gives `None`.
///
/// # Errors
///
/// Returns the error of `parse` for a field with a bad value.
pub(crate) fn optional<T>(
    text: Option<&str>,
    parse: impl FnOnce(&str) -> Result<T, WebError>,
) -> Result<Option<T>, WebError> {
    present(text).map(parse).transpose()
}

/// Read a UUID in the hyphenated form for the field `field`.
///
/// # Errors
///
/// Returns `422 E-WEB-008` with the field `field` for any other text.
pub(crate) fn uuid(field: &'static str, text: &str) -> Result<Uuid, WebError> {
    let bad = || WebError::BadRequest {
        field,
        reason: BAD_VALUE,
    };
    if text.len() != HYPHENATED_UUID_LENGTH {
        return Err(bad());
    }
    Uuid::parse_str(text).map_err(|_reason| bad())
}

/// Read the date field `field`. A bad value adds a line to `lines` and gives `None`.
///
/// `UtcDay::parse` returns a `DayReason`. The reason has no `code()`. This function wraps it into
/// a line of `E-WEB-008` (`WebError::BadRequest`).
fn day_field(lines: &mut FieldLines, field: &'static str, text: Option<&str>) -> Option<UtcDay> {
    let text = present(text)?;
    UtcDay::parse(text)
        .map_err(|_bad| lines.push(field, BAD_VALUE))
        .ok()
}

/// Read the fields `from` and `to` into the range of instants (ADR-0051 (d), design D6).
///
/// `recipe-app` owns the rules of the days and of the range: both days are inside, and the date
/// 9999-12-31 sets no upper bound. A bad value or a `to` before `from` adds a line to `lines`.
/// The `DayRangeReason` of a reversed range has no `code()`. This function wraps it into a line of
/// `E-WEB-008` (`WebError::BadRequest`).
pub(crate) fn time_range(
    lines: &mut FieldLines,
    from: Option<&str>,
    to: Option<&str>,
) -> TimeRange {
    let first = day_field(lines, "from", from);
    let last = day_field(lines, "to", to);
    day_range(first, last).unwrap_or_else(|_reversed| {
        lines.push("to", BAD_VALUE);
        TimeRange::default()
    })
}

/// Return the text of the filter `key` in `pairs`, or an empty text.
///
/// The pages of the audit list and of the security events use it to fill the filter form.
pub(crate) fn value_of(pairs: &[(&'static str, String)], key: &str) -> String {
    pairs
        .iter()
        .find(|(name, _value)| *name == key)
        .map(|(_name, value)| value.clone())
        .unwrap_or_default()
}

/// Build the address of a page of a cursor list: the path, the filters, and the cursor `before`.
///
/// `path` comes from code. An empty value is left out. The cursor comes last.
pub(crate) fn cursor_url(path: &str, pairs: &[(&str, &str)], before: Option<u64>) -> String {
    let mut parts: Vec<String> = pairs
        .iter()
        .filter(|(_key, value)| !value.is_empty())
        .map(|(key, value)| format!("{}={}", encode_query(key), encode_query(value)))
        .collect();
    if let Some(id) = before {
        parts.push(format!("before={id}"));
    }
    if parts.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{}", parts.join("&"))
    }
}

#[cfg(test)]
mod tests;
