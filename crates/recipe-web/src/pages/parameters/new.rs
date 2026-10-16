//! `GET .../parameters/new` and `POST .../parameters` (`api.md` section 2.6, design D6 and D7,
//! ADR-0043 (i)).
//!
//! An HTMX request with `HX-Target: parameter-form` gets the form fragment. Any other request gets
//! the page of the form. A valid HTMX save gets the row of the new parameter for the parameter
//! table, the toast, and the empty regions `#parameter-form` and `#error-panel`. A save without
//! HTMX and a save of the page form (`HX-Target: content`) get `303` to the version page.

use axum::extract::rejection::{PathRejection, QueryRejection, RawFormRejection};
use axum::extract::{Path, Query, RawForm, State};
use axum::response::Response;
use recipe_app::auth::ParameterFormPurpose;
use recipe_app::recipe::{AddParameter, AddParameterInput, AddParameterOutput, ParameterForm};
use recipe_domain::recipe::{LeafTexts, LimitList, ValueInput};
use serde::Deserialize;

use super::definition::Definition;
use super::fields::{FieldSet, read_form};
use super::form_view::{DefinitionTexts, Shown};
use super::prepare::{Prepared, Ready, prepare};
use super::reply::{Ctx, Target, failed, form_fragment, form_page, load};
use crate::error::WebError;
use crate::pages::draft_reply::success;
use crate::pages::path_values::{query_values, unit_path};
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::pages::versions::parameter_table::{
    ParameterRowFragment, ParameterRowView, has_address,
};
use crate::pages::versions::path_version;
use crate::state::WebState;

/// The value of `HX-Target` that asks for the fragment: the `id` of the region of the form.
const FORM_REGION: &str = "parameter-form";

/// The query string of `GET .../parameters/new`. A key other than `unit` gives `422 E-WEB-008`.
///
/// The form query checks the permission first, so a role without `ParameterEdit` gets `403`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewQuery {
    unit: Option<String>,
}

/// `GET .../parameters/new`: show the empty form of a new parameter (ADR-0043 (i)).
pub(crate) async fn new_form(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    query: Result<Query<NewQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let (reference, number) = path_version(path)?;
    let wanted =
        query_values(query).and_then(|values| values.unit.as_deref().map(unit_path).transpose());
    let target = Target {
        reference,
        number,
        key: None,
    };
    let ctx = Ctx {
        state: &state,
        request: &request,
        target: &target,
    };
    // The form query checks the permission of `AddParameter` before a bad query value gives its
    // answer (ADR-0046 (a)). A bad query value runs the query without a unit.
    let unit = wanted.as_ref().ok().and_then(Option::as_ref).cloned();
    let form = load(ctx, ParameterFormPurpose::AddParameter, None, unit).await?;
    wanted?;
    let shown = Shown {
        texts: DefinitionTexts::empty(&form),
        grid: None,
        value_type: String::new(),
        notice: String::new(),
    };
    if request.is_htmx() && request.hx_target() == Some(FORM_REGION) {
        return form_fragment(ctx, &form, shown);
    }
    form_page(ctx, &form, shown)
}

/// Answer a valid save: the row of the new parameter, or `303` to the version page.
///
/// A request without HTMX and a request of the page form get `303`. The boost of the page form
/// follows it and shows the version page (design D7).
fn added(
    ctx: Ctx<'_>,
    (form, definition): (&ParameterForm, &Definition),
    output: AddParameterOutput,
) -> Result<Response, WebError> {
    let request = ctx.request;
    let base = ctx.target.version_href();
    if !request.is_htmx() || ctx.is_page_form() {
        return see_other_path(&base, request.request_id(), []);
    }
    let t = ctx.texts();
    let row = ParameterRowView {
        href: format!("{base}/parameters/{}", output.key),
        tag: definition.tag.to_string(),
        name: definition.name_en.to_string(),
        type_text: definition.type_expr.to_string(),
        unit: definition.unit_path.to_string(),
        address: definition
            .s7_address
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        outdated: false,
    };
    let fragment = ParameterRowFragment {
        t,
        row,
        has_address: has_address(form.machine.platform()),
        message: t.with_arg("parameter-added", "tag", &definition.tag),
    };
    let body = render(&fragment, request.request_id())?;
    success(&output.draft, body, request.request_id())
}

/// Build the input of `AddParameter` from the checked fields of a save.
fn add_input(
    target: &Target,
    definition: Definition,
    (value, limits): (LeafTexts, LimitList),
) -> AddParameterInput {
    AddParameterInput {
        reference: target.reference.clone(),
        number: target.number,
        expected_row_version: definition.row_version,
        unit_path: definition.unit_path,
        tag: definition.tag,
        name_en: definition.name_en,
        description_en: definition.description_en,
        type_expr: definition.type_expr,
        s7_address: definition.s7_address,
        ordinal: definition.ordinal,
        value: ValueInput::Texts(value),
        limits,
    }
}

/// `POST .../parameters`: add a parameter, or show the form again for `fields=1` or `limit_mode`.
pub(crate) async fn add(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<RawForm, RawFormRejection>,
) -> Result<Response, WebError> {
    let (reference, number) = path_version(path)?;
    let target = Target {
        reference,
        number,
        key: None,
    };
    let ctx = Ctx {
        state: &state,
        request: &request,
        target: &target,
    };
    // The form query checks the permission of `AddParameter` before the fields are read (ADR-0046).
    load(ctx, ParameterFormPurpose::AddParameter, None, None).await?;
    let sent = read_form(form, FieldSet::Parameter)?;
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
    let input = add_input(&target, definition.clone(), (value, limits));
    let result = AddParameter::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    match result {
        Ok(output) => added(ctx, (&form, &definition), output),
        Err(error) => Err(failed(ctx, &form, shown, error).await),
    }
}
