//! The route table and the router (design D1, D11).
//!
//! Each route names its access in one table. A route that the table does not list fails closed
//! with `500 E-WEB-010`, so a new route cannot skip the permission step by accident.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, MatchedPath, Request, State};
use axum::http::{Extensions, Method, StatusCode};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{MethodRouter, get, post};
use recipe_app::auth::Operation;
use tracing::Span;

use crate::client::client_address;
use crate::csrf::csrf_layer;
use crate::error::{WebError, render_errors};
use crate::gates::gates;
use crate::limits::{body_limit, largest_limit};
use crate::pages;
use crate::pipeline::request_id_in;
use crate::session::session_layer;
use crate::state::WebState;
use crate::static_files;
use crate::{lang, pipeline};

/// What a route needs from the caller.
///
/// The type solves one problem: the code must tell a route with a use case from a route without
/// one, and it must tell both from a route that nobody listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// The route runs this operation. The use case checks the permission first.
    Operation(Operation),
    /// The route serves a static file. It has no operation and needs no session.
    Static,
    /// `POST /lang`. The route has no use case. It works without a session and in read-only mode.
    Lang,
}

/// One row of the route table.
pub(crate) struct RouteEntry {
    /// The HTTP method of the row.
    pub(crate) method: Method,
    /// The path pattern of axum, for example `/recipes/{ref}`.
    pub(crate) path: &'static str,
    /// What the route needs.
    pub(crate) access: Access,
    /// The handler for `method`.
    pub(crate) handler: MethodRouter<WebState>,
}

/// Build one row of the table for a route with a use case.
pub(crate) fn operation_row(
    method: Method,
    path: &'static str,
    operation: Operation,
    handler: MethodRouter<WebState>,
) -> RouteEntry {
    RouteEntry {
        method,
        path,
        access: Access::Operation(operation),
        handler,
    }
}

/// Return the route table.
///
/// The table holds the static files and the rows of step P3.1. It also holds the rows of six
/// areas (design D12): the machines, the units, the types, the recipes, the versions, and the
/// parameters. Later tasks add one row for each page of `api.md` section 2 to its area.
pub(crate) fn table() -> Vec<RouteEntry> {
    let mut rows = static_rows();
    rows.extend(lang_and_login_rows());
    rows.extend(own_rows());
    rows.extend(pages::recipes::rows());
    rows.extend(user_list_rows());
    rows.extend(user_state_rows());
    rows.extend(pages::machines::rows());
    rows.extend(pages::units::rows());
    rows.extend(pages::types::rows());
    rows.extend(pages::versions::rows());
    rows.extend(pages::parameters::rows());
    rows.extend(pages::history::rows());
    rows.extend(pages::audit::rows());
    rows.extend(pages::security_events::rows());
    rows
}

/// Return one static row for each file of `static_files::STATIC_FILES` and `static_files::FONTS`.
fn static_rows() -> Vec<RouteEntry> {
    let files = static_files::STATIC_FILES.iter().map(|file| RouteEntry {
        method: Method::GET,
        path: file.path,
        access: Access::Static,
        handler: static_files::static_handler(file),
    });
    let fonts = static_files::FONTS.iter().map(|file| RouteEntry {
        method: Method::GET,
        path: file.path,
        access: Access::Static,
        handler: static_files::font_handler(file),
    });
    files.chain(fonts).collect()
}

/// Return the rows of the language, the login, and the logout.
fn lang_and_login_rows() -> Vec<RouteEntry> {
    vec![
        RouteEntry {
            method: Method::POST,
            path: "/lang",
            access: Access::Lang,
            handler: post(lang::switch),
        },
        operation_row(
            Method::GET,
            "/login",
            Operation::GetLoginForm,
            get(pages::login_form),
        ),
        operation_row(
            Method::POST,
            "/login",
            Operation::Login,
            post(pages::login_submit),
        ),
        operation_row(
            Method::POST,
            "/logout",
            Operation::Logout,
            post(pages::logout),
        ),
    ]
}

/// Return the rows of the own profile and the own password change.
fn own_rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/me",
            Operation::GetOwnProfile,
            get(pages::profile),
        ),
        operation_row(
            Method::GET,
            "/me/password",
            Operation::GetOwnPasswordForm,
            get(pages::password_form),
        ),
        operation_row(
            Method::POST,
            "/me/password",
            Operation::ChangeOwnPassword,
            post(pages::change_password),
        ),
    ]
}

/// Return the rows of the user list, the user form, the user detail, and the role change.
fn user_list_rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/users",
            Operation::ListUsers,
            get(pages::list),
        ),
        operation_row(
            Method::POST,
            "/users",
            Operation::CreateUser,
            post(pages::create),
        ),
        operation_row(
            Method::GET,
            "/users/new",
            Operation::GetUserForm,
            get(pages::new_form),
        ),
        operation_row(
            Method::GET,
            "/users/{login}",
            Operation::GetUser,
            get(pages::detail),
        ),
        operation_row(
            Method::POST,
            "/users/{login}/role",
            Operation::SetUserRole,
            post(pages::set_role),
        ),
        operation_row(
            Method::POST,
            "/users/{login}/deactivate",
            Operation::DeactivateUser,
            post(pages::deactivate),
        ),
    ]
}

/// Return the rows that activate, reset the password of, or unlock a user.
fn user_state_rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::POST,
            "/users/{login}/activate",
            Operation::ActivateUser,
            post(pages::activate),
        ),
        operation_row(
            Method::POST,
            "/users/{login}/reset-password",
            Operation::ResetPassword,
            post(pages::reset_password),
        ),
        operation_row(
            Method::POST,
            "/users/{login}/unlock",
            Operation::UnlockLogin,
            post(pages::unlock),
        ),
    ]
}

/// The access of each listed method and path.
#[derive(Default)]
struct AccessTable {
    rows: HashMap<&'static str, Vec<(Method, Access)>>,
}

/// The result of a lookup in the access table.
enum Lookup {
    /// The row exists.
    Found(Access),
    /// The path has rows, but none for this method.
    OtherMethod,
    /// The path has no row at all.
    Unlisted,
}

impl AccessTable {
    fn insert(&mut self, path: &'static str, method: Method, access: Access) {
        self.rows.entry(path).or_default().push((method, access));
    }

    fn lookup(&self, path: &str, method: &Method) -> Lookup {
        // axum answers HEAD with the handler of GET, so HEAD uses the row of GET.
        let wanted = if *method == Method::HEAD {
            &Method::GET
        } else {
            method
        };
        let Some(rows) = self.rows.get(path) else {
            return Lookup::Unlisted;
        };
        rows.iter()
            .find(|(listed, _access)| listed == wanted)
            .map_or(Lookup::OtherMethod, |(_listed, access)| {
                Lookup::Found(*access)
            })
    }
}

/// Answer a request for a path that no route serves.
async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "Not found").into_response()
}

/// Answer a request whose route has no row in the table with `500 E-WEB-010`.
fn fail_closed(extensions: &Extensions) -> Response {
    let request_id = request_id_in(extensions);
    WebError::Internal { request_id }.into_response()
}

/// Find the access of the matched route and put it into the request extensions.
///
/// A request without a matched path reaches the fallback, which serves no data. Every matched
/// route needs a row, else the answer is `500 E-WEB-010` and the handler does not run.
async fn resolve_access(
    State(table): State<Arc<AccessTable>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(matched) = request.extensions().get::<MatchedPath>() else {
        return next.run(request).await;
    };
    let found = table.lookup(matched.as_str(), request.method());
    Span::current().record("route", matched.as_str());
    match found {
        Lookup::Found(access) => {
            request.extensions_mut().insert(access);
            next.run(request).await
        }
        Lookup::OtherMethod => not_found().await,
        Lookup::Unlisted => fail_closed(request.extensions()),
    }
}

/// Build the router from the rows and a router of unlisted routes.
///
/// The router of unlisted routes exists for the test of the fail-closed rule.
pub(crate) fn assemble(
    state: WebState,
    entries: Vec<RouteEntry>,
    unlisted: Router<WebState>,
) -> Router {
    let mut access = AccessTable::default();
    let mut routes = Router::new();
    for entry in entries {
        access.insert(entry.path, entry.method, entry.access);
        routes = routes.route(entry.path, entry.handler);
    }
    let settings = state.settings().clone();
    // A layer that is added later runs earlier. The order of design D11 is therefore the reverse
    // of this list: render_errors, client, body limit, access, session, csrf, gates, handler.
    let routes = routes
        .merge(unlisted)
        .fallback(not_found)
        .layer(from_fn_with_state(state.clone(), gates))
        .layer(from_fn_with_state(state.clone(), csrf_layer))
        .layer(from_fn_with_state(state.clone(), session_layer))
        .layer(from_fn_with_state(Arc::new(access), resolve_access))
        .layer(DefaultBodyLimit::max(largest_limit(&settings)))
        .layer(from_fn_with_state(state.clone(), body_limit))
        .layer(from_fn_with_state(state.clone(), client_address))
        .layer(from_fn_with_state(state.clone(), render_errors))
        .with_state(state);
    pipeline::wrap(routes, &settings)
}

/// Build the router of the application.
///
/// Every answer carries the request id and the security headers of `api.md` section 9.
pub fn router(state: WebState) -> Router {
    assemble(state, table(), Router::new())
}

#[cfg(test)]
pub(crate) mod matrix;
#[cfg(test)]
mod plant_access_tests;
#[cfg(test)]
mod recipe_access_tests;
#[cfg(test)]
mod tests;
