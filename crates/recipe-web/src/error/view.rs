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
/// The users page keeps `q`. The plant pages add the names of design D5. Two forms of one page can
/// use one name, so the `id` of an input is the name of its form, for example `machine_name`.
/// The recipe pages add the field ids of design D6.
const INPUT_FIELDS: [&str; 31] = [
    "login",
    "password",
    "current_password",
    "role",
    "q",
    "code",
    "machine_name",
    "plc_platform",
    "main_unit_name",
    "parent_path",
    "segment",
    "unit_name",
    "type_name",
    "machine_q",
    "show_archived",
    "recipe_q",
    "recipe_machine",
    "recipe_status",
    "show_deleted",
    "machine_code",
    "reference_number",
    "recipe_name",
    "change_note",
    "based_on",
    "unit_path",
    "tag",
    "name_en",
    "description_en",
    "type",
    "s7_address",
    "ordinal",
];

/// The name of the list of member rows. The list is a `fieldset`, so it has no input of its own.
const ROW_LIST: &str = "members";

/// The `id` of the first input of the list of member rows.
const FIRST_ROW_INPUT: &str = "members-0-name";

/// Return true when the form field `name` has an input on a page.
///
/// The list of member rows counts too. Its error line links to the first input of the list.
pub(crate) fn has_input(name: &str) -> bool {
    name == ROW_LIST || INPUT_FIELDS.contains(&name)
}

/// Return the `id` of the input that an error line of the form field `name` links to.
///
/// A field has an input with its own name, except the list of member rows. An error of the whole
/// list links to the first input of the list (`design-system.md` 10.5).
pub(crate) fn input_id(name: &str) -> &str {
    if name == ROW_LIST {
        FIRST_ROW_INPUT
    } else {
        name
    }
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
    /// The summary of a list of errors (ADR-0041 (d)). It replaces the sentence and the lines.
    pub(crate) list: Option<ListView>,
    /// The lines of `conflict.html` (design D10). Only `E-WEB-001` of a draft has them.
    pub(crate) conflict: Option<ConflictLines>,
}

/// The lines of `conflict.html` in the UI language (design D10).
pub(crate) struct ConflictLines {
    /// The sentence `conflict-row-version` with the `row_version` of the server.
    pub(crate) row_version: String,
    /// The sentence `conflict-last-change`, or `conflict-no-editor` without a last change.
    pub(crate) last_change: String,
    /// The sentence `conflict-reload-warning`.
    pub(crate) warning: String,
    /// The label `conflict-reload` of the link.
    pub(crate) reload: String,
    /// The path of the version page, for example `/recipes/R-100/versions/3`.
    pub(crate) href: String,
}

/// One line of a list of errors, in the UI language (design D9).
///
/// The summary in `#error-panel` and the line under the input show the same view, so the two
/// places cannot disagree (ADR-0041).
pub(crate) struct LineView {
    /// The code `E-<CTX>-<nnn>` of the line.
    pub(crate) code: &'static str,
    /// The data that the line is about, such as a tag. Empty when the message names it.
    pub(crate) subject: String,
    /// One sentence in the UI language.
    pub(crate) message: String,
    /// The `id` of the input of the line. Empty when no input exists.
    pub(crate) target: String,
}

/// The summary of a list of errors in `#error-panel` (ADR-0041 (d)).
pub(crate) struct ListView {
    /// The sentence `errors-count` with the count of every line.
    pub(crate) count: String,
    /// The first lines of the list, at most 20.
    pub(crate) lines: Vec<LineView>,
    /// The sentence `errors-more` with the count of the other lines. Empty when no line is left.
    pub(crate) more: String,
}

/// The lines of an input without an error. A field macro takes it as the default.
pub(crate) const NO_LINES: &[LineView] = &[];

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
