//! The lines of a list of errors (design D9, ADR-0041).
//!
//! One function, [`line_view`], turns a line into its text. The summary in `#error-panel` and the
//! line under the input both use it, so the two places cannot disagree.

use fluent::FluentArgs;
use recipe_app::ports::NonEmpty;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{LeafPath, PlantError};
use recipe_domain::recipe::{
    DraftError, ParameterTag, RecipeError, RecipeErrors, SnapshotError, SnapshotItem,
};

use super::leaf_of;
use super::plant::plant_line_args;
use super::recipe::recipe_line_args;
use super::values::text_or;
use super::view::{field_key, has_input, input_id};
use super::{FieldReason, LineView, ListView, RowField, WebError};
use crate::i18n::{Bundles, Lang};

/// The number of lines that the summary in `#error-panel` shows (ADR-0041 (d)).
///
/// The blocker notice of the approval preview shows the same number of lines (ADR-0049 (c)).
pub(crate) const SHOWN_LINES: usize = 20;

/// One argument of the message of a line.
///
/// The type solves one problem: a handler builds a line before the answer knows the UI language.
/// A word of the UI therefore stays a key until the answer renders the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineArg {
    /// Data that the message shows as it is, such as a tag, a path, or a number.
    Data(String),
    /// A word of the UI. It is the text of a key of the bundles.
    Word {
        /// The key of the word in the bundles.
        key: String,
        /// The arguments of the word, each one data.
        args: Vec<(&'static str, String)>,
        /// The English text of the word. The line shows it when the bundle has no such key.
        fallback: String,
    },
}

impl LineArg {
    /// Build a word without arguments.
    pub(crate) fn word(key: impl Into<String>, fallback: impl ToString) -> Self {
        Self::Word {
            key: key.into(),
            args: Vec::new(),
            fallback: fallback.to_string(),
        }
    }

    /// Return the text of the argument in `lang`.
    pub(crate) fn text(&self, bundles: &Bundles, lang: Lang) -> String {
        match self {
            Self::Data(text) => text.clone(),
            Self::Word {
                key,
                args,
                fallback,
            } => {
                let mut values = FluentArgs::new();
                for (name, value) in args {
                    values.set(*name, value.clone());
                }
                text_or(bundles, lang, key, Some(&values), fallback.clone())
            }
        }
    }
}

/// One line of a list of errors (`WebError::Fields`, design D9).
///
/// The type solves one problem: one form answer lists every bad field and every fault of a value
/// at once. Each line holds its code, the arguments of its message, and the `id` of its input.
/// The message is the message of the code, `error-<code>`. No field holds a secret. The data is
/// boxed, so `WebError` stays small (`clippy::result_large_err`).
#[derive(Debug)]
pub struct FieldError(Box<Line>);

/// The data of one line.
#[derive(Debug)]
struct Line {
    /// The code of the line.
    code: ErrorCode,
    /// The arguments of the message `error-<code>`.
    args: Vec<(&'static str, LineArg)>,
    /// The data that the line is about, when the message does not name it.
    subject: Option<String>,
    /// The `id` of the input of the line.
    target: Option<String>,
    /// The English text of the line, for the log and for a message that cannot format.
    fallback: String,
}

impl From<Line> for FieldError {
    fn from(line: Line) -> Self {
        Self(Box::new(line))
    }
}

impl FieldError {
    /// Return the code of the line.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        self.0.code
    }

    /// Return the `id` of the input of the line, or `None` when no input exists.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        self.0.target.as_deref()
    }
}

/// The lines of the form fields. The form handlers of wave 4 and later build them.
impl FieldError {
    /// Build the line `E-WEB-008` of a request field, as `WebError::BadRequest` names it.
    ///
    /// The label is the text of `field-<name>`. The line links to the input of the field when the
    /// field has one.
    pub(crate) fn bad_request(field: &'static str, reason: FieldReason) -> Self {
        let label = LineArg::word(field_key(field), field);
        let target = has_input(field).then(|| input_id(field).to_owned());
        let fallback = WebError::BadRequest { field, reason }.to_string();
        Self::field_line(label, reason, target, fallback)
    }

    /// Build the line `E-WEB-008` of one input of a member row, as `WebError::BadRow` names it.
    pub(crate) fn bad_row(row: u16, field: RowField, reason: FieldReason) -> Self {
        let number = u32::from(row).saturating_add(1).to_string();
        let label = LineArg::Word {
            key: field.label_key().to_owned(),
            args: vec![("number", number)],
            fallback: format!("{field} of member row {row}"),
        };
        let target = Some(format!("members-{row}-{}", field.id_suffix()));
        let fallback = WebError::BadRow { row, field, reason }.to_string();
        Self::field_line(label, reason, target, fallback)
    }

    /// Build the line `E-WEB-008` of an input with a label that takes arguments.
    ///
    /// An example is the value input of a leaf: the label `field-value-leaf` takes the leaf path,
    /// and the input `id` is `value-<n>` (design D6).
    pub(crate) fn bad_input(
        label_key: &'static str,
        label_args: Vec<(&'static str, String)>,
        reason: FieldReason,
        input: String,
    ) -> Self {
        let fallback = format!("Field {input} is not valid: {reason}.");
        let label = LineArg::Word {
            key: label_key.to_owned(),
            args: label_args,
            fallback: input.clone(),
        };
        Self::field_line(label, reason, Some(input), fallback)
    }

    /// Build a line `E-WEB-008` from its label, its reason, and its input.
    fn field_line(
        label: LineArg,
        reason: FieldReason,
        target: Option<String>,
        fallback: String,
    ) -> Self {
        Self::from(Line {
            code: ErrorCode::WEB_008,
            args: vec![
                ("field", label),
                ("reason", LineArg::word(reason.key(), reason)),
            ],
            subject: None,
            target,
            fallback,
        })
    }
}

/// The lines of the domain errors.
impl FieldError {
    /// Build the line of one error of the Recipe Management context.
    ///
    /// The caller names the input of the line, for example `s7_address` for `E-RCP-022`, or
    /// `None` for an error without an input.
    pub(crate) fn recipe(error: &RecipeError, target: Option<String>) -> Self {
        Self::from(Line {
            code: error.code(),
            args: recipe_line_args(error),
            subject: None,
            target,
            fallback: error.to_string(),
        })
    }

    /// Return the line with `subject`, for example the label of the input that the line names.
    ///
    /// The summary shows the subject next to the sentence, so a reader sees the input of a domain
    /// error whose message names no input (ADR-0041 (f)).
    pub(crate) fn with_subject(self, subject: String) -> Self {
        let mut line = self.0;
        line.subject = Some(subject);
        Self(line)
    }

    /// Build the line of one error of the Plant Model with a subject and an input.
    fn plant_line(error: &PlantError, subject: Option<String>, target: Option<String>) -> Self {
        Self::from(Line {
            code: error.code(),
            args: plant_line_args(error),
            subject,
            target,
            fallback: error.to_string(),
        })
    }

    /// Build the line of the error of a snapshot item.
    ///
    /// A message of the Plant Model does not name the parameter, so the line shows the tag as its
    /// subject (ADR-0041 (f)).
    fn snapshot(error: &SnapshotError, tag: Option<&ParameterTag>, target: Option<String>) -> Self {
        match error {
            SnapshotError::Recipe(inner) => Self::recipe(inner, target),
            SnapshotError::Plant(inner) => {
                Self::plant_line(inner, tag.map(ToString::to_string), target)
            }
        }
    }
}

/// The lines of one plant error and of the error of a draft command.
impl FieldError {
    /// Build the line of one error of the Plant Model, for example `E-PLT-020` under `unit_path`.
    pub(crate) fn plant(error: &PlantError, target: Option<String>) -> Self {
        Self::plant_line(error, None, target)
    }
}

/// One fault of a draft command: an error of Recipe Management or of the Plant Model.
///
/// The type solves one problem: the caller of [`draft_lines`] names the input of each fault, and
/// a `DraftError` holds faults of both contexts.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the form handlers of wave 4 and later call it")
)]
pub(crate) enum DraftFault<'a> {
    /// An error of the Recipe Management context.
    Recipe(&'a RecipeError),
    /// An error of the Plant Model context.
    Plant(&'a PlantError),
}

/// Return one line for each fault of the error of a draft command, in the order of the error.
///
/// `target` gives the input `id` of each fault. A fault of a leaf can take the input of its leaf
/// (use `leaf_of`), and a fault of a field can take the input of that field (design D9).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the form handlers of wave 4 and later call it")
)]
pub(crate) fn draft_lines(
    error: &DraftError,
    target: &dyn Fn(DraftFault<'_>) -> Option<String>,
) -> Vec<FieldError> {
    let recipe = |error: &RecipeError| FieldError::recipe(error, target(DraftFault::Recipe(error)));
    match error {
        DraftError::Recipe(inner) => vec![recipe(inner)],
        DraftError::List(errors) => errors.iter().map(recipe).collect(),
        DraftError::Plant(inner) => {
            vec![FieldError::plant(inner, target(DraftFault::Plant(inner)))]
        }
    }
}

/// Return one line for each entry of a list of recipe errors, in the order of the list.
///
/// A line of an entry with a leaf takes its input `id` from `input_id`. The caller builds that
/// function from the form, so this module does not depend on the leaf grid (design D9).
pub fn leaf_lines(
    errors: &RecipeErrors,
    input_id: &dyn Fn(&LeafPath) -> Option<String>,
) -> Vec<FieldError> {
    errors
        .iter()
        .map(|error| FieldError::recipe(error, leaf_of(error).and_then(input_id)))
        .collect()
}

/// Return one line for each item of a failed snapshot check (`E-RCP-020`), in their order.
///
/// A line of a parameter takes the `id` of its row from `row_id`. A line of the version has none.
pub fn item_lines(
    items: &[SnapshotItem],
    row_id: &dyn Fn(&ParameterTag) -> Option<String>,
) -> Vec<FieldError> {
    items
        .iter()
        .map(|item| match item {
            SnapshotItem::Parameter { tag, error } => {
                FieldError::snapshot(error, Some(tag), row_id(tag))
            }
            SnapshotItem::Version { error } => FieldError::snapshot(error, None, None),
            SnapshotItem::UnknownUnit { tag, unit } => FieldError::from(Line {
                code: item.code(),
                args: vec![("path", LineArg::Data(unit.to_string()))],
                subject: Some(tag.to_string()),
                target: row_id(tag),
                fallback: item.to_string(),
            }),
        })
        .collect()
}

/// Return the view of a line in `lang`: its code, its subject, its sentence, and its input.
///
/// The sentence is the message `error-<code>`. When the bundle cannot format it, the line shows
/// the English text of its error, so the sentence never has a hole.
pub(crate) fn line_view(line: &FieldError, bundles: &Bundles, lang: Lang) -> LineView {
    let FieldError(line) = line;
    let mut args = FluentArgs::new();
    for (name, arg) in &line.args {
        args.set(*name, arg.text(bundles, lang));
    }
    let message = text_or(
        bundles,
        lang,
        &format!("error-{}", line.code),
        Some(&args),
        line.fallback.clone(),
    );
    LineView {
        code: line.code.as_str(),
        subject: line.subject.clone().unwrap_or_default(),
        message,
        target: line.target.clone().unwrap_or_default(),
    }
}

/// Return the summary of a list of lines in `lang` (ADR-0041 (d)).
///
/// The summary counts every line and shows the first 20. The sentence `errors-more` counts the
/// other lines.
pub(crate) fn list_view<'a>(
    lines: impl IntoIterator<Item = &'a FieldError>,
    bundles: &Bundles,
    lang: Lang,
) -> ListView {
    let mut views = Vec::new();
    let mut total: usize = 0;
    for line in lines {
        total = total.saturating_add(1);
        if views.len() < SHOWN_LINES {
            views.push(line_view(line, bundles, lang));
        }
    }
    ListView {
        count: count_text(
            bundles,
            lang,
            "errors-count",
            total,
            format!("Errors: {total}."),
        ),
        lines: views,
        more: more_text(total, bundles, lang),
    }
}

/// Return the text of the message `key` with the argument `count`.
fn count_text(bundles: &Bundles, lang: Lang, key: &str, count: usize, fallback: String) -> String {
    let mut args = FluentArgs::new();
    args.set("count", count.to_string());
    text_or(bundles, lang, key, Some(&args), fallback)
}

/// Return the sentence `errors-more` for a list of `total` lines in `lang` (ADR-0041 (d)).
///
/// The sentence counts the lines after the first `SHOWN_LINES`. It is empty when no line is left.
pub(crate) fn more_text(total: usize, bundles: &Bundles, lang: Lang) -> String {
    match total.checked_sub(SHOWN_LINES) {
        Some(other) if other > 0 => count_text(
            bundles,
            lang,
            "errors-more",
            other,
            format!("Other errors: {other}."),
        ),
        _none => String::new(),
    }
}

/// Return the English log text of the lines of a form: the count and the first 20 lines.
pub(super) fn lines_text(lines: &NonEmpty<FieldError>) -> String {
    let first = lines
        .iter()
        .take(SHOWN_LINES)
        .map(|line| line.0.fallback.clone());
    capped_text(lines.len(), first)
}

/// Return the English log text of a list of recipe errors: the count and the first 20 entries.
pub(super) fn errors_text(errors: &RecipeErrors) -> String {
    let first = errors.iter().take(SHOWN_LINES).map(ToString::to_string);
    capped_text(errors.len(), first)
}

/// Return the log text of a list: one line with the count, then the first lines.
///
/// A list can hold 16 384 lines. The log line of a request stays short, because the caller passes
/// only the first `SHOWN_LINES` texts (`reliability.md` section 3.1).
fn capped_text(total: usize, first: impl Iterator<Item = String>) -> String {
    let lines: Vec<String> = first.take(SHOWN_LINES).collect();
    let mut text = format!("{total} errors. The first {} follow.", lines.len());
    for line in lines {
        text.push('\n');
        text.push_str(&line);
    }
    text
}
