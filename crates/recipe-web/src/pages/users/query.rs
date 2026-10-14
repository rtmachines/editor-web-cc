//! The query string of `GET /users` (`api.md` section 2.8, design D9).

use serde::Deserialize;

use crate::error::WebError;

/// The most characters of the search text.
const Q_MAX_CHARS: usize = 64;
/// The reason of the field error for a bad search text.
const Q_REASON: &str =
    "the search text has at most 64 characters from a to z, 0 to 9, period, underscore, and hyphen";
/// The reason of the field error for a bad page number.
const PAGE_REASON: &str = "the page is a whole number from 1";

/// The raw query string. A key that is not listed here gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListQuery {
    q: Option<String>,
    page: Option<String>,
}

/// The checked query: the search text in lower case and the page counted from 1.
pub(super) struct ListRequest {
    /// The text that a login must contain, or `None` for every user.
    pub(super) q: Option<String>,
    /// The page number of the URL. The first page is 1.
    pub(super) page: u32,
    /// The page number of the use case. The first page is 0.
    pub(super) index: u32,
}

/// Tell whether a character belongs to the alphabet of a login after folding to lower case.
fn is_login_character(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')
}

impl ListQuery {
    /// Check `q` and `page`.
    ///
    /// # Errors
    ///
    /// Returns `WebError::BadRequest` (`E-WEB-008`) for a search text with more than 64
    /// characters or a character outside the alphabet of a login, and for a page that is not a
    /// whole number from 1. The error holds a fixed text, never the typed value.
    pub(super) fn check(self) -> Result<ListRequest, WebError> {
        let q = match self.q.as_deref() {
            None | Some("") => None,
            Some(text) => {
                let folded = text.to_ascii_lowercase();
                let valid =
                    folded.chars().count() <= Q_MAX_CHARS && folded.chars().all(is_login_character);
                if !valid {
                    return Err(WebError::BadRequest {
                        field: "q",
                        reason: Q_REASON,
                    });
                }
                Some(folded)
            }
        };
        let bad_page = WebError::BadRequest {
            field: "page",
            reason: PAGE_REASON,
        };
        let page = match self.page.as_deref() {
            None => 1,
            Some(text) => text.parse::<u32>().map_err(|_not_a_number| bad_page)?,
        };
        // The URL counts from 1 and the use case from 0. Page 0 has no predecessor.
        let index = page.checked_sub(1).ok_or(WebError::BadRequest {
            field: "page",
            reason: PAGE_REASON,
        })?;
        Ok(ListRequest { q, page, index })
    }
}
