//! The query string of `GET /users` (`api.md` section 2.8, design D9).

use serde::Deserialize;

use crate::error::{FieldReason, WebError};
use crate::pages::path_values::{self, Page};

/// The most characters of the search text.
const Q_MAX_CHARS: usize = 64;

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
    /// The page of the list.
    pub(super) page: Page,
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
    /// characters or a character outside the alphabet of a login.
    /// It also returns that error for a page that is not a
    /// canonical whole number from 1 (`path_values::page`). The error holds a fixed text, never the typed value.
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
                        reason: FieldReason::Q,
                    });
                }
                Some(folded)
            }
        };
        let page = path_values::page(self.page.as_deref())?;
        Ok(ListRequest { q, page })
    }
}
