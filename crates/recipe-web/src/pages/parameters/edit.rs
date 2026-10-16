//! `POST .../parameters/{key}/definition`: the save of the edit form (`api.md` section 2.6,
//! design D7, ADR-0039 (i), ADR-0041 (h)).
//!
//! A valid HTMX save gets the new definition for `#parameter-summary`, the toast, the empty
//! `#error-panel`, and the clean form out of band with the stored texts and the new `row_version`.
//! A save without HTMX gets `303` to the parameter page.

use axum::extract::rejection::{PathRejection, RawFormRejection};
use axum::extract::{Path, RawForm, State};
use axum::response::Response;
use recipe_app::auth::ParameterFormPurpose;
use recipe_app::recipe::{
    DraftOutput, ParameterForm, RetypeInput, UpdateParameterDefinition,
    UpdateParameterDefinitionInput,
};
use recipe_domain::recipe::{LeafTexts, LimitList, ParameterKey, ValueInput};

use super::definition::Definition;
use super::fields::{FieldSet, read_form};
use super::form_view::{ParameterFormFragment, ParameterFormView, Place};
use super::page::{read_parameter, stored_shown};
use super::path_parameter;
use super::prepare::{Prepared, Ready, prepare};
use super::reply::{Ctx, Target, failed, load};
use super::summary::{SummaryFragment, SummaryView};
use crate::error::{FormErrors, WebError};
use crate::pages::draft_reply::{success, with_form};
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::state::WebState;

/// Build the input of `UpdateParameterDefinition`. The form always sends the value and the limits.
fn update_input(
    target: &Target,
    key: ParameterKey,
    definition: Definition,
    (value, limits): (LeafTexts, LimitList),
) -> UpdateParameterDefinitionInput {
    UpdateParameterDefinitionInput {
        reference: target.reference.clone(),
        number: target.number,
        expected_row_version: definition.row_version,
        key,
        unit_path: definition.unit_path,
        tag: definition.tag,
        name_en: definition.name_en,
        description_en: definition.description_en,
        type_expr: definition.type_expr,
        s7_address: definition.s7_address,
        ordinal: definition.ordinal,
        retype: Some(RetypeInput {
            value: ValueInput::Texts(value),
            limits,
        }),
    }
}

/// Answer a valid save: the summary and the clean form, or `303` to the page without HTMX.
///
/// The clean form reads the parameter again, so it shows the canonical texts of the store and not
/// the sent texts, for example `1.5` for `1.50` (ADR-0041 (h), ADR-0039 (c), A43).
async fn saved(
    ctx: Ctx<'_>,
    (form, definition): (&ParameterForm, &Definition),
    (key, output): (ParameterKey, DraftOutput),
) -> Result<Response, WebError> {
    let request = ctx.request;
    let request_id = request.request_id();
    if !request.is_htmx() {
        let page = format!("{}/parameters/{key}", ctx.target.version_href());
        return see_other_path(&page, request_id, []);
    }
    let t = ctx.texts();
    let toast = t.with_arg("parameter-saved", "tag", &definition.tag);
    let summary = SummaryView::saved(key, definition, toast);
    let body = render(&SummaryFragment { t, summary }, request_id)?;
    let mut clean = stored_shown(ctx, &read_parameter(ctx).await?)?;
    // `HX-Trigger` sends this `row_version`. A newer write of another user then gives a conflict.
    clean.texts.row_version = output.row_version.to_string();
    let csrf = request.csrf_token();
    let place = (clean, Place::OutOfBand);
    let view = ParameterFormView::new(t, form, place, csrf, FormErrors::default());
    let body = with_form(body, &ParameterFormFragment { t, form: view }, request_id)?;
    success(&output, body, request_id)
}

/// `POST .../parameters/{key}/definition`: save the definition, the value, and the limits.
///
/// `fields=1`, `limit_mode`, and a changed type show the form again and write nothing.
pub(crate) async fn save(
    State(state): State<WebState>,
    path: Result<Path<(String, String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<RawForm, RawFormRejection>,
) -> Result<Response, WebError> {
    let target = path_parameter(path)?;
    let ctx = Ctx {
        state: &state,
        request: &request,
        target: &target,
    };
    // The form query checks the permission of `UpdateParameterDefinition` before the fields are
    // read (ADR-0046).
    load(
        ctx,
        ParameterFormPurpose::UpdateParameterDefinition,
        None,
        None,
    )
    .await?;
    let sent = read_form(form, FieldSet::Parameter)?;
    let key = target.key.ok_or(WebError::Internal {
        request_id: request.request_id(),
    })?;
    let Ready {
        form,
        definition,
        shown,
        value,
        limits,
    } = match prepare(ctx, &sent).await? {
        Prepared::Answer(answer) => return Ok(answer),
        Prepared::Ready(ready) => *ready,
    };
    let input = update_input(&target, key, definition.clone(), (value, limits));
    let result = UpdateParameterDefinition::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    match result {
        Ok(output) => saved(ctx, (&form, &definition), (key, output)).await,
        Err(error) => Err(failed(ctx, &form, shown, error).await),
    }
}
