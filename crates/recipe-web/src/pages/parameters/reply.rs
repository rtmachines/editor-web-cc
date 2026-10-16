//! The answers of the parameter forms: the form again, the form with errors, and the lines of a
//! failed command (design D7, D9, ADR-0039 (h), ADR-0041 (e)).
//!
//! The module solves one problem: the form of a new parameter and the edit form answer in the same
//! ways. A request without a write shows the form again with `200`. A bad request shows the lines
//! and swaps the form out of band. One set of functions gives both forms these answers.

use axum::http::{HeaderName, HeaderValue};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::ParameterFormPurpose;
use recipe_app::error::{AppError, Missing};
use recipe_app::ports::NonEmpty;
use recipe_app::recipe::{GetParameterForm, GetParameterFormInput, ParameterForm};
use recipe_domain::plant::{PlantError, TypeExpr, UnitPath};
use recipe_domain::recipe::{ParameterKey, RecipeError, ReferenceNumber};

use super::fields::SentFields;
use super::form_view::{
    DefinitionTexts, GridTexts, ParameterFormFragment, ParameterFormView, Place, Shown, grid_of,
};
use super::grid::GridView;
use super::summary::ParameterPage;
use crate::error::{FieldError, FormErrors, WebError, leaf_of};
use crate::layout::{Layout, NavTarget};
use crate::pages::draft_reply::{conflict, domain_line, refused};
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The HTMX header that names the element that takes the answer.
const HX_RETARGET: HeaderName = HeaderName::from_static("hx-retarget");
/// The HTMX header that names the swap of the answer.
const HX_RESWAP: HeaderName = HeaderName::from_static("hx-reswap");
/// The HTMX header that sets the address in the history.
const HX_PUSH_URL: HeaderName = HeaderName::from_static("hx-push-url");
/// The value of `HX-Target` of the page form of a new parameter: the `id` of `main` (design D7).
const CONTENT: &str = "content";

/// One parameter form of one version, as the URL names it.
pub(crate) struct Target {
    /// The reference number of the recipe.
    pub(crate) reference: ReferenceNumber,
    /// The number of the version.
    pub(crate) number: u32,
    /// The key of the parameter of the edit form, or `None` for a new parameter.
    pub(crate) key: Option<ParameterKey>,
}

impl Target {
    /// Return the address of the version page, for example `/recipes/R-100/versions/3`.
    pub(crate) fn version_href(&self) -> String {
        format!("/recipes/{}/versions/{}", self.reference, self.number)
    }

    /// Return the command of the definition form of the target (ADR-0046 (b)).
    ///
    /// The form of a new parameter sends `AddParameter`. The edit form of a key sends
    /// `UpdateParameterDefinition`.
    pub(crate) const fn form_purpose(&self) -> ParameterFormPurpose {
        match self.key {
            None => ParameterFormPurpose::AddParameter,
            Some(_key) => ParameterFormPurpose::UpdateParameterDefinition,
        }
    }
}

/// What every answer of a parameter form needs: the state, the request, and the form.
#[derive(Clone, Copy)]
pub(crate) struct Ctx<'a> {
    /// The state of the router.
    pub(crate) state: &'a WebState,
    /// The data of the request.
    pub(crate) request: &'a PageRequest,
    /// The form that the URL names.
    pub(crate) target: &'a Target,
}

impl Ctx<'_> {
    /// Return the texts of the UI language.
    pub(crate) fn texts(&self) -> Texts<'_> {
        Texts::new(self.state.bundles(), self.request.lang())
    }

    /// Return true when the page form of a new parameter sent the request (design D7).
    ///
    /// That form targets `#content`, so HTMX sends `HX-Target: content`. The value selects only the
    /// shape of the answer. The command, its permission, and its action stay the same.
    pub(crate) fn is_page_form(&self) -> bool {
        self.target.key.is_none()
            && self.request.is_htmx()
            && self.request.hx_target() == Some(CONTENT)
    }
}

/// Turn `Missing::Parameter` into `E-RCP-051`, the code of a key that the draft does not hold.
///
/// A `POST` route with `{key}` gives `404 E-RCP-051`, also when `fields=1` runs the form query
/// (design D6, `api.md` section 2.6).
pub(crate) fn missing_key(error: AppError) -> AppError {
    match error {
        AppError::NotFound {
            entity:
                Missing::Parameter {
                    reference,
                    number,
                    key,
                },
        } => RecipeError::ParameterNotFound {
            reference,
            number,
            key,
        }
        .into(),
        other => other,
    }
}

/// Return the error of a type that the form query cannot resolve: `E-PLT-013` or `E-PLT-018`.
fn type_error(error: &AppError) -> Option<&PlantError> {
    match error {
        AppError::Plant(
            inner @ (PlantError::DerivedTypeInvalid { .. } | PlantError::TypeTooLarge { .. }),
        ) => Some(inner),
        _other => None,
    }
}

/// Run `GetParameterForm` for the form of `ctx` with the preview of `type_expr`.
///
/// The query checks the permission of the command `purpose` first, so a route runs it before it
/// reads its fields (ADR-0046 (a)).
///
/// # Errors
///
/// Returns the error of the query. `Missing::Parameter` becomes `E-RCP-051`.
pub(crate) async fn load(
    ctx: Ctx<'_>,
    purpose: ParameterFormPurpose,
    type_expr: Option<TypeExpr>,
    unit: Option<UnitPath>,
) -> Result<ParameterForm, AppError> {
    let input = GetParameterFormInput {
        purpose,
        reference: ctx.target.reference.clone(),
        number: ctx.target.number,
        key: ctx.target.key,
        type_expr,
        unit,
    };
    let user = ctx
        .request
        .auth()
        .map_err(|_none| AppError::Unauthenticated)?
        .user();
    GetParameterForm::new(ctx.state.identity().app().clone())
        .execute(user, &ctx.request.meta(), input)
        .await
        .map_err(missing_key)
}

/// The result of the form query with a preview: the form, the type of its preview, and the line of
/// a type that the query cannot resolve.
pub(crate) struct Loaded {
    /// The data of the form.
    pub(crate) form: ParameterForm,
    /// The type of the preview tree of `form`, or `None` without a preview.
    pub(crate) previewed: Option<TypeExpr>,
    /// The line `E-PLT-013` or `E-PLT-018` under the input `type`, if the wanted type failed.
    pub(crate) line: Option<FieldError>,
}

/// Run the form query with the preview of `wanted`.
///
/// A type that the query cannot resolve gives the line of that type under the input `type`. The form
/// then shows the grid of `fallback`, the type of the shown grid, so the sent texts stay. A fallback
/// that fails too gives the form without a grid.
///
/// # Errors
///
/// Returns every other error of the query.
pub(crate) async fn load_shown(
    ctx: Ctx<'_>,
    wanted: Option<TypeExpr>,
    fallback: Option<TypeExpr>,
) -> Result<Loaded, WebError> {
    let purpose = ctx.target.form_purpose();
    let error = match load(ctx, purpose, wanted.clone(), None).await {
        Ok(form) => {
            return Ok(Loaded {
                form,
                previewed: wanted,
                line: None,
            });
        }
        Err(error) => error,
    };
    let Some(inner) = type_error(&error) else {
        return Err(error.into());
    };
    let line = Some(FieldError::plant(inner, Some("type".to_owned())));
    match load(ctx, purpose, fallback.clone(), None).await {
        Ok(form) => Ok(Loaded {
            form,
            previewed: fallback,
            line,
        }),
        Err(again) if type_error(&again).is_some() => Ok(Loaded {
            form: load(ctx, purpose, None, None).await?,
            previewed: None,
            line,
        }),
        Err(again) => Err(again.into()),
    }
}

/// Build what a form shows for the sent fields: the sent texts and the grid of the preview tree.
///
/// `previewed` is the type of the preview tree. The hidden field `value_type` names it.
pub(crate) fn sent_shown(
    ctx: Ctx<'_>,
    form: &ParameterForm,
    sent: &SentFields,
    previewed: Option<&TypeExpr>,
) -> Shown {
    let texts = DefinitionTexts::from_sent(sent);
    let chunk = ctx.state.settings().form_array_chunk;
    let grid = form
        .preview
        .as_ref()
        .map(|tree| grid_of(ctx.texts(), GridTexts::sent(tree, sent), chunk, &texts.tag));
    let value_type = match (&grid, previewed) {
        (Some(_grid), Some(expr)) => expr.to_string(),
        _other => String::new(),
    };
    Shown {
        texts,
        grid,
        value_type,
        notice: String::new(),
    }
}

/// Return the title of the version of `ctx`, for example "Version 3 of recipe R-100".
pub(crate) fn version_text(ctx: Ctx<'_>) -> String {
    let mut args = fluent::FluentArgs::new();
    args.set("number", ctx.target.number);
    args.set("reference", ctx.target.reference.to_string());
    ctx.texts().with("version-title", &args)
}

/// Render the boosted form alone (design D7).
///
/// # Errors
///
/// Returns `E-WEB-010` when the template cannot render.
pub(crate) fn form_fragment(
    ctx: Ctx<'_>,
    form: &ParameterForm,
    shown: Shown,
) -> Result<Response, WebError> {
    let t = ctx.texts();
    let request = ctx.request;
    let csrf = request.csrf_token();
    let view = ParameterFormView::new(
        t,
        form,
        (shown, Place::Fragment),
        csrf,
        FormErrors::default(),
    );
    let fragment = ParameterFormFragment { t, form: view };
    Ok(render(&fragment, request.request_id())?.into_response())
}

/// Render the page of the form: the full page, or the content of `main` (ADR-0037 (w)).
///
/// # Errors
///
/// Returns `E-WEB-010` when the template cannot render, or `E-WEB-004` without a session.
pub(crate) fn form_page(
    ctx: Ctx<'_>,
    form: &ParameterForm,
    shown: Shown,
) -> Result<Response, WebError> {
    let t = ctx.texts();
    let request = ctx.request;
    let title = match form.parameter.as_ref() {
        Some(parameter) => t.with_arg("parameter-title", "tag", &parameter.tag),
        None => t.get("parameter-new-title"),
    };
    let layout = Layout::for_session(t, request, title, Some(NavTarget::Recipes))?;
    let csrf = layout.csrf_token.clone();
    let view = ParameterFormView::new(t, form, (shown, Place::Page), csrf, FormErrors::default());
    let page = ParameterPage {
        summary: None,
        form: Some(view),
        values: None,
        translations: None,
        remove: None,
        layout,
        t,
        version_href: ctx.target.version_href(),
        version_text: version_text(ctx),
    };
    Ok(render(&page, request.request_id())?.into_response())
}

/// Answer the form again after a request without a write (ADR-0039 (h)).
///
/// An HTMX answer names the form as its target, so `fields=1` and `limit_mode` replace the form
/// and not the parameter table. A request without HTMX gets the page of the form. The page form
/// gets the content of its page with `HX-Push-Url: false`, so the address bar keeps the URL.
///
/// # Errors
///
/// Returns `E-WEB-010` when a template cannot render, or `E-WEB-004` without a session.
pub(crate) fn show_form(
    ctx: Ctx<'_>,
    form: &ParameterForm,
    shown: Shown,
) -> Result<Response, WebError> {
    if !ctx.request.is_htmx() {
        return form_page(ctx, form, shown);
    }
    if ctx.is_page_form() {
        let mut response = form_page(ctx, form, shown)?;
        response
            .headers_mut()
            .insert(HX_PUSH_URL, HeaderValue::from_static("false"));
        return Ok(response);
    }
    let id = if form.parameter.is_some() {
        "#parameter-edit-form"
    } else {
        "#parameter-new-form"
    };
    let mut response = form_fragment(ctx, form, shown)?;
    let headers = response.headers_mut();
    headers.insert(HX_RETARGET, HeaderValue::from_static(id));
    headers.insert(HX_RESWAP, HeaderValue::from_static("outerHTML"));
    Ok(response)
}

/// Build the error of a form with every line and the form with the sent input (ADR-0041 (e)).
///
/// A group of the grid that holds a line opens (ADR-0039 (l)). The out-of-band form keeps the
/// attributes of the form that sent the request (design D7).
pub(crate) fn refuse(
    ctx: Ctx<'_>,
    form: &ParameterForm,
    shown: Shown,
    lines: NonEmpty<FieldError>,
) -> WebError {
    let t = ctx.texts();
    let csrf = ctx.request.csrf_token();
    let place = if ctx.is_page_form() {
        Place::PageOutOfBand
    } else {
        Place::OutOfBand
    };
    refused(ctx.state, ctx.request, lines, |errors| {
        let shown = Shown {
            grid: shown.grid.map(|grid| grid.with_errors(&errors)),
            ..shown
        };
        ParameterFormFragment {
            t,
            form: ParameterFormView::new(t, form, (shown, place), csrf, errors),
        }
    })
}

/// Return the input of a recipe error that names a field of the form, or `None`.
fn field_of(error: &RecipeError) -> Option<&'static str> {
    match error {
        RecipeError::DuplicateTag { .. } => Some("tag"),
        RecipeError::S7AddressRequired { .. }
        | RecipeError::S7AddressOverlap { .. }
        | RecipeError::S7AddressMisaligned { .. }
        | RecipeError::S7AddressNotAllowed { .. }
        | RecipeError::S7AddressWidthMismatch { .. } => Some("s7_address"),
        _other => None,
    }
}

/// Return the input of a plant error of a command, or `None`.
const fn plant_field_of(error: &PlantError) -> Option<&'static str> {
    match error {
        PlantError::UnitNotFound { .. } | PlantError::UnitArchived { .. } => Some("unit_path"),
        PlantError::DerivedTypeInvalid { .. } | PlantError::TypeTooLarge { .. } => Some("type"),
        _other => None,
    }
}

/// Return the lines of a failed command whose faults stand under inputs of the form, or `None`.
///
/// A list of faults of a value and its limits gives one line per fault. `input_id` gives the input
/// of a fault of a leaf: the value input or a limit input (ADR-0039 (n)). A fault of one field
/// gives one line under that field. Any other error has no input.
pub(crate) fn command_lines(
    error: &AppError,
    input_id: &dyn Fn(&RecipeError) -> Option<String>,
) -> Option<NonEmpty<FieldError>> {
    let lines = match error {
        AppError::Recipes(errors) => errors
            .iter()
            .map(|fault| FieldError::recipe(fault, input_id(fault)))
            .collect(),
        AppError::Recipe(inner) => {
            let target = match leaf_of(inner) {
                Some(_path) => input_id(inner),
                None => Some(field_of(inner)?.to_owned()),
            };
            vec![FieldError::recipe(inner, target)]
        }
        AppError::Plant(inner) => {
            vec![FieldError::plant(
                inner,
                Some(plant_field_of(inner)?.to_owned()),
            )]
        }
        _other => return None,
    };
    NonEmpty::try_from(lines).ok()
}

/// Turn the error of a draft command into its answer (design D9, D10).
///
/// `AppError::ConcurrentEdit` gives the conflict answer. An error with lines under inputs gives the
/// form with the lines. The grid gives the input of a fault of a leaf. A domain error without an
/// input also gives the form, with one line that stands only in the summary (ADR-0041 (e)). Any
/// other error stays as it is.
pub(crate) async fn failed(
    ctx: Ctx<'_>,
    form: &ParameterForm,
    shown: Shown,
    error: AppError,
) -> WebError {
    if matches!(error, AppError::ConcurrentEdit { .. }) {
        return conflict(ctx.state, ctx.request, error).await;
    }
    let lines = {
        let grid: Option<&GridView> = shown.grid.as_ref();
        command_lines(&error, &|fault| grid.and_then(|grid| grid.input_of(fault)))
            .or_else(|| domain_line(&error, None).map(NonEmpty::new))
    };
    match lines {
        Some(lines) => refuse(ctx, form, shown, lines),
        None => WebError::App(missing_key(error)),
    }
}
