//! The one function that maps an error to the HTTP answer (`api.md` section 5).

use askama::Template;
use axum::http::header::{LOCATION, RETRY_AFTER, SET_COOKIE};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use fluent::FluentArgs;
use recipe_app::error::AppError;
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::IdentityError;

use super::log::{level_of, log_error};
use super::plant::plant_args;
use super::status::status_of;
use super::values::{
    field_reason_text, missing_text, operation_text, password_reason_text, task_stage_text,
};
use super::view::{
    ErrorFragment, ErrorPage, ErrorView, FieldErrorsFragment, FieldLine, field_key, has_input,
    input_id,
};
use super::{Retry, RowField, WebError};
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

/// Return the values of the error as arguments of its `fluent` message.
fn args_of(error: &WebError, ctx: &ErrorContext<'_>) -> FluentArgs<'static> {
    let bundles = ctx.state.bundles();
    let mut args = FluentArgs::new();
    match error {
        WebError::PayloadTooLarge { limit } => args.set("limit", limit.to_string()),
        WebError::BadRequest { field, reason } => {
            args.set("field", label_of(field, ctx));
            args.set("reason", field_reason_text(bundles, ctx.lang, *reason));
        }
        WebError::BadRow { row, field, reason } => {
            args.set("field", row_label_of(*row, *field, ctx));
            args.set("reason", field_reason_text(bundles, ctx.lang, *reason));
        }
        WebError::App(AppError::Forbidden { operation }) => {
            args.set("operation", operation_text(bundles, ctx.lang, *operation));
        }
        WebError::App(AppError::NotFound { entity }) => {
            args.set("entity", missing_text(bundles, ctx.lang, entity));
        }
        WebError::App(AppError::Timeout { stage }) => args.set("stage", stage.to_string()),
        WebError::App(AppError::TaskFailed { stage }) => {
            args.set("stage", task_stage_text(bundles, ctx.lang, *stage));
        }
        WebError::App(AppError::Identity(identity)) => identity_args(&mut args, identity, ctx),
        WebError::App(AppError::Plant(plant)) => plant_args(&mut args, plant, bundles, ctx.lang),
        _other => {}
    }
    args
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
fn retry_after_of(error: &WebError, status: StatusCode, ctx: &ErrorContext<'_>) -> Option<u64> {
    if let WebError::App(AppError::Identity(IdentityError::LoginThrottled { retry_after })) = error
    {
        return Some(*retry_after);
    }
    (status == StatusCode::SERVICE_UNAVAILABLE).then(|| ctx.state.settings().retry_after.as_secs())
}

/// Build the view of an error: the code, one sentence, and the reference.
fn view_of(error: &WebError, shown: ErrorCode, ctx: &ErrorContext<'_>) -> ErrorView {
    let bundles = ctx.state.bundles();
    let mut args = args_of(error, ctx);
    let message = bundles
        .message(ctx.lang, &format!("error-{shown}"), Some(&args))
        .unwrap_or_else(|| error.to_string());
    args.set("request_id", ctx.request_id.to_string());
    let reference = bundles
        .message(ctx.lang, "error-reference", Some(&args))
        .unwrap_or_else(|| format!("Reference: {}.", ctx.request_id));
    ErrorView {
        code: shown.as_str(),
        message,
        reference,
        lines: lines_of(error, ctx),
        link_fields: ctx.htmx || matches!(ctx.retry, Some(Retry::Login(_))),
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

/// Build a redirect answer.
fn redirect(path: &'static str) -> Response {
    let mut response = StatusCode::SEE_OTHER.into_response();
    response
        .headers_mut()
        .insert(LOCATION, HeaderValue::from_static(path));
    response
}

/// Map an error to the HTTP answer and write its one log line.
///
/// The status comes from the column HTTP of `reliability.md` section 1.2. `E-IDN-004` renders as
/// `E-IDN-003` and logs its own code (A42). `E-IDN-007` is the redirect to `/me/password`.
/// `E-WEB-004` redirects to `/login` and ends the session cookie with `Max-Age=0`. For an HTMX
/// request every status of 400 or more, except `E-WEB-004`, retargets the answer to `#error-panel`
/// (`api.md` section 5.3). The same answer sets `HX-Push-Url: false`, so the URL stays.
pub(crate) fn answer(error: &WebError, ctx: &ErrorContext<'_>) -> Response {
    let code = error.code();
    let shown = if code == ErrorCode::IDN_004 {
        ErrorCode::IDN_003
    } else {
        code
    };
    let status = status_of(shown);
    log_error(error, code, level_of(error, code, status), ctx.request_id);

    if shown == ErrorCode::IDN_007 {
        return redirect(PASSWORD_PATH);
    }
    if shown == ErrorCode::WEB_004 && !ctx.htmx {
        let mut response = redirect(LOGIN_PATH);
        response
            .headers_mut()
            .append(SET_COOKIE, clear_session_cookie());
        return response;
    }
    let field_errors = status == StatusCode::UNPROCESSABLE_ENTITY || shown == ErrorCode::IDN_003;
    let view = view_of(error, shown, ctx);
    let body = render(&view, field_errors, ctx)
        .unwrap_or_else(|| format!("{} {}", view.code, view.reference));
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
