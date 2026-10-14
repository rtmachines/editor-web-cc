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
use super::status::status_of;
use super::view::{ErrorFragment, ErrorPage, ErrorView, FieldErrorsFragment, FieldLine};
use super::{Retry, WebError};
use crate::cookies::clear_session_cookie;
use crate::i18n::Lang;
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
/// The element that shows an error to the user.
const ERROR_PANEL: &str = "#error-panel";

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
fn identity_args(args: &mut FluentArgs<'static>, error: &IdentityError) {
    match error {
        IdentityError::LoginThrottled { retry_after } => {
            args.set("retry_after", retry_after.to_string());
        }
        IdentityError::DuplicateLogin { login }
        | IdentityError::LastAdministrator { login }
        | IdentityError::UserAlreadyActive { login }
        | IdentityError::UserAlreadyInactive { login } => args.set("login", login.to_string()),
        IdentityError::PasswordPolicy { reason } => args.set("reason", reason.to_string()),
        _other => {}
    }
}

/// Return the values of the error as arguments of its `fluent` message.
fn args_of(error: &WebError) -> FluentArgs<'static> {
    let mut args = FluentArgs::new();
    match error {
        WebError::PayloadTooLarge { limit } => args.set("limit", limit.to_string()),
        WebError::BadRequest { field, reason } => {
            args.set("field", *field);
            args.set("reason", *reason);
        }
        WebError::App(AppError::Forbidden { operation }) => args.set("operation", operation.name()),
        WebError::App(AppError::NotFound { entity }) => args.set("entity", entity.to_string()),
        WebError::App(AppError::Timeout { stage }) => args.set("stage", stage.to_string()),
        WebError::App(AppError::TaskFailed { stage }) => args.set("stage", stage.to_string()),
        WebError::App(AppError::Identity(identity)) => identity_args(&mut args, identity),
        _other => {}
    }
    args
}

/// Return the form field that the error names, or an empty text.
const fn field_of(error: &WebError) -> &'static str {
    match error {
        WebError::BadRequest { field, .. } => field,
        _other => "",
    }
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
    let mut args = args_of(error);
    let message = bundles
        .message(ctx.lang, &format!("error-{shown}"), Some(&args))
        .unwrap_or_else(|| error.to_string());
    args.set("request_id", ctx.request_id.to_string());
    let reference = bundles
        .message(ctx.lang, "error-reference", Some(&args))
        .unwrap_or_else(|| format!("Reference: {}.", ctx.request_id));
    let lines = vec![FieldLine {
        field: field_of(error).to_owned(),
        message: message.clone(),
    }];
    ErrorView {
        code: shown.as_str(),
        message,
        reference,
        lines,
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
        ErrorPage {
            lang: ctx.lang,
            csrf_token: "",
            title: view.code,
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
/// `E-WEB-004` redirects to `/login` and ends the session cookie with `Max-Age=0`. For an HTMX request every status of 400 or more, except
/// `E-WEB-004`, retargets the answer to `#error-panel` (`api.md` section 5.3).
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
        headers.insert(HX_RESWAP, HeaderValue::from_static("innerHTML"));
    }
    if let (Some(Retry::Login(pre_session)), false) = (&ctx.retry, ctx.htmx) {
        headers.append(SET_COOKIE, pre_session.set_cookie().clone());
    }
    if let Some(seconds) = retry_after_of(error, status, ctx) {
        headers.insert(RETRY_AFTER, HeaderValue::from(seconds));
    }
    response
}
