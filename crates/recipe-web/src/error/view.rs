//! The templates of an error answer (`api.md` section 5.2, design D13).

use askama::Template;

use crate::layout::Layout;

/// One field line of an error fragment.
///
/// The name comes from code (`WebError::BadRequest`), never from the request. An error of the
/// whole form has no line.
pub(crate) struct FieldLine {
    /// The name of the form field, which is also the `id` of its input.
    pub(crate) field: String,
    /// The label of the field in the UI language.
    pub(crate) label: String,
    /// True when an input with this `id` exists on a page (design D6).
    pub(crate) has_input: bool,
}

/// The fields that have an input with the same `id` on a page.
///
/// A field of this list gets a link in an error fragment. Any other field gets its label only.
const INPUT_FIELDS: [&str; 5] = ["login", "password", "current_password", "role", "q"];

/// Return true when the form field `name` has an input on a page.
pub(crate) fn has_input(name: &str) -> bool {
    INPUT_FIELDS.contains(&name)
}

/// Return the key of the label of the form field `name` in the bundles.
///
/// The key is `field-` and the name with each `_` replaced by `-`.
pub(crate) fn field_key(name: &str) -> String {
    format!("field-{}", name.replace('_', "-"))
}

/// What every error fragment shows: the code, one sentence, and the request id.
pub(crate) struct ErrorView {
    /// The code `E-<CTX>-<nnn>`.
    pub(crate) code: &'static str,
    /// One sentence in the UI language.
    pub(crate) message: String,
    /// The sentence "Reference: {request_id}." in the UI language.
    pub(crate) reference: String,
    /// The field lines of `field-errors.html`. Empty for an error of the whole form.
    pub(crate) lines: Vec<FieldLine>,
    /// True when each line links to its input: an HTMX fragment and the login page.
    pub(crate) link_fields: bool,
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
/// The error layer runs before the session layer, so the layout is the bare one: no navigation,
/// no language switch, and no `hx-headers`.
#[derive(Template)]
#[template(path = "error-page.html")]
pub(crate) struct ErrorPage<'a> {
    pub(crate) layout: Layout<'a>,
    pub(crate) field_errors: bool,
    pub(crate) view: &'a ErrorView,
}
