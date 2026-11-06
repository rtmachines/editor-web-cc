//! The pages of the user administration: `/users` and its routes (`api.md` section 2.8).
//!
//! A `{login}` that is not a login gives `404 E-WEB-005` to every caller without a use case
//! (`api.md` section 1). A well-formed login goes to the use case, which checks the permission
//! first. The one-time password appears only in the answer of the command that made it.

use axum::extract::rejection::{FormRejection, PathRejection, QueryRejection};
use axum::extract::{Form, Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::error::{AppError, Missing, StorageError};
use recipe_app::identity::{
    ActivateUser, CreateUser, CreateUserInput, DeactivateUser, GetUser, GetUserForm, ListUsers,
    ListUsersInput, ResetPassword, SetUserRole, SetUserRoleInput, UnlockLogin, UnlockOutcome,
};
use recipe_app::ports::NonEmpty;
use recipe_domain::audit::RequestId;
use recipe_domain::identity::{Login, Role};
use serde::Deserialize;

use super::draft_reply::refused;
use super::recipes::fields::FieldLines;
use super::render::render;
use super::request::PageRequest;
use super::texts::Texts;
use crate::error::{FieldError, FieldReason, WebError, display_chain};
use crate::state::WebState;
use fluent::FluentArgs;
use query::ListQuery;
use templates::{
    NewUserForm, NewUserFormBlock, NewUserPage, ResultFragment, ResultPage, RoleFormBlock,
    UserPage, UserView, UsersPage, UsersPager,
};

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

/// Build the error of a refused form of a new user: every line and the form with the sent values.
///
/// The function solves one problem: the answer must show a line under each bad input (ADR-0041 (e)).
/// The answer keeps the sent login only when it parses. It never echoes a login that does not parse.
/// The error of `GetUserForm` wins, for example `403 E-WEB-002` for a role without the right (ADR-0013).
async fn refuse_create(
    state: &WebState,
    request: &PageRequest,
    fields: &CreateFields,
    lines: NonEmpty<FieldError>,
) -> WebError {
    let auth = match request.auth() {
        Ok(auth) => auth,
        Err(error) => return error,
    };
    let form = match GetUserForm::new(state.identity().clone())
        .execute(auth, &request.meta())
        .await
    {
        Ok(form) => form,
        Err(error) => return WebError::from(error),
    };
    let texts = Texts::new(state.bundles(), request.lang());
    refused(state, request, lines, |errors| {
        NewUserFormBlock::new(
            texts,
            NewUserForm::new(texts, &form.roles, request.csrf_token()).sent(
                &fields.login,
                &fields.role,
                errors,
            ),
        )
    })
}

/// Build the error of a refused set-role form: the line `E-WEB-008` under the select `role`.
///
/// The function solves one problem: the form must show the current role of the user and the line
/// (ADR-0041 (e)). `GetUser` checks the permission first, so its error wins.
async fn refuse_role(state: &WebState, request: &PageRequest, login: &Login) -> WebError {
    let auth = match request.auth() {
        Ok(auth) => auth,
        Err(error) => return error,
    };
    let detail = match GetUser::new(state.identity().clone())
        .execute(auth, &request.meta(), login)
        .await
    {
        Ok(detail) => detail,
        Err(error) => return WebError::from(error),
    };
    let texts = Texts::new(state.bundles(), request.lang());
    let lines = NonEmpty::new(FieldError::bad_request("role", FieldReason::Role));
    let view = match UserView::new(texts, request, &detail, false) {
        Ok(view) => view,
        Err(error) => return error,
    };
    refused(state, request, lines, |errors| {
        RoleFormBlock::new(texts, view.with_role_errors(errors))
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
    let after_end = users.is_empty() && checked.page > 1;
    let previous = (checked.page > 1 && !after_end)
        .then(|| checked.page.checked_sub(1))
        .flatten()
        .map(|page| page_url(&q, page));
    let next = full_page
        .then(|| checked.page.checked_add(1))
        .flatten()
        .map(|page| page_url(&q, page));
    let pager = UsersPager {
        previous,
        next,
        current: checked.page,
        after_end,
        first: page_url(&q, 1),
    };
    let texts = Texts::new(state.bundles(), request.lang());
    let page = UsersPage::new(texts, &request, &q, &users, pager)?;
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

/// Read the user for the out-of-band parts of an HTMX answer.
///
/// The function solves one problem: the facts and the action forms of the user page must show
/// the state that the command made (design D1). A request without HTMX gets the full result
/// page, so it runs no query. A read error goes to the `WebError` mapping. After a commit, the
/// command has taken effect, and a second try of it is harmless. `reset_password` calls the
/// function before the command, so a read error can never hide the one-time password.
async fn refreshed(
    state: &WebState,
    request: &PageRequest,
    texts: Texts<'_>,
    login: &Login,
) -> Result<Option<UserView>, WebError> {
    if !request.is_htmx() {
        return Ok(None);
    }
    let detail = GetUser::new(state.identity().clone())
        .execute(request.auth()?, &request.meta(), login)
        .await?;
    Ok(Some(UserView::new(texts, request, &detail, true)?))
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
    let mut lines = FieldLines::default();
    let login = lines.field(
        Login::parse(&form.login),
        "login",
        FieldReason::LoginGrammar,
    );
    let role = lines.field(Role::parse(&form.role), "role", FieldReason::Role);
    let (login, role) = match lines.finish(login.zip(role)) {
        Ok(checked) => checked,
        Err(lines) => return Err(refuse_create(&state, &request, &form, lines).await),
    };
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
        !issued.role().is_person(),
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
    let Ok(role) = Role::parse(&form.role) else {
        return Err(refuse_role(&state, &request, &login).await);
    };
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
    let refresh = refreshed(&state, &request, texts, &login).await?;
    let fragment =
        ResultFragment::new(texts, text, None, false, login.to_string()).with_refresh(refresh);
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
    let refresh = refreshed(&state, &request, texts, &login).await?;
    command_result(
        &request,
        texts,
        ResultFragment::new(texts, text, None, false, login.to_string()).with_refresh(refresh),
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
    let refresh = refreshed(&state, &request, texts, &login).await?;
    command_result(
        &request,
        texts,
        ResultFragment::new(texts, text, None, false, login.to_string()).with_refresh(refresh),
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
    let texts = Texts::new(state.bundles(), request.lang());
    // The read comes first: an error after the commit must never hide the one-time password.
    let before = refreshed(&state, &request, texts, &login).await?;
    let issued = ResetPassword::new(state.identity().clone())
        .execute(request.auth()?, &request.meta(), &login)
        .await?;
    // The command sets the forced change for a person and never for a technical account.
    let facts = before.map(|view| view.with_must_change(issued.role().is_person()));
    let fragment = ResultFragment::new(
        texts,
        message(texts, "user-password-reset", &login),
        Some(issued.password().as_str().to_owned()),
        !issued.role().is_person(),
        login.to_string(),
    )
    .with_facts(facts);
    command_result(&request, texts, fragment)
}

/// Write the one `WARN` line of an unlock whose security event is lost (ADR-0130).
///
/// The function solves one problem: the unlock is committed and the answer is a success, so no
/// error mapping logs the lost event. This line is the one log place of that fault.
///
/// The field `chain` holds the `Display` chain of the fault, as the line of `log_error` does.
fn log_lost_unlock_event(lost: &StorageError, request_id: RequestId) {
    tracing::warn!(
        error_code = %lost.code(),
        request_id = %request_id,
        chain = %display_chain(lost),
        event = "unlock",
        "security event lost after a committed unlock"
    );
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
    let outcome = UnlockLogin::new(state.identity().clone())
        .execute(request.auth()?, &request.meta(), &login, request.client())
        .await?;
    if let UnlockOutcome::EventLost(lost) = &outcome {
        log_lost_unlock_event(lost, request.request_id());
    }
    let texts = Texts::new(state.bundles(), request.lang());
    let text = message(texts, "user-unlocked", &login);
    let refresh = refreshed(&state, &request, texts, &login).await?;
    command_result(
        &request,
        texts,
        ResultFragment::new(texts, text, None, false, login.to_string()).with_refresh(refresh),
    )
}

#[cfg(test)]
mod actions_tests;
#[cfg(test)]
mod form_error_tests;
#[cfg(test)]
mod read_only_tests;
#[cfg(test)]
mod technical_tests;
#[cfg(test)]
mod tests;
