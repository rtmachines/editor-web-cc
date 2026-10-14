//! The pages of the user administration: `/users` and its routes (`api.md` section 2.8).
//!
//! A `{login}` that is not a login gives `404 E-WEB-005` to every caller without a use case
//! (`api.md` section 1). A well-formed login goes to the use case, which checks the permission
//! first. The one-time password appears only in the answer of the command that made it.

use axum::extract::rejection::{FormRejection, PathRejection, QueryRejection};
use axum::extract::{Form, Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::error::{AppError, Missing};
use recipe_app::identity::{
    ActivateUser, CreateUser, CreateUserInput, DeactivateUser, GetUser, GetUserForm, ListUsers,
    ListUsersInput, ResetPassword, SetUserRole, SetUserRoleInput, UnlockLogin,
};
use recipe_domain::identity::{Login, Role};
use serde::Deserialize;

use super::render::render;
use super::request::PageRequest;
use super::texts::Texts;
use crate::error::{FieldReason, WebError};
use crate::state::WebState;
use fluent::FluentArgs;
use query::ListQuery;
use templates::{NewUserPage, ResultFragment, ResultPage, UserPage, UsersPage};

mod query;
mod templates;

/// The fields of the form of `POST /users`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateFields {
    login: String,
    role: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// The fields of the form of `POST /users/{login}/role`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoleFields {
    role: String,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// The fields of a form that carries only the CSRF token.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TokenFields {
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// Read the path value `{login}`. A text that is not a login gives `404 E-WEB-005`.
///
/// The function solves one problem: axum answers a path value that is not UTF-8 with its own
/// plain-text `400`. The handlers take the rejection as a value, so that one mapping gives the
/// `404` for a bad text and for a bad encoding. The answer never holds the path text.
fn path_login(path: Result<Path<String>, PathRejection>) -> Result<Login, WebError> {
    let malformed = || {
        WebError::App(AppError::NotFound {
            entity: Missing::MalformedLogin,
        })
    };
    let Path(text) = path.map_err(|_rejection| malformed())?;
    Login::parse(&text).map_err(|_reason| malformed())
}

/// Read the form field `login` of a new user.
fn field_login(text: &str) -> Result<Login, WebError> {
    Login::parse(text).map_err(|_reason| WebError::BadRequest {
        field: "login",
        reason: FieldReason::LoginGrammar,
    })
}

/// Read a form field `role`.
fn field_role(text: &str) -> Result<Role, WebError> {
    Role::parse(text).map_err(|_reason| WebError::BadRequest {
        field: "role",
        reason: FieldReason::Role,
    })
}

/// Build the address of one page of the list.
fn page_url(q: &str, page: u32) -> String {
    if q.is_empty() {
        format!("/users?page={page}")
    } else {
        format!("/users?q={q}&page={page}")
    }
}

/// `GET /users`: list the users, one page at a time.
pub(crate) async fn list(
    State(state): State<WebState>,
    request: PageRequest,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Response, WebError> {
    let Query(query) = query.map_err(|_rejection| WebError::BadRequest {
        field: "query",
        reason: FieldReason::Query,
    })?;
    let checked = query.check()?;
    let input = ListUsersInput {
        q: checked.q.clone(),
        page: checked.index,
    };
    let users = ListUsers::new(state.identity().clone())
        .execute(request.auth()?, &request.meta(), input)
        .await?;
    let q = checked.q.unwrap_or_default();
    let full_page = usize::try_from(state.identity().settings().page_users)
        .is_ok_and(|size| users.len() >= size);
    let prev = (checked.page > 1)
        .then(|| checked.page.checked_sub(1))
        .flatten()
        .map(|page| page_url(&q, page));
    let next = full_page
        .then(|| checked.page.checked_add(1))
        .flatten()
        .map(|page| page_url(&q, page));
    let texts = Texts::new(state.bundles(), request.lang());
    let page = UsersPage::new(texts, &request, &q, &users, (prev, next))?;
    Ok(render(&page, request.request_id())?.into_response())
}

/// `GET /users/new`: show the form of a new user.
pub(crate) async fn new_form(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let form = GetUserForm::new(state.identity().clone())
        .execute(request.auth()?, &request.meta())
        .await?;
    let texts = Texts::new(state.bundles(), request.lang());
    let page = NewUserPage::new(texts, &request, &form.roles)?;
    Ok(render(&page, request.request_id())?.into_response())
}

/// Build the answer of a command that made a user or a password: always a full page.
fn result_page(
    request: &PageRequest,
    texts: Texts<'_>,
    title_key: &str,
    fragment: ResultFragment<'_>,
) -> Result<Response, WebError> {
    let page = ResultPage::new(texts, request, texts.get(title_key), fragment)?;
    Ok(render(&page, request.request_id())?.into_response())
}

/// Build the answer of a command: a fragment for HTMX and a page for another request.
fn command_result(
    request: &PageRequest,
    texts: Texts<'_>,
    fragment: ResultFragment<'_>,
) -> Result<Response, WebError> {
    if request.is_htmx() {
        return Ok(render(&fragment, request.request_id())?.into_response());
    }
    result_page(request, texts, "user-title", fragment)
}

/// Format a message that names a login.
fn message(texts: Texts<'_>, key: &str, login: &Login) -> String {
    let mut args = FluentArgs::new();
    args.set("login", login.to_string());
    texts.with(key, &args)
}

/// `POST /users`: create a user and show the one-time password once.
pub(crate) async fn create(
    State(state): State<WebState>,
    request: PageRequest,
    form: Result<Form<CreateFields>, FormRejection>,
) -> Result<Response, WebError> {
    let Form(form) = form?;
    let login = field_login(&form.login)?;
    let role = field_role(&form.role)?;
    let issued = CreateUser::new(state.identity().clone())
        .execute(
            request.auth()?,
            &request.meta(),
            CreateUserInput {
                login: login.clone(),
                role,
            },
        )
        .await?;
    let texts = Texts::new(state.bundles(), request.lang());
    let fragment = ResultFragment::new(
        texts,
        message(texts, "user-created", &login),
        Some(issued.password().as_str().to_owned()),
        login.to_string(),
    );
    result_page(&request, texts, "user-created-title", fragment)
}

/// `GET /users/{login}`: show a user with the throttle state of the login.
pub(crate) async fn detail(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let login = path_login(path)?;
    let detail = GetUser::new(state.identity().clone())
        .execute(request.auth()?, &request.meta(), &login)
        .await?;
    let texts = Texts::new(state.bundles(), request.lang());
    let page = UserPage::new(texts, &request, &detail)?;
    Ok(render(&page, request.request_id())?.into_response())
}

/// `POST /users/{login}/role`: set the role and end the sessions of the user.
pub(crate) async fn set_role(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<RoleFields>, FormRejection>,
) -> Result<Response, WebError> {
    let login = path_login(path)?;
    let Form(form) = form?;
    let role = field_role(&form.role)?;
    SetUserRole::new(state.identity().clone())
        .execute(
            request.auth()?,
            &request.meta(),
            SetUserRoleInput {
                login: login.clone(),
                role,
            },
        )
        .await?;
    let texts = Texts::new(state.bundles(), request.lang());
    let mut args = FluentArgs::new();
    args.set("login", login.to_string());
    args.set("role", texts.role(role));
    let text = texts.with("user-role-changed", &args);
    let fragment = ResultFragment::new(texts, text, None, login.to_string());
    command_result(&request, texts, fragment)
}

/// `POST /users/{login}/deactivate`: deactivate the user and end the sessions.
pub(crate) async fn deactivate(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<TokenFields>, FormRejection>,
) -> Result<Response, WebError> {
    let login = path_login(path)?;
    let Form(_fields) = form?;
    DeactivateUser::new(state.identity().clone())
        .execute(request.auth()?, &request.meta(), &login)
        .await?;
    let texts = Texts::new(state.bundles(), request.lang());
    let text = message(texts, "user-deactivated", &login);
    command_result(
        &request,
        texts,
        ResultFragment::new(texts, text, None, login.to_string()),
    )
}

/// `POST /users/{login}/activate`: activate the user.
pub(crate) async fn activate(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<TokenFields>, FormRejection>,
) -> Result<Response, WebError> {
    let login = path_login(path)?;
    let Form(_fields) = form?;
    ActivateUser::new(state.identity().clone())
        .execute(request.auth()?, &request.meta(), &login)
        .await?;
    let texts = Texts::new(state.bundles(), request.lang());
    let text = message(texts, "user-activated", &login);
    command_result(
        &request,
        texts,
        ResultFragment::new(texts, text, None, login.to_string()),
    )
}

/// `POST /users/{login}/reset-password`: make a new one-time password and show it once.
pub(crate) async fn reset_password(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<TokenFields>, FormRejection>,
) -> Result<Response, WebError> {
    let login = path_login(path)?;
    let Form(_fields) = form?;
    let issued = ResetPassword::new(state.identity().clone())
        .execute(request.auth()?, &request.meta(), &login)
        .await?;
    let texts = Texts::new(state.bundles(), request.lang());
    let fragment = ResultFragment::new(
        texts,
        message(texts, "user-password-reset", &login),
        Some(issued.password().as_str().to_owned()),
        login.to_string(),
    );
    command_result(&request, texts, fragment)
}

/// `POST /users/{login}/unlock`: clear the login delay of the user.
pub(crate) async fn unlock(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
    form: Result<Form<TokenFields>, FormRejection>,
) -> Result<Response, WebError> {
    let login = path_login(path)?;
    let Form(_fields) = form?;
    UnlockLogin::new(state.identity().clone())
        .execute(request.auth()?, &request.meta(), &login, request.client())
        .await?;
    let texts = Texts::new(state.bundles(), request.lang());
    let text = message(texts, "user-unlocked", &login);
    command_result(
        &request,
        texts,
        ResultFragment::new(texts, text, None, login.to_string()),
    )
}

#[cfg(test)]
mod tests;
