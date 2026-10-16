//! The checks of a save of a parameter form before its command (ADR-0039 (h) and (i),
//! ADR-0041 (a), design D6).
//!
//! The module solves one problem: the form of a new parameter and the edit form run the same
//! steps before their command. `fields=1`, `limit_mode`, and a changed type show the form again and
//! write nothing. Bad fields give every line at once. Only a clean save reaches the command.

use axum::response::Response;
use recipe_app::ports::NonEmpty;
use recipe_app::recipe::ParameterForm;
use recipe_domain::plant::TypeExpr;
use recipe_domain::recipe::{LeafTexts, LimitList};

use super::definition::{Checked, Definition, check};
use super::fields::SentFields;
use super::form_view::Shown;
use super::reply::{Ctx, load_shown, refuse, sent_shown, show_form};
use crate::error::{FieldError, WebError};

/// What a save sends to its command after every check of the web layer.
pub(crate) struct Ready {
    /// The data of the form with the preview tree of the sent type.
    pub(crate) form: ParameterForm,
    /// The definition fields.
    pub(crate) definition: Definition,
    /// What the form shows with the sent texts. An error answer of the command reuses it.
    pub(crate) shown: Shown,
    /// The texts of the leaves of the value.
    pub(crate) value: LeafTexts,
    /// The limit rows.
    pub(crate) limits: LimitList,
}

/// The result of the checks: the input of the command, or the answer of a request without a write.
pub(crate) enum Prepared {
    /// The save is clean. The command runs next.
    Ready(Box<Ready>),
    /// The form again: `fields=1`, `limit_mode`, or a changed type (ADR-0039 (h), (i), (k)).
    Answer(Response),
}

/// Show the form again with the grid of `wanted` and the notice `notice` (a key, or empty).
async fn again(
    ctx: Ctx<'_>,
    sent: &SentFields,
    wanted: Option<TypeExpr>,
    notice: &str,
) -> Result<Response, WebError> {
    let loaded = load_shown(ctx, wanted, sent.value_type().cloned()).await?;
    let mut shown = sent_shown(ctx, &loaded.form, sent, loaded.previewed.as_ref());
    if let Some(line) = loaded.line {
        return Err(refuse(ctx, &loaded.form, shown, NonEmpty::new(line)));
    }
    if !notice.is_empty() {
        shown.notice = ctx.texts().get(notice);
    }
    show_form(ctx, &loaded.form, shown)
}

/// Build the error with `lines` and the form with the grid of the sent `value_type`.
async fn refuse_lines(ctx: Ctx<'_>, sent: &SentFields, lines: Vec<FieldError>) -> WebError {
    let loaded = match load_shown(ctx, sent.value_type().cloned(), None).await {
        Ok(loaded) => loaded,
        Err(error) => return error,
    };
    let shown = sent_shown(ctx, &loaded.form, sent, loaded.previewed.as_ref());
    let mut lines = lines;
    lines.extend(loaded.line);
    match NonEmpty::try_from(lines) {
        Ok(lines) => refuse(ctx, &loaded.form, shown, lines),
        Err(_empty) => WebError::Internal {
            request_id: ctx.request.request_id(),
        },
    }
}

/// Answer `fields=1`: the grid of the sent type, or the line of a type that does not parse.
async fn fields(ctx: Ctx<'_>, sent: &SentFields) -> Result<Response, WebError> {
    let text = sent.text("type").unwrap_or_default().trim();
    if text.is_empty() {
        return again(ctx, sent, None, "").await;
    }
    match TypeExpr::parse(text) {
        Ok(expr) => again(ctx, sent, Some(expr), "").await,
        Err(error) => {
            let line = FieldError::plant(&error, Some("type".to_owned()));
            Err(refuse_lines(ctx, sent, vec![line]).await)
        }
    }
}

/// Turn the value fields into the texts of the leaves of the preview tree.
///
/// A form without a preview tree has a line already, so it gives `None` and no line.
fn leaf_texts(
    form: &ParameterForm,
    sent: &SentFields,
    lines: &mut Vec<FieldError>,
) -> Result<Option<LeafTexts>, WebError> {
    let Some(tree) = form.preview.as_ref() else {
        return Ok(None);
    };
    match sent.leaf_texts(tree) {
        Ok(texts) => Ok(Some(texts)),
        Err(WebError::BadRequest { field, reason }) => {
            lines.push(FieldError::bad_request(field, reason));
            Ok(None)
        }
        Err(other) => Err(other),
    }
}

/// Run the form query with the preview of `type_expr` and build the input of the command.
async fn ready(
    ctx: Ctx<'_>,
    sent: &SentFields,
    checked: Checked,
    type_expr: TypeExpr,
) -> Result<Ready, WebError> {
    let loaded = load_shown(ctx, Some(type_expr), None).await?;
    let shown = sent_shown(ctx, &loaded.form, sent, loaded.previewed.as_ref());
    let mut lines = checked.lines;
    lines.extend(loaded.line);
    let value = leaf_texts(&loaded.form, sent, &mut lines)?;
    let limits = sent.limit_list().map_err(|bad| lines.extend(bad)).ok();
    match (
        NonEmpty::try_from(lines),
        (checked.definition, value, limits),
    ) {
        (Ok(lines), _parts) => Err(refuse(ctx, &loaded.form, shown, lines)),
        (Err(_empty), (Some(definition), Some(value), Some(limits))) => Ok(Ready {
            form: loaded.form,
            definition,
            shown,
            value,
            limits,
        }),
        (Err(_empty), _missing) => Err(WebError::Internal {
            request_id: ctx.request.request_id(),
        }),
    }
}

/// Check a save of a parameter form (ADR-0039 (h) and (i), ADR-0041 (a)).
///
/// `fields=1` shows the grid of the sent type. `limit_mode` switches the limit mode of the shown
/// grid. A type that differs from `value_type` shows the grid of the new type with a notice. These
/// three answers write nothing and keep the sent texts and the sent `row_version`.
///
/// # Errors
///
/// Returns the lines of every bad field with the form, or the error of the form query.
pub(crate) async fn prepare(ctx: Ctx<'_>, sent: &SentFields) -> Result<Prepared, WebError> {
    if sent.wants_fields() {
        return fields(ctx, sent).await.map(Prepared::Answer);
    }
    if sent.limit_mode().is_some() {
        let shown_type = sent.value_type().cloned();
        return again(ctx, sent, shown_type, "").await.map(Prepared::Answer);
    }
    let checked = check(sent);
    let Some(type_expr) = checked.type_expr.clone() else {
        return Err(refuse_lines(ctx, sent, checked.lines).await);
    };
    if sent.value_type() != Some(&type_expr) {
        let answer = again(ctx, sent, Some(type_expr), "value-type-notice").await?;
        return Ok(Prepared::Answer(answer));
    }
    let ready = ready(ctx, sent, checked, type_expr).await?;
    Ok(Prepared::Ready(Box::new(ready)))
}
