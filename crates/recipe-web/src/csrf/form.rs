//! Read one field of an `application/x-www-form-urlencoded` body.

use crate::hex::hex_value;

/// The content type of an HTML form.
const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";

/// Tell whether a `Content-Type` value names an urlencoded form.
pub(super) fn is_form_content_type(value: &str) -> bool {
    value
        .trim_start()
        .to_ascii_lowercase()
        .starts_with(FORM_CONTENT_TYPE)
}

/// Decode `%XX` and `+` in one name or value.
///
/// A `%` that no two hex digits follow stays as it is.
fn percent_decode(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut bytes = raw.iter().copied();
    while let Some(byte) = bytes.next() {
        match byte {
            b'+' => out.push(b' '),
            b'%' => {
                let mut ahead = bytes.clone();
                let decoded = ahead
                    .next()
                    .and_then(hex_value)
                    .zip(ahead.next().and_then(hex_value))
                    .and_then(|(high, low)| high.checked_mul(16)?.checked_add(low));
                if let Some(decoded) = decoded {
                    out.push(decoded);
                    bytes = ahead;
                } else {
                    out.push(byte);
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Return the value of the first field called `name`, or `None` when the body has no such field.
///
/// The value must be UTF-8. The function reads no more than the body that the caller buffered.
pub(super) fn form_field(body: &[u8], name: &str) -> Option<String> {
    let (_name, raw_value) = body
        .split(|byte| *byte == b'&')
        .map(split_pair)
        .find(|(raw_name, _value)| percent_decode(raw_name) == name.as_bytes())?;
    String::from_utf8(percent_decode(raw_value)).ok()
}

/// Split `name=value` at the first `=`. A pair without `=` has an empty value.
fn split_pair(pair: &[u8]) -> (&[u8], &[u8]) {
    match pair.iter().position(|byte| *byte == b'=') {
        Some(at) => (
            pair.get(..at).unwrap_or_default(),
            pair.get(at.saturating_add(1)..).unwrap_or_default(),
        ),
        None => (pair, &[]),
    }
}
