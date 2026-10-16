//! The shared parts of the endpoint-by-role matrix (design D12, task 4.3). This module is test code.
//!
//! The module solves one problem: the matrix of the routes is no longer one list. `routes::tests`
//! holds the rows of step P3.1, and the test module of each area (machines, units, types) holds the
//! rows of its own routes. All of them need the same principals, answers, placeholders, and sender,
//! so the parts live here and each test module imports them.

use axum::http::{Method, StatusCode};
use recipe_domain::identity::Role;

pub(crate) use crate::pages::test_helpers::assert_error;
use crate::test_support::plant::M12_ID;
use crate::test_support::recipes::LINE_KEY;
use crate::test_support::{
    Fixture, Reply, TestSession, deactivate_user, get_with, post_fields, post_pre_session,
};

/// The password of every seeded account.
const PASSWORD: &str = "correct horse battery";
/// The password of `PASSWORD` as a form value.
const PASSWORD_FIELD: &str = "correct+horse+battery";
/// The login that the placeholder `{login}` names. It is the ProcessEngineer.
const LOGIN_VALUE: &str = "user2";
/// The machine code that the placeholder `{code}` names. It is the live machine `M7`.
const CODE_VALUE: &str = "M7";
/// The unit path that the placeholder `{path}` names. It is a live unit of `M7` with children.
const PATH_VALUE: &str = "U1.2";
/// The struct name that the placeholder `{name}` names. It is a struct of `M7` in three revisions.
const NAME_VALUE: &str = "ST_Zone";
/// The reference number that the placeholder `{ref}` names. It is the recipe `R-100` of `M7`.
const REF_VALUE: &str = "R-100";
/// The version number that the placeholder `{n}` names. It is the live draft of `R-100`.
const N_VALUE: &str = "3";
/// The language code that the placeholder `{lang}` names. It is the translation of `LINE`.
const LANG_VALUE: &str = "pl";

/// The six principals of design D19.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Principal {
    /// A request without a session cookie.
    NoSession,
    /// A session of an Administrator.
    Administrator,
    /// A session of a ProcessEngineer.
    ProcessEngineer,
    /// A session of an Operator.
    Operator,
    /// A session of a user with `must_change_password`.
    MustChange,
    /// The session of a user who was deactivated after the session started.
    Deactivated,
}

/// The principals in the order of the columns of the matrix.
pub(crate) const PRINCIPALS: [Principal; 6] = [
    Principal::NoSession,
    Principal::Administrator,
    Principal::ProcessEngineer,
    Principal::Operator,
    Principal::MustChange,
    Principal::Deactivated,
];

/// The expected answer of one cell of the matrix.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Want {
    /// `200` with the page or the fragment.
    Page,
    /// `303` to `/login`. A page request without a valid session gets it (`E-WEB-004`).
    Login,
    /// `303` to `/me/password`. A user with `must_change_password` gets it (`E-IDN-007`).
    Password,
    /// `303` to `/recipes`. A command without `HX-Request` gets it after success.
    Recipes,
    /// `303` to this path. A command of the machine, unit, or type pages gets it after success.
    Redirect(&'static str),
    /// `401` with the code in the body.
    Unauthorized(&'static str),
    /// `403` with `E-WEB-002` in the body. The role has no permission.
    Forbidden,
    /// `409` with the code in the body.
    Conflict(&'static str),
    /// This status with the code in the body.
    Status(StatusCode, &'static str),
}

/// One endpoint of the matrix with a form body that the endpoint accepts.
///
/// The path and the body may hold the placeholders of [`fill`].
pub(crate) struct Endpoint {
    /// The method of the request.
    pub(crate) method: Method,
    /// The path pattern of the route, for example `/machines/{code}`.
    pub(crate) path: &'static str,
    /// The form body of a `POST`.
    pub(crate) body: &'static str,
}

/// Build one endpoint of the matrix.
pub(crate) fn endpoint(method: Method, path: &'static str, body: &'static str) -> Endpoint {
    Endpoint { method, path, body }
}

/// Replace the placeholders of a path or a body with the values of the world.
///
/// - `{login}` is `user2`.
/// - `{code}` is `M7`.
/// - `{path}` is `U1.2`.
/// - `{name}` is `ST_Zone`.
/// - `{uuid}` is the fixed identifier of the archived machine `M12`.
/// - `{ref}` is `R-100`.
/// - `{n}` is `3`, the live draft of `R-100`.
/// - `{key}` is the key of the parameter `LINE`.
/// - `{lang}` is `pl`.
pub(crate) fn fill(template: &str) -> String {
    template
        .replace("{login}", LOGIN_VALUE)
        .replace("{code}", CODE_VALUE)
        .replace("{path}", PATH_VALUE)
        .replace("{name}", NAME_VALUE)
        .replace("{uuid}", &M12_ID.to_string())
        .replace("{ref}", REF_VALUE)
        .replace("{n}", N_VALUE)
        .replace("{key}", &LINE_KEY.to_string())
        .replace("{lang}", LANG_VALUE)
}

/// The login of the account that a principal uses. `NoSession` logs in as the Administrator.
fn login_of(principal: Principal) -> &'static str {
    match principal {
        Principal::NoSession | Principal::Administrator => "user1",
        Principal::ProcessEngineer => "user2",
        Principal::Operator => "user3",
        Principal::MustChange => "user4",
        Principal::Deactivated => "user5",
    }
}

/// Build a fixture with five accounts that share `PASSWORD`, the plant data of design D12, and the
/// session of `principal`.
pub(crate) async fn world(principal: Principal) -> (Fixture, Option<TestSession>) {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Administrator, PASSWORD, false);
    fixture.seed_account(2, Role::ProcessEngineer, PASSWORD, false);
    fixture.seed_account(3, Role::Operator, PASSWORD, false);
    fixture.seed_account(4, Role::Administrator, PASSWORD, true);
    fixture.seed_account(5, Role::Administrator, PASSWORD, false);
    fixture.seed_recipes();
    let session = match principal {
        Principal::NoSession => None,
        Principal::Administrator => Some(fixture.session_of(1)),
        Principal::ProcessEngineer => Some(fixture.session_of(2)),
        Principal::Operator => Some(fixture.session_of(3)),
        Principal::MustChange => Some(fixture.session_of(4)),
        Principal::Deactivated => {
            let session = fixture.session_of(5);
            deactivate_user(&fixture, "user5").await;
            Some(session)
        }
    };
    (fixture, session)
}

/// Send the request of `endpoint` as `principal` and return the answer.
///
/// The login sends its own pre-session token. Each other mutating request sends the session token
/// of the principal, and a `GET` sends the session cookie only. [`fill`] replaces the placeholders
/// of the path and of the body.
pub(crate) async fn answer(endpoint: &Endpoint, principal: Principal) -> Reply {
    let (fixture, session) = world(principal).await;
    let router = fixture.router();
    let path = fill(endpoint.path);
    let body = fill(endpoint.body);
    let cookies: Vec<(&str, &str)> = session
        .iter()
        .map(|session| ("cookie", session.cookie.as_str()))
        .collect();
    if endpoint.method == Method::POST && endpoint.path == "/login" {
        let fields = format!("login={}&password={PASSWORD_FIELD}", login_of(principal));
        return post_pre_session(&fixture, &path, &fields, &cookies).await;
    }
    if endpoint.method == Method::POST && endpoint.path == "/lang" {
        return post_pre_session(&fixture, &path, &body, &cookies).await;
    }
    if endpoint.method == Method::GET {
        return get_with(&router, &path, &cookies).await;
    }
    post_fields(&router, &path, session.as_ref(), &[], &body).await
}

/// Assert a `303` to `location`.
pub(crate) fn assert_redirect(reply: &Reply, location: &str, what: &str) {
    assert_eq!(
        reply.status,
        StatusCode::SEE_OTHER,
        "{what}: {}",
        reply.body
    );
    assert_eq!(reply.header("location"), location, "{what}");
}

/// Assert that `reply` is the answer that `want` names.
pub(crate) fn assert_want(reply: &Reply, want: Want, what: &str) {
    match want {
        Want::Page => assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body),
        Want::Login => assert_redirect(reply, "/login", what),
        Want::Password => assert_redirect(reply, "/me/password", what),
        Want::Recipes => assert_redirect(reply, "/recipes", what),
        Want::Redirect(location) => assert_redirect(reply, location, what),
        Want::Forbidden => assert_error(reply, StatusCode::FORBIDDEN, "E-WEB-002", what),
        Want::Unauthorized(code) => assert_error(reply, StatusCode::UNAUTHORIZED, code, what),
        Want::Conflict(code) => assert_error(reply, StatusCode::CONFLICT, code, what),
        Want::Status(status, code) => assert_error(reply, status, code, what),
    }
}

#[cfg(test)]
mod recipe_tests;
#[cfg(test)]
mod tests;
