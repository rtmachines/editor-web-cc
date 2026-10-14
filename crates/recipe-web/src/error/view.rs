//! The templates of an error answer (`api.md` section 5.2, design D13).

use askama::Template;

use crate::i18n::Lang;

/// One line of `field-errors.html`.
pub(crate) struct FieldLine {
    /// The form field, or an empty text for an error of the whole form.
    pub(crate) field: String,
    /// The sentence in the UI language.
    pub(crate) message: String,
}

/// What every error fragment shows: the code, one sentence, and the request id.
pub(crate) struct ErrorView {
    /// The code `E-<CTX>-<nnn>`.
    pub(crate) code: &'static str,
    /// One sentence in the UI language.
    pub(crate) message: String,
    /// The sentence "Reference: {request_id}." in the UI language.
    pub(crate) reference: String,
    /// The lines of `field-errors.html`.
    pub(crate) lines: Vec<FieldLine>,
}

/// The fragment `error.html`.
#[derive(Template)]
#[template(path = "error.html")]
pub(crate) struct ErrorFragment<'a> {
    pub(crate) view: &'a ErrorView,
}

/// The fragment `field-errors.html`.
#[derive(Template)]
#[template(path = "field-errors.html")]
pub(crate) struct FieldErrorsFragment<'a> {
    pub(crate) view: &'a ErrorView,
}

/// The full error page for a request without `HX-Request`.
///
/// `csrf_token` is the token of the session for `hx-headers`. It is empty without a session.
#[derive(Template)]
#[template(path = "error-page.html")]
pub(crate) struct ErrorPage<'a> {
    pub(crate) lang: Lang,
    pub(crate) csrf_token: &'a str,
    pub(crate) title: &'a str,
    pub(crate) field_errors: bool,
    pub(crate) view: &'a ErrorView,
}
