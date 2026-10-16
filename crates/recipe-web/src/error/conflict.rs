//! What the conflict answer knows about the version (design D10, `api.md` section 4).

use recipe_app::error::AppError;
use recipe_domain::identity::Login;

/// The result of the read of the version after a conflict (`WebError::Conflict`).
///
/// The type solves one problem: the answer `409 E-WEB-001` shows the current `row_version`, the
/// login, and the time of the last change. The read of these values can fail. The answer is then
/// still `409 E-WEB-001`, and the mapping of `WebError` logs the failure once.
#[derive(Debug)]
pub enum ConflictView {
    /// `GetVersion` read the version.
    Read {
        /// The `row_version` that the server holds.
        row_version: i64,
        /// The login of the last editor and the time of the change, or `None` when the version has
        /// no audit entry of its last change.
        last_change: Option<Box<(Login, String)>>,
    },
    /// `GetVersion` failed with this error.
    Unread(Box<AppError>),
}

impl ConflictView {
    /// Return the failure of the read, if the read failed.
    pub(crate) fn failure(&self) -> Option<&AppError> {
        match self {
            Self::Read { .. } => None,
            Self::Unread(failure) => Some(failure),
        }
    }
}
