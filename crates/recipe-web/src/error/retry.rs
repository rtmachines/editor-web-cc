//! The form that a failed request shows again.

use std::sync::Arc;

use crate::csrf::PreSession;

/// A form that the answer of an error shows again, with the error on top.
///
/// The type solves one problem: a failed login must show the login form again with a fresh
/// pre-session token, but only the layer `render_errors` knows the language, the request id, and
/// the HTMX header, and only that layer writes the answer. The handler stores this value in the
/// response, next to the error, and the layer reads both.
#[derive(Clone)]
pub(crate) enum Retry {
    /// The login form with a new pre-session token.
    Login(Arc<PreSession>),
}
