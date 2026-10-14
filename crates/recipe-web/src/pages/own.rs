//! The pages of the own account: `/login`, `/logout`, `/me`, and `/me/password` (`api.md` 2.1, 2.8).

use std::sync::Arc;

use axum::extract::rejection::FormRejection;
use axum::extract::{Form, State};
use axum::http::{HeaderName, HeaderValue};
use axum::response::{IntoResponse, Response};
use recipe_app::error::AppError;
use recipe_app::identity::{
    ChangeOwnPassword, ChangeOwnPasswordInput, GetLoginForm, GetOwnPasswordForm, GetOwnProfile,
    Login, LoginInput, Logout,
};
use recipe_domain::identity::{IdentityError, Password, TypedLogin, TypedLoginReason};
use serde::Deserialize;

use super::render::{render, see_other};
use super::request::PageRequest;
use super::texts::Texts;
use crate::cookies::{clear_session_cookie, session_cookie};
use crate::csrf::issue_pre_session;
use crate::error::{FieldReason, Retry, WebError};
use crate::state::WebState;

mod templates;

use templates::{LoginPage, PasswordChanged, PasswordPage, ProfilePage};

pub(crate) use templates::login_page_with_error;

/// The page after a login, and after a password change without HTMX.
const AFTER_LOGIN: &str = "/recipes";
/// The page of a user who must change the password.
const PASSWORD_PAGE: &str = "/me/password";
/// The page of a user without a session.
const LOGIN_PAGE: &str = "/login";
/// The HTMX header that reloads the page.
const HX_REFRESH: HeaderName = HeaderName::from_static("hx-refresh");

/// The fields of the form of `POST /login`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LoginFields {
    login: String,
    password: String,
    /// The pre-session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_pre")]
    _csrf_pre: Option<String>,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// The fields of the form of `POST /me/password`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PasswordFields {
    current_password: String,
    password: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// Turn a session token into the `Set-Cookie` value, or `E-WEB-010`.
fn session_header(
    request: &PageRequest,
    token: &recipe_domain::identity::SessionToken,
) -> Result<HeaderValue, WebError> {
    session_cookie(token).map_err(|_invalid| WebError::Internal {
        request_id: request.request_id(),
    })
}

/// `GET /login`: show the login form and set the pre-session cookie.
pub(crate) async fn login_form(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<Response, WebError> {
    GetLoginForm::new().execute()?;
    let pre_session = issue_pre_session(state.csrf_keys())?;
    let texts = Texts::new(state.bundles(), request.lang());
    let page = LoginPage::new(texts, request.lang(), pre_session.nonce(), None);
    let mut response = render(&page, request.request_id())?.into_response();
    response.headers_mut().append(
        axum::http::header::SET_COOKIE,
        pre_session.set_cookie().clone(),
    );
    Ok(response)
}

/// `POST /login`: start a session. A failure shows the login form again.
pub(crate) async fn login_submit(
    State(state): State<WebState>,
    request: PageRequest,
    form: Result<Form<LoginFields>, FormRejection>,
) -> Response {
    match start_session(&state, &request, form).await {
        Ok(response) => response,
        Err(error) => retry_login(&state, error),
    }
}

/// Run `Login` for the form and answer with the redirect and the session cookie.
async fn start_session(
    state: &WebState,
    request: &PageRequest,
    form: Result<Form<LoginFields>, FormRejection>,
) -> Result<Response, WebError> {
    let Form(form) = form?;
    let login = TypedLogin::parse(&form.login).map_err(|reason| WebError::BadRequest {
        field: "login",
        reason: match reason {
            TypedLoginReason::Length => FieldReason::LoginLength,
            TypedLoginReason::Control => FieldReason::LoginControl,
        },
    })?;
    let password = Password::parse(&form.password).map_err(|_reason| WebError::BadRequest {
        field: "password",
        reason: FieldReason::PasswordLength,
    })?;
    let input = LoginInput {
        login,
        password,
        client_ip: request.client(),
        old_session: request.cookie_session().cloned(),
    };
    let outcome = Login::new(state.identity().clone())
        .execute(&request.meta(), input)
        .await?;
    let cookie = session_header(request, outcome.token())?;
    let location = if outcome.must_change_password() {
        PASSWORD_PAGE
    } else {
        AFTER_LOGIN
    };
    Ok(see_other(location, [cookie]))
}

/// Store a fresh pre-session token next to the error, so the answer shows the form again.
fn retry_login(state: &WebState, error: WebError) -> Response {
    match issue_pre_session(state.csrf_keys()) {
        Ok(pre_session) => {
            let mut response = error.into_response();
            response
                .extensions_mut()
                .insert(Retry::Login(Arc::new(pre_session)));
            response
        }
        Err(failed) => WebError::from(failed).into_response(),
    }
}

/// `POST /logout`: end the session and clear the cookie.
pub(crate) async fn logout(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let auth = request.auth()?;
    Logout::new(state.identity().clone())
        .execute(auth, &request.meta())
        .await?;
    Ok(see_other(LOGIN_PAGE, [clear_session_cookie()]))
}

/// `GET /me`: show the own profile.
pub(crate) async fn profile(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let auth = request.auth()?;
    let own = GetOwnProfile::new(state.identity().clone())
        .execute(auth, &request.meta())
        .await?;
    let texts = Texts::new(state.bundles(), request.lang());
    let page = ProfilePage::new(texts, &request, &own)?;
    Ok(render(&page, request.request_id())?.into_response())
}

/// `GET /me/password`: show the form that changes the own password.
pub(crate) async fn password_form(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let auth = request.auth()?;
    let form = GetOwnPasswordForm::new(state.identity().clone())
        .execute(auth, &request.meta())
        .await?;
    let texts = Texts::new(state.bundles(), request.lang());
    let page = PasswordPage::new(texts, &request, form)?;
    Ok(render(&page, request.request_id())?.into_response())
}

/// `POST /me/password`: change the password and replace the session.
///
/// An HTMX request gets `200` with `HX-Refresh: true`, so the browser loads the page again with
/// the new CSRF token. Another request gets `303` to `/recipes`. Both carry the new cookie.
pub(crate) async fn change_password(
    State(state): State<WebState>,
    request: PageRequest,
    form: Result<Form<PasswordFields>, FormRejection>,
) -> Result<Response, WebError> {
    let auth = request.auth()?;
    let Form(form) = form?;
    let current_password =
        Password::parse(&form.current_password).map_err(|_reason| WebError::BadRequest {
            field: "current_password",
            reason: FieldReason::PasswordLength,
        })?;
    let new_password = Password::parse(&form.password)
        .map_err(|reason| WebError::App(AppError::Identity(IdentityError::from(reason))))?;
    let input = ChangeOwnPasswordInput {
        client_ip: request.client(),
        current_password,
        new_password,
    };
    let issued = ChangeOwnPassword::new(state.identity().clone())
        .execute(auth, &request.meta(), input)
        .await?;
    let cookie = session_header(&request, issued.token())?;
    if !request.is_htmx() {
        return Ok(see_other(AFTER_LOGIN, [cookie]));
    }
    let texts = Texts::new(state.bundles(), request.lang());
    let mut response = render(&PasswordChanged::new(texts), request.request_id())?.into_response();
    let headers = response.headers_mut();
    headers.insert(HX_REFRESH, HeaderValue::from_static("true"));
    headers.append(axum::http::header::SET_COOKIE, cookie);
    Ok(response)
}

#[cfg(test)]
mod tests;
