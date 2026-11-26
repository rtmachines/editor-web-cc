//! Helpers of the value leg tests: the form body, its hash, and the field lines of a page.

use crate::pages::parameters::test_kit::form;
use crate::test_support::heap::BodyDigest;

/// Return the body of the form with `pairs`.
pub(super) fn body_of(pairs: &[(String, String)]) -> String {
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    form(&refs)
}

/// Return the length and the SHA-256 in lowercase hexadecimal of `body`.
pub(super) fn digest_of(body: &str) -> (usize, String) {
    let mut digest = BodyDigest::new("");
    digest.push(body.as_bytes());
    digest.finish()
}

/// Return the field and the code of each field line of `page`, in the order of the page.
///
/// A line matches the field pattern and the code pattern of the section "Interfaces" of the
/// design `value-body-memory-load`. The test code has no regex, so it matches by hand.
pub(super) fn field_lines(page: &str) -> Vec<(String, String)> {
    const DIV: &str = "<div class=\"field-error\" id=\"";
    const SPAN: &str = "<span class=\"field-error-code\">";
    let mut lines = Vec::new();
    for chunk in page.split(DIV).skip(1) {
        let id: String = chunk
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.[]-".contains(*c))
            .collect();
        let Some(field) = id.strip_suffix("-error").filter(|field| !field.is_empty()) else {
            continue;
        };
        if !chunk[id.len()..].starts_with("\">") {
            continue;
        }
        let Some((_before, after)) = chunk.split_once(SPAN) else {
            continue;
        };
        let code: String = after.chars().take_while(|c| *c != '<').collect();
        let bytes = code.as_bytes();
        let is_code = bytes.len() == 9
            && code.starts_with("E-")
            && bytes[2..5].iter().all(u8::is_ascii_uppercase)
            && bytes[5] == b'-'
            && bytes[6..].iter().all(u8::is_ascii_digit)
            && after[code.len()..].starts_with("</span>");
        if is_code {
            lines.push((field.to_owned(), code));
        }
    }
    lines
}
