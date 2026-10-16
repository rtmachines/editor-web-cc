//! The routes of the translation rows: `GET .../translations`, `POST .../translations/{lang}`,
//! and `POST .../translations/{lang}/delete` (`api.md` section 2.6, ADR-0040 (a) and (b),
//! ADR-0043 (f)).

use axum::extract::rejection::{FormRejection, PathRejection, QueryRejection};
use axum::extract::{Form, Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::{Operation, ParameterFormPurpose};
use recipe_app::error::{AppError, Missing};
use recipe_app::ports::NonEmpty;
use recipe_app::recipe::{
    GetTranslations, ParameterRef, RemoveTranslation, RemoveTranslationInput, TranslationsView,
    UpsertTranslation, UpsertTranslationInput,
};
use recipe_domain::recipe::{LangCode, Translation, VersionState};
use recipe_domain::text::{LongText, Text};
use serde::Deserialize;

use super::page::{NoQuery, no_query, parameter_page};
use super::path_parameter;
use super::reply::{Ctx, Target, load, missing_key};
use super::translations::{
    RowSource, TranslationRow, TranslationRowFragment, TranslationsBlock, TranslationsFragment,
    translation_rows,
};
use crate::error::{FieldError, FieldReason, WebError};
use crate::pages::draft_reply::{conflict, refusal_lines, refused, success};
use crate::pages::path_values::{lang as read_lang, parameter_key, reference, version_number};
use crate::pages::render::{render, see_other_path};
use crate::pages::request::PageRequest;
use crate::state::WebState;

/// The fields of the form of one translation row.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TranslationFields {
    #[serde(default)]
    row_version: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// The fields of a form that sends only the `row_version`: a removal.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RowFields {
    /// The text of the hidden field `row_version`.
    #[serde(default)]
    pub(crate) row_version: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// Read the hidden field `row_version`: a whole number with no sign and no zero in front.
///
/// # Errors
///
/// Returns `422 E-WEB-008` with the reason `Reload` for any other text.
pub(crate) fn sent_row_version(text: &str) -> Result<i64, WebError> {
    let canonical = !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    canonical
        .then(|| text.parse().ok())
        .flatten()
        .ok_or(WebError::BadRequest {
            field: "row_version",
            reason: FieldReason::Reload,
        })
}

/// Read the route `.../translations/{lang}`. A malformed `{lang}` gives `422 E-RCP-027`.
fn path_translation(
    path: Result<Path<(String, String, String, String)>, PathRejection>,
) -> Result<(Target, LangCode), WebError> {
    let Path((text, number, key, lang)) = path.map_err(|_rejection| {
        WebError::App(AppError::NotFound {
            entity: Missing::MalformedRecipe,
        })
    })?;
    let target = Target {
        reference: reference(&text)?,
        number: version_number(&number)?,
        key: Some(parameter_key(&key)?),
    };
    Ok((target, read_lang(&lang)?))
}

/// Return the address of the parameter page of `target`.
fn parameter_href(target: &Target) -> String {
    let key = target.key.map(|key| key.to_string()).unwrap_or_default();
    format!("{}/parameters/{key}", target.version_href())
}

/// Return the parameter of `target` as the input of the translation queries.
fn parameter_ref(ctx: Ctx<'_>) -> Result<ParameterRef, WebError> {
    let key = ctx.target.key.ok_or(WebError::Internal {
        request_id: ctx.request.request_id(),
    })?;
    Ok(ParameterRef {
        reference: ctx.target.reference.clone(),
        number: ctx.target.number,
        key,
    })
}

/// Build the rows of a parameter from `GetTranslations`, with the toast `toast`.
fn block_of(
    ctx: Ctx<'_>,
    view: &TranslationsView,
    toast: String,
) -> Result<TranslationsBlock, WebError> {
    let role = ctx.request.auth()?.user().role();
    let editable = view.detail.state == VersionState::LiveDraft
        && Operation::UpsertTranslation.requirement().allows(role);
    let source = RowSource {
        parameter_href: parameter_href(ctx.target),
        csrf_token: ctx.request.csrf_token(),
        row_version: view.detail.row_version.to_string(),
        editable,
    };
    let langs = &ctx.state.settings().translation_langs;
    Ok(TranslationsBlock {
        name_en: view.name_en.to_string(),
        description_en: view.description_en.to_string(),
        rows: translation_rows(&source, langs, &view.translations),
        toast,
    })
}

/// Read the rows with `GetTranslations` and render them with the toast `toast`.
async fn rows_body(ctx: Ctx<'_>, toast: String) -> Result<axum::response::Html<String>, WebError> {
    let view = GetTranslations::new(ctx.state.identity().app().clone())
        .execute(
            ctx.request.auth()?.user(),
            &ctx.request.meta(),
            parameter_ref(ctx)?,
        )
        .await?;
    let block = block_of(ctx, &view, toast)?;
    render(
        &TranslationsFragment {
            t: ctx.texts(),
            block,
        },
        ctx.request.request_id(),
    )
}

/// `GET .../translations`: show the rows, or the parameter page without HTMX.
pub(crate) async fn translations(
    State(state): State<WebState>,
    path: Result<Path<(String, String, String)>, PathRejection>,
    query: Result<Query<NoQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let target = path_parameter(path)?;
    no_query(query)?;
    let ctx = Ctx {
        state: &state,
        request: &request,
        target: &target,
    };
    if !request.is_htmx() {
        return parameter_page(ctx).await;
    }
    Ok(rows_body(ctx, String::new()).await?.into_response())
}

/// The language and the sent fields of the row form that an error answer swaps.
type SentRow<'a> = (&'a LangCode, &'a TranslationFields);

/// Turn the error of a translation command into its answer (design D10, ADR-0041 (e)).
///
/// A conflict keeps the input. With `row`, a domain error also swaps the row form with the sent
/// texts and only its line, so no line of an earlier answer stays.
async fn command_failed(ctx: Ctx<'_>, row: Option<SentRow<'_>>, error: AppError) -> WebError {
    if matches!(error, AppError::ConcurrentEdit { .. }) {
        return conflict(ctx.state, ctx.request, error).await;
    }
    let error = missing_key(error);
    let Some((lang, fields)) = row else {
        return WebError::App(error);
    };
    // No error of the row names one of its inputs, so its line stands in the summary.
    match refusal_lines(error, |_refusal| None) {
        Ok(lines) => refuse_row(ctx, lang, fields, lines),
        Err(other) => other,
    }
}

/// Run the form query with `purpose`. It checks that permission before the field check (ADR-0046).
///
/// A role without the right gets `403 E-WEB-002`, no form, and one event that names the command.
/// With `row`, a domain error of the query gets the row form with the sent texts.
///
/// # Errors
///
/// Returns the answer of [`command_failed`] for the error of the query.
async fn check_access(
    ctx: Ctx<'_>,
    purpose: ParameterFormPurpose,
    row: Option<SentRow<'_>>,
) -> Result<(), WebError> {
    match load(ctx, purpose, None, None).await {
        Ok(_form) => Ok(()),
        Err(error) => Err(command_failed(ctx, row, error).await),
    }
}

/// Check the fields of a row. Every bad field gives one line under its input (ADR-0041 (a)).
fn check_row(
    lang: &LangCode,
    fields: &TranslationFields,
) -> Result<(i64, Translation), Vec<FieldError>> {
    let mut lines = Vec::new();
    let version = sent_row_version(&fields.row_version)
        .map_err(|_bad| lines.push(FieldError::bad_request("row_version", FieldReason::Reload)))
        .ok();
    let label = |key: &'static str, reason, input: String| {
        FieldError::bad_input(key, vec![("lang", lang.to_string())], reason, input)
    };
    let name = Text::parse(&fields.name)
        .map_err(|_reason| {
            lines.push(label(
                "field-translation-name",
                FieldReason::Name,
                format!("translation-{lang}-name"),
            ));
        })
        .ok();
    let description = match fields.description.is_empty() {
        true => Some(None),
        false => LongText::parse(&fields.description)
            .map(Some)
            .map_err(|_reason| {
                lines.push(label(
                    "field-translation-description",
                    FieldReason::TranslationDescription,
                    format!("translation-{lang}-description"),
                ));
            })
            .ok(),
    };
    match (version, name, description) {
        (Some(version), Some(name), Some(description)) if lines.is_empty() => Ok((
            version,
            Translation {
                lang: lang.clone(),
                name,
                description,
            },
        )),
        _bad => Err(lines),
    }
}

/// Build the error with `lines` and the form of the row with the sent texts (ADR-0041 (e)).
fn refuse_row(
    ctx: Ctx<'_>,
    lang: &LangCode,
    fields: &TranslationFields,
    lines: NonEmpty<FieldError>,
) -> WebError {
    let source = RowSource {
        parameter_href: parameter_href(ctx.target),
        csrf_token: ctx.request.csrf_token(),
        row_version: fields.row_version.clone(),
        editable: true,
    };
    let t = ctx.texts();
    refused(ctx.state, ctx.request, lines, |errors| {
        let row = TranslationRow {
            name: fields.name.clone(),
            description: fields.description.clone(),
            oob: true,
            errors,
            ..TranslationRow::new(&source, lang, None)
        };
        TranslationRowFragment {
            t,
            row,
            form_only: true,
        }
    })
}

/// Render the new row of a valid save with the toast and the new `row_version`.
fn saved(
    ctx: Ctx<'_>,
    translation: &Translation,
    row_version: i64,
) -> Result<axum::response::Html<String>, WebError> {
    let request_id = ctx.request.request_id();
    let href = parameter_href(ctx.target);
    let t = ctx.texts();
    let source = RowSource {
        parameter_href: href,
        csrf_token: ctx.request.csrf_token(),
        row_version: row_version.to_string(),
        editable: true,
    };
    let row = TranslationRow {
        toast: t.with_arg("translation-saved", "lang", &translation.lang),
        ..TranslationRow::new(&source, &translation.lang, Some(translation))
    };
    let fragment = TranslationRowFragment {
        t,
        row,
        form_only: false,
    };
    render(&fragment, request_id)
}

/// Check the fields of a row, or build the error with every line and the form of the row.
fn checked_row(
    ctx: Ctx<'_>,
    lang: &LangCode,
    fields: &TranslationFields,
) -> Result<(i64, Translation), WebError> {
    check_row(lang, fields).map_err(|lines| match NonEmpty::try_from(lines) {
        Ok(lines) => refuse_row(ctx, lang, fields, lines),
        Err(_empty) => WebError::Internal {
            request_id: ctx.request.request_id(),
        },
    })
}

/// `POST .../translations/{lang}`: add or replace the translation of a tag of the list.
pub(crate) async fn upsert(
    State(state): State<WebState>,
    path: Result<Path<(String, String, String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<Form<TranslationFields>, FormRejection>,
) -> Result<Response, WebError> {
    let (target, lang) = path_translation(path)?;
    if !state.settings().translation_langs.contains(&lang) {
        return Err(WebError::BadRequest {
            field: "lang",
            reason: FieldReason::LangList,
        });
    }
    let ctx = Ctx {
        state: &state,
        request: &request,
        target: &target,
    };
    let sent = form.as_ref().ok().map(|Form(fields)| (&lang, fields));
    check_access(ctx, ParameterFormPurpose::UpsertTranslation, sent).await?;
    let Form(fields) = form?;
    let (expected_row_version, translation) = checked_row(ctx, &lang, &fields)?;
    let input = UpsertTranslationInput {
        reference: target.reference.clone(),
        number: target.number,
        expected_row_version,
        key: parameter_ref(ctx)?.key,
        translation: translation.clone(),
    };
    let result = UpsertTranslation::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    let output = match result {
        Ok(output) => output,
        Err(error) => return Err(command_failed(ctx, Some((&lang, &fields)), error).await),
    };
    if !request.is_htmx() {
        return see_other_path(&parameter_href(&target), request.request_id(), []);
    }
    let body = saved(ctx, &translation, output.row_version)?;
    success(&output, body, request.request_id())
}

/// `POST .../translations/{lang}/delete`: remove the translation of any tag of the parameter.
pub(crate) async fn remove(
    State(state): State<WebState>,
    path: Result<Path<(String, String, String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<Form<RowFields>, FormRejection>,
) -> Result<Response, WebError> {
    let (target, lang) = path_translation(path)?;
    let ctx = Ctx {
        state: &state,
        request: &request,
        target: &target,
    };
    check_access(ctx, ParameterFormPurpose::RemoveTranslation, None).await?;
    let Form(fields) = form?;
    let input = RemoveTranslationInput {
        reference: target.reference.clone(),
        number: target.number,
        expected_row_version: sent_row_version(&fields.row_version)?,
        key: parameter_ref(ctx)?.key,
        lang: lang.clone(),
    };
    let result = RemoveTranslation::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    let output = match result {
        Ok(output) => output,
        Err(error) => return Err(command_failed(ctx, None, error).await),
    };
    if !request.is_htmx() {
        return see_other_path(&parameter_href(&target), request.request_id(), []);
    }
    let toast = ctx.texts().with_arg("translation-deleted", "lang", &lang);
    success(&output, rows_body(ctx, toast).await?, request.request_id())
}
