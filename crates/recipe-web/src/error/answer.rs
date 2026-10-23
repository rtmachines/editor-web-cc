//! The one function that maps an error to the HTTP answer (`api.md` section 5).

use askama::Template;
use axum::http::header::{LOCATION, RETRY_AFTER, SET_COOKIE};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use fluent::FluentArgs;
use recipe_app::error::AppError;
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::ExportError;
use recipe_domain::identity::IdentityError;
use recipe_domain::plant::LeafPath;
use recipe_domain::recipe::{ParameterTag, RecipeError};

use super::lists::list_view;
use super::log::{level_of, log_error};
use super::plant::plant_args;
use super::recipe::recipe_line_args;
use super::status::status_of;
use super::values::{
    field_reason_text, missing_text, operation_text, password_reason_text, task_stage_text,
};
use super::view::ConflictLines;
use super::view::{
    ErrorFragment, ErrorPage, ErrorView, FieldErrorsFragment, FieldLine, field_key, has_input,
    input_id,
};
use super::{ConflictView, ListView, RenderedForm};
use super::{Retry, RowField, WebError, item_lines, leaf_lines};
use crate::cookies::clear_session_cookie;
use crate::i18n::Lang;
use crate::layout::Layout;
use crate::pages::{Texts, login_page_with_error};
use crate::state::WebState;

/// The page that a request without a session goes to.
const LOGIN_PATH: &str = "/login";
/// The page where a user with `must_change_password` goes.
const PASSWORD_PATH: &str = "/me/password";
/// The HTMX header that sends the browser to another page.
const HX_REDIRECT: HeaderName = HeaderName::from_static("hx-redirect");
/// The HTMX header that names the element for the answer.
const HX_RETARGET: HeaderName = HeaderName::from_static("hx-retarget");
/// The HTMX header that sets the swap style of the answer.
const HX_RESWAP: HeaderName = HeaderName::from_static("hx-reswap");
/// The HTMX header that controls the address bar. The value `false` keeps the address.
const HX_PUSH_URL: HeaderName = HeaderName::from_static("hx-push-url");
/// The value of `HX-Push-Url` of an error: a refused form keeps the URL of its page (ADR-0037 z).
const KEEP_URL: &str = "false";
/// The element that shows an error to the user.
const ERROR_PANEL: &str = "#error-panel";
/// The swap style of an error. The error panel scrolls into view (ADR-0036 p, `api.md` 5.3).
const ERROR_RESWAP: &str = "innerHTML show:#error-panel:top";
/// The element of an HTMX error answer that empties `#toast` out of band, so no earlier success
/// message stands next to the error (`design-system.md` section 10.7). The values are literals.
const TOAST_CLEAR: &str = "<div id=\"toast\" hx-swap-oob=\"innerHTML\"></div>";
/// The end tag of `main`. The page of a refused form inserts the form before it.
const MAIN_END: &str = "</main>";

/// The full page of a refused form for a request without HTMX (`api.md` section 5.3).
///
/// The page has the layout of the session, the summary in `#error-panel`, and one `h1`. The
/// answer inserts the form with the sent texts after the `h1`, so no template uses `|safe`.
#[derive(Template)]
#[template(path = "refused-form-page.html")]
struct RefusedFormPage<'a> {
    /// The layout of the session.
    layout: Layout<'a>,
    /// The summary of the lines.
    view: &'a ErrorView,
}

/// What the mapping needs to know about the request.
pub(crate) struct ErrorContext<'a> {
    /// The id of the request, for the answer and the log line.
    pub(crate) request_id: RequestId,
    /// The UI language of the request.
    pub(crate) lang: Lang,
    /// True when the request has the header `HX-Request: true`.
    pub(crate) htmx: bool,
    /// The form that the handler asked to show again, if any.
    pub(crate) retry: Option<Retry>,
    /// The state with the bundles and the settings.
    pub(crate) state: &'a WebState,
}

/// Add the arguments of the identity errors that carry values.
fn identity_args(args: &mut FluentArgs<'static>, error: &IdentityError, ctx: &ErrorContext<'_>) {
    match error {
        IdentityError::LoginThrottled { retry_after } => {
            args.set("retry_after", retry_after.to_string());
        }
        IdentityError::DuplicateLogin { login }
        | IdentityError::LastAdministrator { login }
        | IdentityError::UserAlreadyActive { login }
        | IdentityError::UserAlreadyInactive { login } => args.set("login", login.to_string()),
        IdentityError::PasswordPolicy { reason } => args.set(
            "reason",
            password_reason_text(ctx.state.bundles(), ctx.lang, *reason),
        ),
        _other => {}
    }
}

/// Add the arguments of the export errors: `reference` and `number` (design D7 (d)).
fn export_args(args: &mut FluentArgs<'static>, error: &ExportError) {
    match error {
        ExportError::NoApprovedVersion { reference } => {
            args.set("reference", reference.to_string());
        }
        ExportError::ContentHashMismatch { reference, number }
        | ExportError::VersionNotExportable { reference, number } => {
            args.set("reference", reference.to_string());
            args.set("number", number.to_string());
        }
        ExportError::Serialize(_failure) => {}
    }
}

/// Return the values of the error as arguments of its `fluent` message.
pub(super) fn args_of(error: &WebError, ctx: &ErrorContext<'_>) -> FluentArgs<'static> {
    let bundles = ctx.state.bundles();
    let mut args = FluentArgs::new();
    match error {
        WebError::PayloadTooLarge { limit } => args.set("limit", limit.to_string()),
        WebError::BadRequest { field, reason } | WebError::BadQuery { field, reason } => {
            args.set("field", label_of(field, ctx));
            args.set("reason", field_reason_text(bundles, ctx.lang, *reason));
        }
        WebError::BadRow { row, field, reason } => {
            args.set("field", row_label_of(*row, *field, ctx));
            args.set("reason", field_reason_text(bundles, ctx.lang, *reason));
        }
        WebError::App(app) => app_args(&mut args, app, ctx),
        WebError::Conflict { error: app, .. } => app_args(&mut args, app, ctx),
        _other => {}
    }
    args
}

/// Add the arguments of an error of a use case.
fn app_args(args: &mut FluentArgs<'static>, error: &AppError, ctx: &ErrorContext<'_>) {
    let bundles = ctx.state.bundles();
    match error {
        AppError::Forbidden { operation } => {
            args.set("operation", operation_text(bundles, ctx.lang, *operation));
        }
        AppError::NotFound { entity } => {
            args.set("entity", missing_text(bundles, ctx.lang, entity));
        }
        AppError::Timeout { stage } => args.set("stage", stage.to_string()),
        AppError::TaskFailed { stage } => {
            args.set("stage", task_stage_text(bundles, ctx.lang, *stage));
        }
        AppError::Identity(identity) => identity_args(args, identity, ctx),
        AppError::Export(export) => export_args(args, export),
        AppError::Plant(plant) => plant_args(args, plant, bundles, ctx.lang),
        AppError::ConcurrentEdit {
            reference, number, ..
        } => {
            args.set("reference", reference.to_string());
            args.set("number", number.to_string());
        }
        AppError::Recipe(recipe) => {
            for (name, arg) in recipe_line_args(recipe) {
                args.set(name, arg.text(bundles, ctx.lang));
            }
        }
        _other => {}
    }
}

/// Return the label of the form field `field` in the UI language, or the name without a label.
fn label_of(field: &str, ctx: &ErrorContext<'_>) -> String {
    ctx.state
        .bundles()
        .message(ctx.lang, &field_key(field), None)
        .unwrap_or_else(|| field.to_owned())
}

/// Return the label of one input of a member row, for example "Name of member 3".
///
/// The label takes the number of the row, which counts from 1. The index of the row counts from 0.
fn row_label_of(row: u16, field: RowField, ctx: &ErrorContext<'_>) -> String {
    let texts = Texts::new(ctx.state.bundles(), ctx.lang);
    texts.with_arg(
        field.label_key(),
        "number",
        u32::from(row).saturating_add(1),
    )
}

/// Return the `Retry-After` seconds of an answer, if it needs the header.
pub(super) fn retry_after_of(
    error: &WebError,
    status: StatusCode,
    ctx: &ErrorContext<'_>,
) -> Option<u64> {
    if let WebError::App(AppError::Identity(IdentityError::LoginThrottled { retry_after })) = error
    {
        return Some(*retry_after);
    }
    (status == StatusCode::SERVICE_UNAVAILABLE).then(|| ctx.state.settings().retry_after.as_secs())
}

/// Return the sentence of the code `shown` in the UI language, with the arguments `args`.
///
/// The sentence holds no reference. When the bundle has no text for the code or the arguments,
/// the sentence is the English `Display` text of the error.
pub(super) fn sentence_of(
    error: &WebError,
    shown: ErrorCode,
    args: &FluentArgs<'_>,
    ctx: &ErrorContext<'_>,
) -> String {
    ctx.state
        .bundles()
        .message(ctx.lang, &format!("error-{shown}"), Some(args))
        .unwrap_or_else(|| error.to_string())
}

/// Build the view of an error: the code, one sentence, and the reference.
fn view_of(error: &WebError, shown: ErrorCode, ctx: &ErrorContext<'_>) -> ErrorView {
    let bundles = ctx.state.bundles();
    let mut args = args_of(error, ctx);
    let list = list_of(error, ctx);
    // A summary shows no sentence of its own, so a list never builds its long English text.
    let message = if list.is_some() {
        String::new()
    } else {
        sentence_of(error, shown, &args, ctx)
    };
    args.set("request_id", ctx.request_id.to_string());
    let reference = bundles
        .message(ctx.lang, "error-reference", Some(&args))
        .unwrap_or_else(|| format!("Reference: {}.", ctx.request_id));
    ErrorView {
        code: shown.as_str(),
        message,
        reference,
        lines: lines_of(error, ctx),
        link_fields: ctx.htmx
            || matches!(ctx.retry, Some(Retry::Login(_)))
            || page_form(error, ctx).is_some(),
        list,
        conflict: conflict_of(error, ctx),
    }
}

/// Build the lines of `conflict.html` for a conflict on a draft (design D10).
///
/// The `row_version` comes from the read of the version. When the read failed, it comes from the
/// error, and the answer shows no login and no time.
fn conflict_of(error: &WebError, ctx: &ErrorContext<'_>) -> Option<ConflictLines> {
    let WebError::Conflict { error: app, view } = error else {
        return None;
    };
    let AppError::ConcurrentEdit {
        reference,
        number,
        actual,
        ..
    } = app.as_ref()
    else {
        return None;
    };
    let t = Texts::new(ctx.state.bundles(), ctx.lang);
    let (row_version, last_change) = match view {
        ConflictView::Read {
            row_version,
            last_change,
        } => (*row_version, last_change.as_deref()),
        ConflictView::Unread(_failure) => (*actual, None),
    };
    let last_change = last_change.map_or_else(
        || t.get("conflict-no-editor"),
        |(login, time)| {
            let mut args = FluentArgs::new();
            args.set("login", login.to_string());
            args.set("time", time.clone());
            t.with("conflict-last-change", &args)
        },
    );
    Some(ConflictLines {
        row_version: t.with_arg("conflict-row-version", "row_version", row_version),
        last_change,
        warning: t.get("conflict-reload-warning"),
        reload: t.get("conflict-reload"),
        href: format!("/recipes/{reference}/versions/{number}"),
    })
}

/// Build the summary of an error with a list: the lines of a form, the entries of a list of
/// recipe errors, or the items of `E-RCP-020` (ADR-0027, ADR-0041).
fn list_of(error: &WebError, ctx: &ErrorContext<'_>) -> Option<ListView> {
    let bundles = ctx.state.bundles();
    let no_input = |_path: &LeafPath| None;
    let no_row = |_tag: &ParameterTag| None;
    match error {
        WebError::Fields { lines, .. } => Some(list_view(lines.iter(), bundles, ctx.lang)),
        WebError::App(AppError::Recipes(errors)) => {
            Some(list_view(&leaf_lines(errors, &no_input), bundles, ctx.lang))
        }
        WebError::App(AppError::Recipe(RecipeError::ValidationFailed { items })) => {
            Some(list_view(&item_lines(items, &no_row), bundles, ctx.lang))
        }
        _other => None,
    }
}

/// Build the field lines of an error. An error of the whole form has none.
///
/// The field of a line is the `id` of the input that the line links to.
fn lines_of(error: &WebError, ctx: &ErrorContext<'_>) -> Vec<FieldLine> {
    match error {
        WebError::BadRequest { field, .. } => vec![FieldLine {
            field: input_id(field).to_owned(),
            label: label_of(field, ctx),
            has_input: has_input(field),
        }],
        WebError::BadQuery { field, .. } => vec![FieldLine {
            field: (*field).to_owned(),
            label: label_of(field, ctx),
            has_input: false,
        }],
        WebError::BadRow { row, field, .. } => vec![FieldLine {
            field: format!("members-{row}-{}", field.id_suffix()),
            label: row_label_of(*row, *field, ctx),
            has_input: true,
        }],
        _other => Vec::new(),
    }
}

/// Render the body: a fragment for an HTMX request and a full page otherwise.
///
/// A handler can ask for its form instead of the error page. The page then shows the form with
/// the error on top.
fn render(view: &ErrorView, field_errors: bool, ctx: &ErrorContext<'_>) -> Option<String> {
    if ctx.htmx {
        if field_errors {
            FieldErrorsFragment { view }.render().ok()
        } else {
            ErrorFragment { view }.render().ok()
        }
    } else if let Some(Retry::Login(pre_session)) = &ctx.retry {
        let texts = Texts::new(ctx.state.bundles(), ctx.lang);
        login_page_with_error(texts, ctx.lang, pre_session.nonce(), view)
            .render()
            .ok()
    } else {
        let texts = Texts::new(ctx.state.bundles(), ctx.lang);
        let title = format!("{} · {}", view.code, texts.get("error-page-title"));
        ErrorPage {
            layout: Layout::bare(texts, ctx.lang, title),
            field_errors,
            view,
        }
        .render()
        .ok()
    }
}

/// Return the form of a refused request without HTMX that has the session parts of the layout.
fn page_form<'e>(error: &'e WebError, ctx: &ErrorContext<'_>) -> Option<&'e RenderedForm> {
    match error {
        WebError::Fields {
            form: Some(form), ..
        } if !ctx.htmx && form.frame().is_some() => Some(form),
        _other => None,
    }
}

/// Render the full page of a refused form: the session layout, the summary, and the form.
///
/// The form keeps the sent texts and the lines under its inputs (ADR-0041 (e)). It loses every
/// `hx-*` attribute, because the page is not a swap and has no target of the page that sent the
/// form. A second save is a normal `POST` (ADR-0047 (a)). Askama escapes each value of the page,
/// so the literal `</main>` of the layout is the last match.
fn refused_page(view: &ErrorView, form: &RenderedForm, ctx: &ErrorContext<'_>) -> Option<String> {
    let frame = form.frame()?;
    let texts = Texts::new(ctx.state.bundles(), ctx.lang);
    let title = format!("{} · {}", view.code, texts.get("error-page-title"));
    let page = RefusedFormPage {
        layout: Layout::refused(texts, ctx.lang, title, frame),
        view,
    }
    .render()
    .ok()?;
    let (head, tail) = page.rsplit_once(MAIN_END)?;
    Some(format!("{head}{}{MAIN_END}{tail}", form.page_html()))
}

/// Render the body of an error answer.
///
/// An HTMX answer of `WebError::Fields` with a form appends the form after the fragment. The
/// form swaps out of band (ADR-0041 (e)). Every HTMX answer also empties `#toast` out of band.
/// A full page shows the form only when the handler gave the session parts of the layout.
/// `field-errors.html` also renders the lines of a conflict (design D10).
fn body_of(
    error: &WebError,
    shown: ErrorCode,
    status: StatusCode,
    ctx: &ErrorContext<'_>,
) -> String {
    let own_fragment = matches!(error, WebError::Fields { .. } | WebError::Conflict { .. });
    let field_errors =
        own_fragment || status == StatusCode::UNPROCESSABLE_ENTITY || shown == ErrorCode::IDN_003;
    let view = view_of(error, shown, ctx);
    if let Some(page) = page_form(error, ctx).and_then(|form| refused_page(&view, form, ctx)) {
        return page;
    }
    let mut body = render(&view, field_errors, ctx)
        .unwrap_or_else(|| format!("{} {}", view.code, view.reference));
    if ctx.htmx {
        if let WebError::Fields {
            form: Some(form), ..
        } = error
        {
            body.push_str(form.as_str());
        }
        body.push_str(TOAST_CLEAR);
    }
    body
}

/// Build a redirect answer.
fn redirect(path: &'static str) -> Response {
    let mut response = StatusCode::SEE_OTHER.into_response();
    response
        .headers_mut()
        .insert(LOCATION, HeaderValue::from_static(path));
    response
}

/// Write the one log line of an error. Return the code that the answer shows and its status.
///
/// The status comes from the column HTTP of `reliability.md` section 1.2. `E-IDN-004` shows as
/// `E-IDN-003`, and the log line keeps its own code (A42). The HTML answer and the JSON answer
/// both call this function, so each error gets one log line (`reliability.md` section 1.1).
pub(super) fn logged(error: &WebError, ctx: &ErrorContext<'_>) -> (ErrorCode, StatusCode) {
    let code = error.code();
    let shown = if code == ErrorCode::IDN_004 {
        ErrorCode::IDN_003
    } else {
        code
    };
    let status = status_of(shown);
    log_error(error, code, level_of(error, code, status), ctx.request_id);
    (shown, status)
}

/// Return the redirect of a session code, if the code has one.
///
/// `E-IDN-007` is the redirect to `/me/password`. `E-WEB-004` redirects to `/login` and ends the
/// session cookie with `Max-Age=0`, but only when `htmx` is false.
pub(super) fn session_redirect(shown: ErrorCode, htmx: bool) -> Option<Response> {
    if shown == ErrorCode::IDN_007 {
        return Some(redirect(PASSWORD_PATH));
    }
    if shown != ErrorCode::WEB_004 || htmx {
        return None;
    }
    let mut response = redirect(LOGIN_PATH);
    response
        .headers_mut()
        .append(SET_COOKIE, clear_session_cookie());
    Some(response)
}

/// Map an error to the HTTP answer and write its one log line.
///
/// `E-IDN-007` and `E-WEB-004` without HTMX are redirects (`session_redirect`). For an HTMX
/// request every status of 400 or more, except `E-WEB-004`, retargets the answer to `#error-panel`
/// (`api.md` section 5.3). The same answer sets `HX-Push-Url: false`, so the URL stays.
pub(crate) fn answer(error: &WebError, ctx: &ErrorContext<'_>) -> Response {
    let (shown, status) = logged(error, ctx);
    if let Some(response) = session_redirect(shown, ctx.htmx) {
        return response;
    }
    let body = body_of(error, shown, status, ctx);
    let mut response = (status, Html(body)).into_response();
    let headers = response.headers_mut();
    if shown == ErrorCode::WEB_004 {
        headers.insert(HX_REDIRECT, HeaderValue::from_static(LOGIN_PATH));
        headers.append(SET_COOKIE, clear_session_cookie());
    } else if ctx.htmx {
        headers.insert(HX_RETARGET, HeaderValue::from_static(ERROR_PANEL));
        headers.insert(HX_RESWAP, HeaderValue::from_static(ERROR_RESWAP));
        headers.insert(HX_PUSH_URL, HeaderValue::from_static(KEEP_URL));
    }
    if let (Some(Retry::Login(pre_session)), false) = (&ctx.retry, ctx.htmx) {
        headers.append(SET_COOKIE, pre_session.set_cookie().clone());
    }
    if let Some(seconds) = retry_after_of(error, status, ctx) {
        headers.insert(RETRY_AFTER, HeaderValue::from(seconds));
    }
    response
}
