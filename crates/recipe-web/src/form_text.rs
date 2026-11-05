//! Text conversion for the multi-line fields of a form (ADR-0112 (g)).

use std::borrow::Cow;

/// Convert each CRLF of a multi-line field to LF and return the result.
///
/// A browser sends CRLF for each line end of a `textarea`. The domain rule accepts LF and CRLF,
/// but a stored text keeps one form only (ADR-0112 (g)). The function solves the problem that the
/// server counts one character for each line end. A lone LF stays. A text with a lone CR, even
/// next to a CRLF, comes back unchanged. The parse of the value object then refuses it as a
/// control character (ADR-0028). A text without CRLF comes back borrowed and unchanged.
pub(crate) fn crlf_to_lf(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") && every_cr_starts_a_crlf(text) {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

/// Return `true` when an LF directly follows each CR of the text.
fn every_cr_starts_a_crlf(text: &str) -> bool {
    text.split('\r')
        .skip(1)
        .all(|after_cr| after_cr.starts_with('\n'))
}

#[cfg(test)]
mod tests;
