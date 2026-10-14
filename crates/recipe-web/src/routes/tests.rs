use axum::body::{Body, to_bytes};
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, Request, StatusCode};
use axum::routing::get;
use axum::{Router, response::Response};
use tower::ServiceExt as _;

use recipe_domain::audit::SecurityEventKind;
use recipe_domain::identity::Role;

use super::*;
use crate::csrf::issue_pre_session;
use crate::test_support::{
    Fixture, PEER, Reply, TestSession, call, cookie_pair, deactivate_user, get_with, post_fields,
    post_pre_session, request, state,
};

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

/// The password of every seeded account.
const PASSWORD: &str = "correct horse battery";
/// The password of `PASSWORD` as a form value.
const PASSWORD_FIELD: &str = "correct+horse+battery";
/// The host of the test requests.
const HOST: &str = "recipes.plant.local";
/// The origin of the foreign-origin requests.
const FOREIGN_ORIGIN: &str = "https://evil.example";

/// The six principals of design D19.
#[derive(Clone, Copy, Debug)]
enum Principal {
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

/// The principals in the order of the columns of `matrix`.
const PRINCIPALS: [Principal; 6] = [
    Principal::NoSession,
    Principal::Administrator,
    Principal::ProcessEngineer,
    Principal::Operator,
    Principal::MustChange,
    Principal::Deactivated,
];

/// The expected answer of one cell of the matrix.
#[derive(Clone, Copy, Debug)]
enum Want {
    /// `200` with the page or the fragment.
    Page,
    /// `303` to `/login`. A page request without a valid session gets it (`E-WEB-004`).
    Login,
    /// `303` to `/me/password`. A user with `must_change_password` gets it (`E-IDN-007`).
    Password,
    /// `303` to `/recipes`. A command without `HX-Request` gets it after success.
    Recipes,
    /// `401` with the code in the body.
    Unauthorized(&'static str),
    /// `403` with `E-WEB-002` in the body. The role has no permission.
    Forbidden,
    /// `409` with the code in the body.
    Conflict(&'static str),
}

/// One endpoint of the matrix with a form body that the endpoint accepts.
struct Endpoint {
    method: Method,
    path: &'static str,
    body: &'static str,
}

fn endpoint(method: Method, path: &'static str, body: &'static str) -> Endpoint {
    Endpoint { method, path, body }
}

/// The matrix of `api.md` sections 2.1 and 2.8. Each row holds the answer of each principal.
fn matrix() -> Vec<(Endpoint, [Want; 6])> {
    use Want::*;
    let manage = [Login, Page, Forbidden, Forbidden, Password, Login];
    let own = [Login, Page, Page, Page, Password, Login];
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
        (endpoint(Method::GET, "/recipes", ""), own),
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

/// Build a fixture with five accounts that share `PASSWORD`, and the session of `principal`.
async fn world(principal: Principal) -> (Fixture, Option<TestSession>) {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Administrator, PASSWORD, false);
    fixture.seed_account(2, Role::ProcessEngineer, PASSWORD, false);
    fixture.seed_account(3, Role::Operator, PASSWORD, false);
    fixture.seed_account(4, Role::Administrator, PASSWORD, true);
    fixture.seed_account(5, Role::Administrator, PASSWORD, false);
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
/// of the principal, and a `GET` sends the session cookie only. The path `{login}` names `user2`.
async fn answer(endpoint: &Endpoint, principal: Principal) -> Reply {
    let (fixture, session) = world(principal).await;
    let router = fixture.router();
    let path = endpoint.path.replace("{login}", "user2");
    let cookies: Vec<(&str, &str)> = session
        .iter()
        .map(|session| ("cookie", session.cookie.as_str()))
        .collect();
    if endpoint.method == Method::POST && endpoint.path == "/login" {
        let fields = format!("login={}&password={PASSWORD_FIELD}", login_of(principal));
        return post_pre_session(&fixture, &path, &fields, &cookies).await;
    }
    if endpoint.method == Method::POST && endpoint.path == "/lang" {
        return post_pre_session(&fixture, &path, endpoint.body, &cookies).await;
    }
    if endpoint.method == Method::GET {
        return get_with(&router, &path, &cookies).await;
    }
    post_fields(&router, &path, session.as_ref(), &[], endpoint.body).await
}

fn assert_redirect(reply: &Reply, location: &str, what: &str) {
    assert_eq!(
        reply.status,
        StatusCode::SEE_OTHER,
        "{what}: {}",
        reply.body
    );
    assert_eq!(reply.header("location"), location, "{what}");
}

fn assert_error(reply: &Reply, status: StatusCode, code: &str, what: &str) {
    assert_eq!(reply.status, status, "{what}: {}", reply.body);
    assert!(reply.body.contains(code), "{what}: {}", reply.body);
}

fn assert_want(reply: &Reply, want: Want, what: &str) {
    match want {
        Want::Page => assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body),
        Want::Login => assert_redirect(reply, "/login", what),
        Want::Password => assert_redirect(reply, "/me/password", what),
        Want::Recipes => assert_redirect(reply, "/recipes", what),
        Want::Forbidden => assert_error(reply, StatusCode::FORBIDDEN, "E-WEB-002", what),
        Want::Unauthorized(code) => assert_error(reply, StatusCode::UNAUTHORIZED, code, what),
        Want::Conflict(code) => assert_error(reply, StatusCode::CONFLICT, code, what),
    }
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

/// The routes of the matrix and the static file, as sorted pairs of method and path.
fn expected_routes() -> Vec<(String, &'static str)> {
    let mut routes: Vec<(String, &'static str)> = matrix()
        .iter()
        .map(|(endpoint, _wants)| (endpoint.method.to_string(), endpoint.path))
        .collect();
    routes.push(("GET".to_owned(), "/static/htmx.min.js"));
    routes.push(("GET".to_owned(), "/static/app.css"));
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
fn route_coverage_table_holds_the_routes_of_the_matrix_and_the_static_file() {
    let mut listed: Vec<(String, &'static str)> = table()
        .iter()
        .map(|row| (row.method.to_string(), row.path))
        .collect();
    listed.sort();
    assert_eq!(listed, expected_routes());
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
    assert_eq!(mutating.len(), 10);
    for (method, template) in mutating {
        let path = template.replace("{login}", "user2");
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
            let events_before = fixture.sink.events().len();
            let state_before = format!("{:?}", fixture.store.state());
            let audit_before = fixture.store.audit_entries().len();
            let bad = bad_request(method.clone(), &path, pre_mode, defect, own, other);
            let reply = call(&router, bad).await;
            assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-007", &what);
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
        let path = template.replace("{login}", "user2");
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
