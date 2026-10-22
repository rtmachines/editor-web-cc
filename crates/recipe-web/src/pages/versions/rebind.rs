//! `POST /recipes/{ref}/versions/{n}/rebind` (`api.md` section 2.5, ADR-0043 (l), design D7).
//!
//! A valid HTMX rebind gets the parameter table, the new outdated list out of band, the toast, and
//! `HX-Trigger` with the new `row_version`. A rebind without HTMX gets `303` to the version page.
//! A list of `E-RCP-028`, `E-RCP-029`, or `E-RCP-005` gets one line per entry and the form with
//! the sent texts out of band (ADR-0041). A stale `row_version` gets the conflict.

use std::collections::{BTreeMap, BTreeSet};

use askama::Template;
use axum::extract::rejection::{PathRejection, RawFormRejection};
use axum::extract::{Path, RawForm, State};
use axum::response::{Html, Response};
use recipe_app::auth::{Operation, ParameterFormPurpose};
use recipe_app::error::AppError;
use recipe_app::ports::NonEmpty;
use recipe_app::recipe::{
    DraftOutput, GetParameterForm, GetParameterFormInput, GetVersion, GetVersionInput,
    ListParameters, ListParametersInput, RebindTypeRevision, RebindTypeRevisionInput,
};
use recipe_domain::plant::{LeafPath, PlcTypeId};
use recipe_domain::recipe::{MemberValue, ParameterKey, ReferenceNumber};
use uuid::Uuid;

use super::outdated::{OutdatedList, version_list};
use super::parameter_table::ParameterTable;
use super::path_version;
use super::rebind_form::{RebindForm, posted_form, shows_forms};
use crate::error::{FieldError, FieldReason, FormErrors, WebError};
use crate::pages::draft_reply::{conflict, domain_line, refused, success};
use crate::pages::parameters::fields::{FieldSet, SentFields, read_form};
use crate::pages::path_values::page as read_page;
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The largest number of digits of a `row_version`: the digits of `i64::MAX`.
const ROW_VERSION_DIGITS: usize = 19;
/// The length of a type row identifier in the hyphenated form of a UUID.
const TYPE_ID_LENGTH: usize = 36;

/// The values of the new member leaves, by parameter and leaf path.
type MemberValues = BTreeMap<(ParameterKey, LeafPath), MemberValue>;

/// The answers of a rebind in `rebind-form.html`: the form of an error, or the list and the toast.
#[derive(Template)]
#[template(path = "rebind-form.html")]
struct RebindFragment<'a> {
    t: Texts<'a>,
    /// The form of an error answer. `None` gives the answer of a success.
    form: Option<RebindForm>,
    /// The list of a success answer, out of band.
    outdated: OutdatedList,
    /// The sentence `rebind-done` of a success answer.
    toast: String,
}

/// The parameter table alone: the target of a rebind answer.
#[derive(Template)]
#[template(path = "parameter-table.html")]
struct TableFragment<'a> {
    t: Texts<'a>,
    table: ParameterTable,
}

/// The hidden fields of the form: the row version and the two type rows.
struct Hidden {
    row_version: i64,
    row_text: String,
    from: PlcTypeId,
    to: PlcTypeId,
}

/// Read a `row_version`: one to 19 digits that fit an `i64`.
fn row_number(text: &str) -> Option<i64> {
    let is_digits = (1..=ROW_VERSION_DIGITS).contains(&text.len())
        && text.bytes().all(|byte| byte.is_ascii_digit());
    is_digits.then(|| text.parse().ok()).flatten()
}

/// Read a type row identifier. Only the hyphenated form of a UUID with 36 characters is valid.
fn type_row(text: Option<&str>) -> Option<PlcTypeId> {
    let text = text.filter(|text| text.len() == TYPE_ID_LENGTH)?;
    Uuid::try_parse(text).ok().map(PlcTypeId::from)
}

/// Read the hidden fields. Collect a line for each bad field (ADR-0041 (a)).
///
/// A bad `row_version` gives one line. A bad type row gives one line for both type rows.
fn hidden_fields(sent: &SentFields) -> Result<Hidden, NonEmpty<FieldError>> {
    let row_text = sent.text("row_version").unwrap_or_default().to_owned();
    let row_version = row_number(&row_text);
    let from = type_row(sent.text("from_type_id"));
    let to = type_row(sent.text("to_type_id"));
    if let (Some(row_version), Some(from), Some(to)) = (row_version, from, to) {
        return Ok(Hidden {
            row_version,
            row_text,
            from,
            to,
        });
    }
    let types = FieldError::bad_request("form", FieldReason::Reload);
    if row_version.is_none() {
        let mut lines = NonEmpty::new(FieldError::bad_request("row_version", FieldReason::Reload));
        if from.is_none() || to.is_none() {
            lines.push(types);
        }
        return Err(lines);
    }
    Err(NonEmpty::new(types))
}

/// Return the texts of the values, for a form that the answer renders again.
fn texts_of(values: &MemberValues) -> BTreeMap<(ParameterKey, LeafPath), String> {
    values
        .iter()
        .filter_map(|(pair, value)| match value {
            MemberValue::Text(text) => Some((pair.clone(), text.clone())),
            MemberValue::Json(_json) => None,
        })
        .collect()
}

/// Run the form query of the draft before the fields are read (ADR-0046 (a)).
///
/// `GetParameterForm` with the purpose `RebindTypeRevision` checks the permission of that command.
/// It refuses a deleted recipe and a version that is not a live draft, as the command does. So a
/// role without the permission gets `403 E-WEB-002` and one event `forbidden` that names
/// `RebindTypeRevision`, and never a line of a field.
async fn check_access(
    state: &WebState,
    request: &PageRequest,
    (reference, number): (&ReferenceNumber, u32),
) -> Result<(), WebError> {
    let input = GetParameterFormInput {
        purpose: ParameterFormPurpose::RebindTypeRevision,
        reference: reference.clone(),
        number,
        key: None,
        type_expr: None,
        unit: None,
    };
    GetParameterForm::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    Ok(())
}

/// Build the form of the request: the rebind form of the page for the sent pair of revisions.
///
/// A role without `ParameterEdit` gets `None`, so the command refuses it first and writes the
/// security event. A pair that no parameter of the draft moves also gives `None`.
async fn sent_form(
    state: &WebState,
    request: &PageRequest,
    (reference, number): (&ReferenceNumber, u32),
    hidden: &Hidden,
) -> Result<Option<RebindForm>, WebError> {
    let role = request.auth()?.user().role();
    if !Operation::RebindTypeRevision.requirement().allows(role) {
        return Ok(None);
    }
    let input = GetVersionInput {
        reference: reference.clone(),
        number,
        rebind_bound: state.settings().form_rebind_leaves,
    };
    let view = GetVersion::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    if !shows_forms(request, &view)? {
        return Ok(None);
    }
    let t = Texts::new(state.bundles(), request.lang());
    Ok(posted_form(t, request, &view, (hidden.from, hidden.to)))
}

/// Return the values of the new member leaves as the command takes them (ADR-0039 (e)).
///
/// The planned tree of each parameter of the form gives the class of a leaf. A `String` or `Char`
/// leaf keeps its text, and every other class trims it.
///
/// # Errors
///
/// Returns one line `E-WEB-008` when the form is known and does not hold a sent field.
fn member_values(
    sent: &SentFields,
    form: Option<&RebindForm>,
) -> Result<MemberValues, NonEmpty<FieldError>> {
    let values = sent.member_values(&|key| form.and_then(|form| form.tree_of(*key)));
    let unknown = form.is_some_and(|form| values.keys().any(|(key, path)| !form.holds(*key, path)));
    if unknown {
        return Err(NonEmpty::new(FieldError::bad_request(
            "form",
            FieldReason::ValueKey,
        )));
    }
    Ok(values)
}

/// Return the lines of a refused rebind, or `None` for an error that keeps its own answer.
///
/// A list gives one line per entry. A single domain error gives one line, under its input or only
/// in the summary (ADR-0041 (e)).
fn list_lines(error: &AppError, form: &RebindForm) -> Option<NonEmpty<FieldError>> {
    if let AppError::Recipes(list) = error {
        let lines: Vec<FieldError> = list
            .iter()
            .map(|entry| FieldError::recipe(entry, form.target_of(entry)))
            .collect();
        return NonEmpty::try_from(lines).ok();
    }
    let target = match error {
        AppError::Recipe(single) => form.target_of(single),
        _other => None,
    };
    domain_line(error, target).map(NonEmpty::new)
}

/// Turn the error of the command into its answer (design D9 and D10).
async fn refusal(
    state: &WebState,
    request: &PageRequest,
    error: AppError,
    (form, values, hidden): (Option<RebindForm>, &MemberValues, &Hidden),
) -> WebError {
    if matches!(error, AppError::ConcurrentEdit { .. }) {
        return conflict(state, request, error).await;
    }
    let Some(form) = form else {
        return WebError::from(error);
    };
    let Some(lines) = list_lines(&error, &form) else {
        return WebError::from(error);
    };
    let t = Texts::new(state.bundles(), request.lang());
    let row_text = hidden.row_text.clone();
    refused(state, request, lines, |errors: FormErrors| RebindFragment {
        t,
        form: Some(form.sent(&texts_of(values), row_text, errors)),
        outdated: OutdatedList::new("", &[]),
        toast: String::new(),
    })
}

/// Answer a valid HTMX rebind: the table, the new list out of band, the toast, and `HX-Trigger`.
async fn rebound(
    state: &WebState,
    request: &PageRequest,
    (reference, number): (&ReferenceNumber, u32),
    (output, to_text): (&DraftOutput, &str),
) -> Result<Response, WebError> {
    let user = request.auth()?.user();
    let ctx = state.identity().app().clone();
    let target = GetVersionInput {
        reference: reference.clone(),
        number,
        rebind_bound: state.settings().form_rebind_leaves,
    };
    let view = GetVersion::new(ctx.clone())
        .execute(user, &request.meta(), target)
        .await?;
    let input = ListParametersInput {
        reference: reference.clone(),
        number,
        unit: None,
        page: 0,
    };
    let list = ListParameters::new(ctx)
        .execute(user, &request.meta(), input)
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let base = format!("/recipes/{reference}/versions/{number}");
    let keys: BTreeSet<ParameterKey> = view.outdated.iter().map(|item| item.key).collect();
    let table = ParameterTable::new(t, &base, &list, &keys, read_page(None)?);
    let fragment = RebindFragment {
        t,
        form: None,
        outdated: version_list(request, &view, t)?.out_of_band(),
        toast: t.with_arg("rebind-done", "to", to_text),
    };
    let Html(mut body) = render(&TableFragment { t, table }, request.request_id())?;
    body.push_str(&render(&fragment, request.request_id())?.0);
    success(output, Html(body), request.request_id())
}

/// `POST /recipes/{ref}/versions/{n}/rebind`: move the parameters of the draft to the newer revision.
pub(crate) async fn rebind(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<RawForm, RawFormRejection>,
) -> Result<Response, WebError> {
    let (reference, number) = path_version(path)?;
    check_access(&state, &request, (&reference, number)).await?;
    let sent = read_form(form, FieldSet::Rebind)?;
    let fields = |lines| WebError::Fields {
        lines,
        form: None,
        failure: None,
    };
    let hidden = hidden_fields(&sent).map_err(fields)?;
    let form = sent_form(&state, &request, (&reference, number), &hidden).await?;
    let values = member_values(&sent, form.as_ref()).map_err(fields)?;
    let input = RebindTypeRevisionInput {
        reference: reference.clone(),
        number,
        expected_row_version: hidden.row_version,
        from: hidden.from,
        to: hidden.to,
        new_member_values: values.clone(),
    };
    let result = RebindTypeRevision::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            let parts = (form, &values, &hidden);
            return Err(refusal(&state, &request, error, parts).await);
        }
    };
    if !request.is_htmx() {
        let page = format!("/recipes/{reference}/versions/{number}");
        return see_other_path(&page, request.request_id(), []);
    }
    let to_text = form.map_or_else(|| hidden.to.to_string(), |form| form.to_text);
    rebound(&state, &request, (&reference, number), (&output, &to_text)).await
}

#[cfg(test)]
mod lines_tests;
