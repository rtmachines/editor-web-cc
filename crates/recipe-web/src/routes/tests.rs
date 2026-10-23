use axum::body::{Body, to_bytes};
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, Request, StatusCode};
use axum::routing::get;
use axum::{Router, response::Response};
use tower::ServiceExt as _;

use recipe_domain::audit::SecurityEventKind;
use recipe_domain::identity::Role;

use super::matrix::{
    Endpoint, PRINCIPALS, Want, answer, assert_error, assert_want, endpoint, fill,
};
use super::*;
use crate::csrf::issue_pre_session;
use crate::test_support::{Fixture, PEER, call, cookie_pair, request, state};

const BODY_LIMIT: usize = 64 * 1024;

async fn send(router: Router, method: Method, path: &str) -> (Response, String) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, BODY_LIMIT).await.unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    (Response::from_parts(parts, Body::empty()), text)
}

fn with_unlisted_route() -> Router {
    let unlisted = Router::new().route("/test/unlisted", get(|| async { "handler reached" }));
    assemble(state(), table(), unlisted).layer(MockConnectInfo(PEER))
}

#[tokio::test]
async fn routes_without_an_operation_fail_closed() {
    let (response, body) = send(with_unlisted_route(), Method::GET, "/test/unlisted").await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body.contains("E-WEB-010"), "{body}");
    assert!(!body.contains("handler reached"), "{body}");
}

#[tokio::test]
async fn routes_with_an_entry_in_the_table_reach_their_handler() {
    let (response, body) = send(with_unlisted_route(), Method::GET, "/static/htmx.min.js").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body.starts_with("var htmx="));
}

#[tokio::test]
async fn routes_listed_for_another_method_answer_not_found() {
    let (response, _body) = send(with_unlisted_route(), Method::POST, "/static/htmx.min.js").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn routes_unknown_path_answers_not_found() {
    let (response, _body) = send(with_unlisted_route(), Method::GET, "/no/such/page").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn routes_table_has_one_row_for_each_method_and_path() {
    let rows = table();
    let mut keys: Vec<_> = rows
        .iter()
        .map(|row| (row.method.as_str().to_owned(), row.path))
        .collect();
    keys.sort();
    let count = keys.len();
    keys.dedup();
    assert_eq!(keys.len(), count);
}

#[test]
fn routes_static_file_has_no_operation() {
    let rows = table();
    let row = rows
        .iter()
        .find(|row| row.path == "/static/htmx.min.js")
        .unwrap();
    assert_eq!(row.access, Access::Static);
}

// --- Endpoint x role (task 6.9, design D19) ------------------------------------------------------

/// The host of the test requests.
const HOST: &str = "recipes.plant.local";
/// The origin of the foreign-origin requests.
const FOREIGN_ORIGIN: &str = "https://evil.example";

/// The rows of the matrix: the rows of P3.1 and the rows of the six areas of design D12.
///
/// The types of the matrix live in `routes::matrix`, so the test modules of the areas can build
/// their own rows.
fn matrix() -> Vec<(Endpoint, [Want; 6])> {
    let mut rows = p31_matrix();
    rows.extend(pages::machines::route_tests::route_rows());
    rows.extend(pages::units::route_tests::route_rows());
    rows.extend(pages::types::route_tests::route_rows());
    rows.extend(pages::recipes::route_tests::route_rows());
    rows.extend(pages::versions::route_tests::route_rows());
    rows.extend(pages::parameters::route_tests::route_rows());
    rows.extend(p34_rows());
    rows
}

/// The rows of step P3.4: history, audit, and security events (wave 3), and the approval preview
/// (wave 4).
///
/// The rows of delete and restore live in `pages::recipes::route_tests`. The rows of the approval
/// and of the diff live in `pages::versions::route_tests`, so the security sweeps of the recipe
/// areas find them. Each row holds the answer of each principal. The placeholders of
/// `routes::matrix::fill` name `R-100`, the audit entry 1, and the old side `{m}` of a diff (2).
pub(super) fn p34_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::*;
    let view = [Login, Page, Page, Page, Password, Login];
    let audit = [Login, Page, Page, Forbidden, Password, Login];
    // The store of the matrix holds no audit entry, so a role with a permission gets `404`.
    let absent = Status(StatusCode::NOT_FOUND, "E-WEB-005");
    let entry = [Login, absent, absent, Forbidden, Password, Login];
    let events = [Login, Page, Forbidden, Forbidden, Password, Login];
    // The Administrator `user1` created `R-100`, so the preview shows `E-RCP-015` with `200`.
    let approve = [Login, Page, Page, Forbidden, Password, Login];
    vec![
        (endpoint(Method::GET, "/audit", ""), audit),
        (endpoint(Method::GET, "/audit/{id}", ""), entry),
        (endpoint(Method::GET, "/security-events", ""), events),
        (endpoint(Method::GET, "/recipes/{ref}/history", ""), view),
        (
            endpoint(Method::GET, "/recipes/{ref}/versions/{n}/approve", ""),
            approve,
        ),
    ]
}

/// The number of the rows of `rows` that send a mutating method.
fn mutating_rows(rows: &[(Endpoint, [Want; 6])]) -> usize {
    rows.iter()
        .filter(|(endpoint, _wants)| crate::csrf::is_mutating(&endpoint.method))
        .count()
}

/// The matrix of `api.md` sections 2.1 and 2.8, the rows of step P3.1. Each row holds the answer
/// of each principal.
fn p31_matrix() -> Vec<(Endpoint, [Want; 6])> {
    let mut rows = p31_session_rows();
    rows.extend(p31_user_rows());
    rows.extend(p31_own_rows());
    rows
}

/// The rows of P3.1 for the login, the logout, and the language. The row `GET /recipes` moved to
/// the area `recipes` (design D12).
fn p31_session_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::*;
    vec![
        (endpoint(Method::GET, "/login", ""), [Page; 6]),
        (
            endpoint(Method::POST, "/login", ""),
            [
                Recipes,
                Recipes,
                Recipes,
                Recipes,
                Password,
                Unauthorized("E-IDN-003"),
            ],
        ),
        (endpoint(Method::POST, "/logout", ""), [Login; 6]),
        (
            endpoint(Method::POST, "/lang", "lang=en&next=/recipes"),
            [Recipes; 6],
        ),
    ]
}

/// The rows of P3.1 for the user list, the user form, the user page, and the user commands.
fn p31_user_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::*;
    let manage = [Login, Page, Forbidden, Forbidden, Password, Login];
    vec![
        (endpoint(Method::GET, "/users", ""), manage),
        (endpoint(Method::GET, "/users/new", ""), manage),
        (
            endpoint(Method::POST, "/users", "login=newuser&role=operator"),
            manage,
        ),
        (endpoint(Method::GET, "/users/{login}", ""), manage),
        (
            endpoint(Method::POST, "/users/{login}/role", "role=operator"),
            manage,
        ),
        (
            endpoint(Method::POST, "/users/{login}/deactivate", ""),
            manage,
        ),
        (
            endpoint(Method::POST, "/users/{login}/activate", ""),
            [
                Login,
                Conflict("E-IDN-009"),
                Forbidden,
                Forbidden,
                Password,
                Login,
            ],
        ),
        (
            endpoint(Method::POST, "/users/{login}/reset-password", ""),
            manage,
        ),
        (endpoint(Method::POST, "/users/{login}/unlock", ""), manage),
    ]
}

/// The rows of P3.1 for the own profile and the own password.
fn p31_own_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::*;
    let own = [Login, Page, Page, Page, Password, Login];
    vec![
        (endpoint(Method::GET, "/me", ""), own),
        (
            endpoint(Method::GET, "/me/password", ""),
            [Login, Page, Page, Page, Page, Login],
        ),
        (
            endpoint(
                Method::POST,
                "/me/password",
                "current_password=correct+horse+battery&password=another+long+passphrase",
            ),
            [Login, Recipes, Recipes, Recipes, Recipes, Login],
        ),
    ]
}

#[tokio::test]
async fn endpoint_role_each_route_answers_each_principal() {
    for (endpoint, wants) in matrix() {
        for (principal, want) in PRINCIPALS.into_iter().zip(wants) {
            let reply = answer(&endpoint, principal).await;
            let what = format!("{} {} as {principal:?}", endpoint.method, endpoint.path);
            assert_want(&reply, want, &what);
        }
    }
}

// --- Route coverage (task 6.9, design D19) -------------------------------------------------------

/// The two export routes of `api.md` section 2.7. Task 7.4 writes their rows of the six principals
/// in its own file, so the matrix does not hold them (design D12).
const EXPORT_ROUTES: [&str; 2] = [
    "/export/v1/recipes/{ref}/approved",
    "/export/v1/recipes/{ref}/versions/{n}",
];

/// The routes of the matrix, the export routes, and the static file, as sorted pairs of method
/// and path.
fn expected_routes() -> Vec<(String, &'static str)> {
    let mut routes: Vec<(String, &'static str)> = matrix()
        .iter()
        .map(|(endpoint, _wants)| (endpoint.method.to_string(), endpoint.path))
        .collect();
    routes.extend(EXPORT_ROUTES.map(|path| ("GET".to_owned(), path)));
    routes.push(("GET".to_owned(), "/static/htmx.min.js"));
    routes.push(("GET".to_owned(), "/static/app.css"));
    routes.push(("GET".to_owned(), "/static/components.css"));
    routes.push(("GET".to_owned(), "/static/row-version.js"));
    routes.push(("GET".to_owned(), "/static/tree.js"));
    routes.extend(
        font_paths()
            .into_iter()
            .map(|path| ("GET".to_owned(), path)),
    );
    routes.sort();
    routes
}

/// The routes of the 8 font files (design D3). `static_files::FONTS` is the one list of them.
fn font_paths() -> Vec<&'static str> {
    crate::static_files::FONTS
        .iter()
        .map(|font| font.path)
        .collect()
}

#[test]
fn routes_each_font_file_has_a_static_row() {
    let rows = table();
    let paths = font_paths();
    assert_eq!(paths.len(), 8);
    for path in paths {
        let row = rows.iter().find(|row| row.path == path).unwrap();
        assert_eq!(row.access, Access::Static, "{path}");
    }
}

#[test]
fn route_coverage_table_holds_the_routes_of_the_matrix_the_export_area_and_the_static_files() {
    let mut listed: Vec<(String, &'static str)> = table()
        .iter()
        .map(|row| (row.method.to_string(), row.path))
        .collect();
    listed.sort();
    assert_eq!(listed, expected_routes());
}

/// The method and the path of each row, as pairs.
fn keys_of(rows: &[RouteEntry]) -> Vec<(String, &'static str)> {
    rows.iter()
        .map(|row| (row.method.to_string(), row.path))
        .collect()
}

#[test]
fn routes_table_collects_the_rows_of_each_area() {
    let listed = keys_of(&table());
    let areas = [
        ("machines", pages::machines::rows()),
        ("units", pages::units::rows()),
        ("types", pages::types::rows()),
        ("recipes", pages::recipes::rows()),
        ("versions", pages::versions::rows()),
        ("parameters", pages::parameters::rows()),
        ("history", pages::history::rows()),
        ("audit", pages::audit::rows()),
        ("security_events", pages::security_events::rows()),
        ("export", pages::export::rows()),
    ];
    for (area, rows) in &areas {
        for key in keys_of(rows) {
            assert!(listed.contains(&key), "{area}: {key:?} is not in the table");
        }
    }
    let area_rows: usize = areas.iter().map(|(_area, rows)| rows.len()).sum();
    let static_rows = expected_routes()
        .len()
        .checked_sub(matrix().len())
        .and_then(|rows| rows.checked_sub(EXPORT_ROUTES.len()))
        .unwrap();
    let parts = [static_rows, p31_matrix().len(), area_rows];
    assert_eq!(listed.len(), parts.iter().sum::<usize>());
}

#[test]
fn routes_matrix_collects_the_rows_of_each_area() {
    let keys: Vec<(String, &'static str)> = matrix()
        .iter()
        .map(|(endpoint, _wants)| (endpoint.method.to_string(), endpoint.path))
        .collect();
    let areas = [
        ("machines", pages::machines::route_tests::route_rows()),
        ("units", pages::units::route_tests::route_rows()),
        ("types", pages::types::route_tests::route_rows()),
        ("recipes", pages::recipes::route_tests::route_rows()),
        ("versions", pages::versions::route_tests::route_rows()),
        ("parameters", pages::parameters::route_tests::route_rows()),
    ];
    for (area, rows) in &areas {
        for (endpoint, _wants) in rows {
            let key = (endpoint.method.to_string(), endpoint.path);
            assert!(keys.contains(&key), "{area}: {key:?} is not in the matrix");
        }
    }
    let area_rows: usize = areas.iter().map(|(_area, rows)| rows.len()).sum();
    let expected = p31_matrix()
        .len()
        .checked_add(area_rows)
        .and_then(|rows| rows.checked_add(p34_rows().len()))
        .unwrap();
    assert_eq!(matrix().len(), expected);
}

#[test]
fn routes_the_recipe_list_row_comes_from_the_recipes_area_and_stands_once() {
    let rows = table();
    let list: Vec<&RouteEntry> = rows
        .iter()
        .filter(|row| row.method == Method::GET && row.path == "/recipes")
        .collect();
    assert_eq!(list.len(), 1);
    assert_eq!(
        list.first().unwrap().access,
        Access::Operation(Operation::ListRecipes)
    );
    let area = pages::recipes::rows();
    assert!(keys_of(&area).contains(&("GET".to_owned(), "/recipes")));
}

#[test]
fn routes_the_rows_of_p31_hold_ten_mutating_rows() {
    assert_eq!(mutating_rows(&p31_matrix()), 10);
}

/// The four defects of design D19 for one mutating request.
#[derive(Clone, Copy, Debug)]
enum Defect {
    /// No token. A pre-session request also loses its cookie.
    NoToken,
    /// The token of another session, or of another pre-session form.
    OtherToken,
    /// A foreign `Origin`, and no fetch metadata.
    ForeignOrigin,
    /// No `Origin` and no `Sec-Fetch-Site`.
    NoOriginNoMetadata,
}

/// A cookie and the token that belongs to it.
struct Credential {
    cookie: String,
    token: String,
}

/// The credentials of the coverage test: a session and a pre-session token, and the other ones.
struct Credentials {
    session: Credential,
    other_session: Credential,
    pre: Credential,
    other_pre: Credential,
}

fn pre_credential(fixture: &Fixture) -> Credential {
    let token = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    Credential {
        cookie: cookie_pair(token.set_cookie().to_str().unwrap()),
        token: token.nonce().to_owned(),
    }
}

fn credentials(fixture: &Fixture) -> Credentials {
    let session = fixture.seed_session(1, Role::Administrator, false);
    let other = fixture.seed_session(2, Role::ProcessEngineer, false);
    Credentials {
        session: Credential {
            cookie: session.cookie,
            token: session.csrf,
        },
        other_session: Credential {
            cookie: other.cookie,
            token: other.csrf,
        },
        pre: pre_credential(fixture),
        other_pre: pre_credential(fixture),
    }
}

/// Build one bad request. `pre_mode` selects the pre-session token of `POST /login` and `/lang`.
fn bad_request(
    method: Method,
    path: &str,
    pre_mode: bool,
    defect: Defect,
    own: &Credential,
    other: &Credential,
) -> Request<Body> {
    let mut headers: Vec<(&str, String)> = Vec::new();
    if !(pre_mode && matches!(defect, Defect::NoToken)) {
        headers.push(("cookie", own.cookie.clone()));
    }
    let token = match defect {
        Defect::NoToken => None,
        Defect::OtherToken => Some(other.token.clone()),
        Defect::ForeignOrigin | Defect::NoOriginNoMetadata => Some(own.token.clone()),
    };
    let mut body = String::from("a=b");
    match (pre_mode, token) {
        (true, Some(token)) => body = format!("a=b&csrf_pre={token}"),
        (false, Some(token)) => headers.push(("x-csrf-token", token)),
        _ => {}
    }
    match defect {
        Defect::NoToken | Defect::OtherToken => {
            headers.push(("sec-fetch-site", "same-origin".to_owned()));
        }
        Defect::ForeignOrigin => {
            headers.push(("host", HOST.to_owned()));
            headers.push(("origin", FOREIGN_ORIGIN.to_owned()));
        }
        Defect::NoOriginNoMetadata => headers.push(("host", HOST.to_owned())),
    }
    let mut builder =
        request(method, path).header("content-type", "application/x-www-form-urlencoded");
    for (name, value) in headers {
        builder = builder.header(name, value);
    }
    builder.body(Body::from(body)).unwrap()
}

/// Send `bad` and assert `403 E-WEB-007`, one `csrf_rejected` event, and no change of the data.
async fn assert_csrf_rejected(fixture: &Fixture, router: &Router, bad: Request<Body>, what: &str) {
    let events_before = fixture.sink.events().len();
    let state_before = format!("{:?}", fixture.store.state());
    let audit_before = fixture.store.audit_entries().len();
    let reply = call(router, bad).await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-007", what);
    let events = fixture.sink.events();
    assert_eq!(
        events.len(),
        events_before.checked_add(1).unwrap(),
        "{what}"
    );
    assert!(
        events
            .last()
            .is_some_and(|event| event.kind() == SecurityEventKind::CsrfRejected),
        "{what}"
    );
    assert_eq!(
        format!("{:?}", fixture.store.state()),
        state_before,
        "{what}"
    );
    assert_eq!(fixture.store.audit_entries().len(), audit_before, "{what}");
}

#[tokio::test]
async fn route_coverage_each_mutating_route_rejects_four_bad_requests() {
    let fixture = Fixture::new();
    let credentials = credentials(&fixture);
    let router =
        assemble(fixture.state.clone(), table(), Router::new()).layer(MockConnectInfo(PEER));
    let mutating: Vec<(Method, &'static str)> = table()
        .into_iter()
        .filter(|row| crate::csrf::is_mutating(&row.method))
        .map(|row| (row.method, row.path))
        .collect();
    assert_eq!(mutating.len(), mutating_rows(&matrix()));
    for (method, template) in mutating {
        let path = fill(template);
        let pre_mode = path == "/login" || path == "/lang";
        let (own, other) = if pre_mode {
            (&credentials.pre, &credentials.other_pre)
        } else {
            (&credentials.session, &credentials.other_session)
        };
        for defect in [
            Defect::NoToken,
            Defect::OtherToken,
            Defect::ForeignOrigin,
            Defect::NoOriginNoMetadata,
        ] {
            let what = format!("{method} {path} {defect:?}");
            let bad = bad_request(method.clone(), &path, pre_mode, defect, own, other);
            assert_csrf_rejected(&fixture, &router, bad, &what).await;
        }
    }
}

/// A `PATCH` to a path of the table stops in the access layer (step 4), before the CSRF layer.
///
/// The table has no `PATCH` row, so `resolve_access` answers `404` with the fallback text. The test sends the
/// request with a valid session but with no token and no `Origin`. No handler runs and nothing
/// changes.
#[tokio::test]
async fn route_coverage_a_patch_to_a_listed_path_stops_before_any_handler() {
    let fixture = Fixture::new();
    let credentials = credentials(&fixture);
    let router =
        assemble(fixture.state.clone(), table(), Router::new()).layer(MockConnectInfo(PEER));
    let paths: Vec<&'static str> = table()
        .into_iter()
        .filter(|row| crate::csrf::is_mutating(&row.method))
        .map(|row| row.path)
        .collect();
    for template in paths {
        let path = fill(template);
        let what = format!("PATCH {path}");
        let events_before = fixture.sink.events().len();
        let state_before = format!("{:?}", fixture.store.state());
        let audit_before = fixture.store.audit_entries().len();
        let bad = request(Method::PATCH, &path)
            .header("content-type", "application/x-www-form-urlencoded")
            .header("cookie", credentials.session.cookie.as_str())
            .body(Body::from("role=operator"))
            .unwrap();
        let reply = call(&router, bad).await;
        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{what}: {}",
            reply.body
        );
        assert_eq!(fixture.sink.events().len(), events_before, "{what}");
        assert_eq!(
            format!("{:?}", fixture.store.state()),
            state_before,
            "{what}"
        );
        assert_eq!(fixture.store.audit_entries().len(), audit_before, "{what}");
    }
}
