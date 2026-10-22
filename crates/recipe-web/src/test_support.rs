//! Support code of the unit tests: a state over an in-memory store, test routes, and sessions.

pub(crate) mod plant;
pub(crate) mod recipes;
pub(crate) mod source_walk;
pub(crate) mod visible_text;

use std::cell::RefCell;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::Extension;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::request::Builder;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use axum::routing::{MethodRouter, delete, get, patch, post, put};
use parking_lot::Mutex;
use recipe_app::auth::{Operation, RequestMeta};
use recipe_app::blocking::AppSettings;
use recipe_app::context::AppContext;
use recipe_app::identity::{
    Argon2Hasher, Argon2Params, Authenticated, DeactivateUser, HashGate, IdentityContext,
    IdentitySettings, PasswordHasher as _, authenticate,
};
use recipe_app::memory::{MemoryClock, MemorySink, MemoryStore};
use recipe_app::ports::{NewSession, NewUser};
use recipe_domain::audit::RequestId;
use recipe_domain::identity::{
    CsrfToken, KeyRing, Login, NewPassword, PasswordHash, Role, SessionId, SessionToken, UserId,
};
use time::OffsetDateTime;
use tower::ServiceExt as _;
use tracing_subscriber::fmt::MakeWriter;
use uuid::Uuid;

use crate::client::ClientAddr;
use crate::csrf::issue_pre_session;
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::routes::{Access, RouteEntry, assemble, table};
use crate::{WebSettings, WebState};

/// A key file with one valid key of 64 lower-case hex characters.
const LOGIN_KEY: &str = "0101010101010101010101010101010101010101010101010101010101010101";
/// The newest key of the pre-session CSRF cookie.
pub(crate) const CSRF_KEY_NEW: &str =
    "0202020202020202020202020202020202020202020202020202020202020202";
/// The older key of the pre-session CSRF cookie. A rotation keeps it in the second line.
pub(crate) const CSRF_KEY_OLD: &str =
    "0303030303030303030303030303030303030303030303030303030303030303";
/// The start of the clock, in Unix seconds.
const START_SECONDS: i64 = 1_000_000;
/// The idle timeout of a seeded session.
const IDLE_TIMEOUT: Duration = Duration::from_secs(1800);
/// The longest body that a test reads.
const BODY_LIMIT: usize = 1024 * 1024;
/// A password hash that the store accepts. No test checks it.
const HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0$ZG91YmxlZG91YmxlZG91YmxlZG91Ymxl";
/// The socket address of a test client.
pub(crate) const PEER: SocketAddr = SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, 10)),
    40_000,
);
/// Cheap Argon2 values, so a test does not wait for a real hash.
const TEST_PARAMS: Argon2Params = Argon2Params {
    m_cost_kib: 8,
    t_cost: 1,
    p_cost: 1,
};

/// A session that a test seeded.
pub(crate) struct TestSession {
    /// The value of the `Cookie` header.
    pub(crate) cookie: String,
    /// The CSRF token of the session as 64 hex characters.
    pub(crate) csrf: String,
}

/// The state of a test and the adapters behind it.
pub(crate) struct Fixture {
    /// The state of the router.
    pub(crate) state: WebState,
    /// The store behind the use cases.
    pub(crate) store: MemoryStore,
    /// The sink that records the security events.
    pub(crate) sink: MemorySink,
}

impl Fixture {
    /// Build a fixture with the default web settings and the key file of one key.
    pub(crate) fn new() -> Self {
        Self::with_settings(WebSettings::default(), CSRF_KEY_NEW)
    }

    /// Build a fixture with the given web settings and the given key file for the CSRF cookie.
    pub(crate) fn with_settings(web: WebSettings, csrf_keys: &str) -> Self {
        Self::build(web, csrf_keys, AppSettings::default())
    }

    /// Build a fixture from the three parts that a test changes.
    ///
    /// The function solves one problem: `with_settings` and `with_app_settings` must wire the state
    /// in one way. Only the web settings, the key file, and the settings `app_settings` of the use
    /// cases differ between them.
    fn build(web: WebSettings, csrf_keys: &str, app_settings: AppSettings) -> Self {
        install_global_subscriber();
        let start = OffsetDateTime::from_unix_timestamp(START_SECONDS).unwrap();
        let clock = MemoryClock::new(start);
        let store = MemoryStore::with_clock(clock.clone());
        let sink = store.sink();
        let settings = IdentitySettings::default();
        let app = AppContext::new(
            Arc::new(store.clone()),
            Arc::new(sink.clone()),
            Arc::new(clock),
            app_settings,
        );
        let hasher = Arc::new(Argon2Hasher::new(TEST_PARAMS).unwrap());
        let gate = HashGate::new(
            settings.argon2_permits,
            settings.argon2_acquire_timeout,
            hasher,
        );
        let identity = IdentityContext::new(
            app,
            Arc::new(store.clone()),
            Arc::new(store.clone()),
            gate,
            KeyRing::parse(LOGIN_KEY).unwrap(),
            settings,
        );
        let state = WebState::new(identity, web, KeyRing::parse(csrf_keys).unwrap()).unwrap();
        Self { state, store, sink }
    }

    /// Insert the user number `n` and one session of that user. Return the session.
    pub(crate) fn seed_session(
        &self,
        n: u8,
        role: Role,
        must_change_password: bool,
    ) -> TestSession {
        self.seed_user_row(
            n,
            role,
            must_change_password,
            PasswordHash::parse(HASH).unwrap(),
        );
        self.session_of(n)
    }

    /// Insert the user number `n` with the login `user<n>` and a real hash of `password`.
    ///
    /// The hash uses the cheap Argon2 values of the tests. The user has no session.
    pub(crate) fn seed_account(
        &self,
        n: u8,
        role: Role,
        password: &str,
        must_change_password: bool,
    ) {
        let hasher = Argon2Hasher::new(TEST_PARAMS).unwrap();
        let hash = hasher.hash(&NewPassword::parse(password).unwrap()).unwrap();
        self.seed_user_row(n, role, must_change_password, hash);
    }

    fn seed_user_row(&self, n: u8, role: Role, must_change_password: bool, hash: PasswordHash) {
        let user_id = UserId::from(Uuid::from_u128(u128::from(n)));
        self.store.seed_user(&NewUser {
            id: user_id,
            login: Login::parse(&format!("user{n}")).unwrap(),
            password_hash: hash,
            role,
            must_change_password,
        });
    }

    /// Insert one session of the seeded user number `n`. Return the session.
    pub(crate) fn session_of(&self, n: u8) -> TestSession {
        let user_id = UserId::from(Uuid::from_u128(u128::from(n)));
        let token = SessionToken::from_bytes([n; 16]);
        let csrf = CsrfToken::from_bytes([n.wrapping_add(100); 32]);
        self.store.seed_session(&NewSession {
            id: SessionId::of(&token),
            user_id,
            epoch: 0,
            csrf_token: csrf.clone(),
            idle_timeout: IDLE_TIMEOUT,
        });
        TestSession {
            cookie: format!("__Host-recipe_session={}", token.to_hex()),
            csrf: csrf.to_hex(),
        }
    }

    /// Build the router with the test routes and a client at `PEER`.
    pub(crate) fn router(&self) -> Router {
        self.router_at(PEER)
    }

    /// Build the router with the test routes and a client at `peer`.
    pub(crate) fn router_at(&self, peer: SocketAddr) -> Router {
        self.router_without_peer().layer(MockConnectInfo(peer))
    }

    /// Build the router with the test routes and no `ConnectInfo`.
    pub(crate) fn router_without_peer(&self) -> Router {
        let mut rows = table();
        rows.extend(test_rows());
        assemble(self.state.clone(), rows, Router::new())
    }
}

/// The text that the test handlers return.
pub(crate) const REACHED: &str = "handler reached";

async fn reached() -> &'static str {
    REACHED
}

async fn client(Extension(address): Extension<ClientAddr>) -> String {
    address.ip().to_string()
}

async fn who(Extension(session): Extension<Option<Authenticated>>) -> String {
    session.map_or_else(
        || "none".to_owned(),
        |session| session.user().login().as_str().to_owned(),
    )
}

/// One test row for each kind of requirement and each gate exception.
fn test_rows() -> Vec<RouteEntry> {
    let row = |method: Method,
               path: &'static str,
               operation: Operation,
               handler: MethodRouter<WebState>| RouteEntry {
        method,
        path,
        access: Access::Operation(operation),
        handler,
    };
    vec![
        row(
            Method::GET,
            "/test/login-form",
            Operation::GetLoginForm,
            get(who),
        ),
        row(
            Method::GET,
            "/test/client",
            Operation::GetLoginForm,
            get(client),
        ),
        row(Method::POST, "/test/login", Operation::Login, post(reached)),
        row(
            Method::GET,
            "/test/profile",
            Operation::GetOwnProfile,
            get(who),
        ),
        row(
            Method::GET,
            "/test/users",
            Operation::ListUsers,
            get(reached),
        ),
        row(
            Method::POST,
            "/test/users",
            Operation::CreateUser,
            post(reached),
        ),
        row(
            Method::PUT,
            "/test/users",
            Operation::SetUserRole,
            put(reached),
        ),
        row(
            Method::DELETE,
            "/test/users",
            Operation::DeactivateUser,
            delete(reached),
        ),
        row(
            Method::PATCH,
            "/test/users",
            Operation::ActivateUser,
            patch(reached),
        ),
        row(
            Method::GET,
            "/test/me-password",
            Operation::GetOwnPasswordForm,
            get(reached),
        ),
        row(
            Method::POST,
            "/test/me-password",
            Operation::ChangeOwnPassword,
            post(reached),
        ),
        row(
            Method::POST,
            "/test/logout",
            Operation::Logout,
            post(reached),
        ),
    ]
}

/// A response that a test read completely.
pub(crate) struct Reply {
    /// The status code.
    pub(crate) status: StatusCode,
    /// The headers.
    pub(crate) headers: HeaderMap,
    /// The body as text.
    pub(crate) body: String,
}

impl Reply {
    /// Return the text of a header, or an empty string.
    pub(crate) fn header(&self, name: &str) -> String {
        self.headers
            .get(name)
            .map(|value| value.to_str().unwrap().to_owned())
            .unwrap_or_default()
    }

    /// Return the text of every `Set-Cookie` header.
    pub(crate) fn set_cookies(&self) -> Vec<String> {
        self.headers
            .get_all("set-cookie")
            .iter()
            .map(|value| value.to_str().unwrap().to_owned())
            .collect()
    }
}

/// Send one request through `router` and read the whole answer.
pub(crate) async fn call(router: &Router, request: Request<Body>) -> Reply {
    let response = router.clone().oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, BODY_LIMIT).await.unwrap();
    Reply {
        status: parts.status,
        headers: parts.headers,
        body: String::from_utf8_lossy(&bytes).into_owned(),
    }
}

/// Start a request builder with the method and path.
pub(crate) fn request(method: Method, path: &str) -> Builder {
    Request::builder().method(method).uri(path)
}

/// Send a `GET` request with the headers.
pub(crate) async fn get_with(router: &Router, path: &str, headers: &[(&str, &str)]) -> Reply {
    let mut builder = request(Method::GET, path);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    call(router, builder.body(Body::empty()).unwrap()).await
}

/// Send a same-origin form with the method `POST`, the session, and the extra headers.
///
/// The request carries `Sec-Fetch-Site: same-origin` and, with a session, its CSRF token in the
/// header `X-CSRF-Token`. The extra headers come last.
pub(crate) async fn post_with(
    router: &Router,
    path: &str,
    session: Option<&TestSession>,
    headers: &[(&str, &str)],
) -> Reply {
    post_fields(router, path, session, headers, "a=b").await
}

/// Send a same-origin form with the given body, the session, and the extra headers.
///
/// The request carries `Sec-Fetch-Site: same-origin` and, with a session, its cookie and its
/// CSRF token in the header `X-CSRF-Token`. The extra headers come last.
pub(crate) async fn post_fields(
    router: &Router,
    path: &str,
    session: Option<&TestSession>,
    headers: &[(&str, &str)],
    body: &str,
) -> Reply {
    let mut builder = request(Method::POST, path)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin");
    if let Some(session) = session {
        builder = builder
            .header("cookie", session.cookie.as_str())
            .header("x-csrf-token", session.csrf.as_str());
    }
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    call(router, builder.body(Body::from(body.to_owned())).unwrap()).await
}

/// Deactivate the user `login` through the use case, as the Administrator of user number 9.
///
/// The use case ends the sessions of the user. A session of the user is invalid afterwards.
pub(crate) async fn deactivate_user(fixture: &Fixture, login: &str) {
    let _admin = fixture.seed_session(9, Role::Administrator, false);
    let token = SessionToken::from_bytes([9; 16]);
    let auth = authenticate(fixture.state.identity(), &token)
        .await
        .unwrap()
        .unwrap();
    let meta = RequestMeta::new(RequestId::from(Uuid::nil()), None, None, None);
    DeactivateUser::new(fixture.state.identity().clone())
        .execute(&auth, &meta, &Login::parse(login).unwrap())
        .await
        .unwrap();
}

/// Return the pair `name=value` of a `Set-Cookie` text, without the attributes.
pub(crate) fn cookie_pair(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap_or_default().to_owned()
}

/// Send a same-origin form with a valid pre-session token and the extra form fields.
///
/// The request carries the cookie `__Host-recipe_csrf` and the field `csrf_pre`.
pub(crate) async fn post_pre_session(
    fixture: &Fixture,
    path: &str,
    fields: &str,
    headers: &[(&str, &str)],
) -> Reply {
    let token = issue_pre_session(fixture.state.csrf_keys()).unwrap();
    let cookie = cookie_pair(token.set_cookie().to_str().unwrap());
    let body = format!("{fields}&csrf_pre={}", token.nonce());
    let mut builder = request(Method::POST, path)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", cookie);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    call(&fixture.router(), builder.body(Body::from(body)).unwrap()).await
}

/// A buffer that holds the log lines of one test.
#[derive(Clone, Default)]
pub(crate) struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl LogBuffer {
    /// Return everything that the subscriber wrote on the thread of the test.
    pub(crate) fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock()).into_owned()
    }
}

thread_local! {
    /// The buffer of the test that runs on this thread, if the test captures its log.
    static CAPTURE: RefCell<Option<LogBuffer>> = const { RefCell::new(None) };
}

/// A writer that copies a line into the buffer of the current thread, or drops it.
#[derive(Clone, Copy)]
struct ThreadWriter;

impl std::io::Write for ThreadWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        CAPTURE.with(|slot| {
            if let Some(buffer) = slot.borrow().as_ref() {
                buffer.0.lock().extend_from_slice(bytes);
            }
        });
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for ThreadWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        *self
    }
}

/// Install the one global subscriber of the unit tests, once.
///
/// A subscriber that each test sets for its own thread would race with the other tests: a callsite
/// that a parallel test hits first can stay off for the test that captures. A global subscriber
/// exists before any request runs, so no callsite starts off. The function returns false when
/// another subscriber was already global.
fn install_global_subscriber() -> bool {
    static INSTALLED: OnceLock<bool> = OnceLock::new();
    *INSTALLED.get_or_init(|| {
        let subscriber = tracing_subscriber::fmt()
            .with_writer(ThreadWriter)
            .with_ansi(false)
            .with_max_level(tracing::Level::TRACE)
            .finish();
        tracing::subscriber::set_global_default(subscriber).is_ok()
    })
}

/// End the capture of the thread when the test drops it.
pub(crate) struct CaptureGuard;

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        CAPTURE.with(|slot| *slot.borrow_mut() = None);
    }
}

/// Capture the log lines that the current thread writes. Keep the guard while the test runs.
pub(crate) fn capture_logs() -> (LogBuffer, CaptureGuard) {
    assert!(
        install_global_subscriber(),
        "another global subscriber exists"
    );
    let buffer = LogBuffer::default();
    CAPTURE.with(|slot| *slot.borrow_mut() = Some(buffer.clone()));
    (buffer, CaptureGuard)
}

/// Build a state over a fresh `MemoryStore` with the default settings.
pub(crate) fn state() -> WebState {
    Fixture::new().state
}

/// Return the text of `key` in `lang`.
pub(crate) fn text(lang: Lang, key: &str) -> String {
    let bundles = Bundles::load().unwrap();
    Texts::new(&bundles, lang).get(key)
}

/// Return the fixed opening tag of the icon `name` (`design-system.md` section 8).
pub(crate) fn icon_tag(name: &str) -> String {
    format!(
        "<svg class=\"icon\" aria-hidden=\"true\" focusable=\"false\" viewBox=\"0 0 24 24\" data-icon=\"{name}\" fill=\"none\" stroke=\"currentColor\">"
    )
}

/// Send `GET path` in the language `code`, with the session when there is one.
pub(crate) async fn get_page(
    fixture: &Fixture,
    session: Option<&TestSession>,
    code: &str,
    path: &str,
) -> Reply {
    let cookie = match session {
        Some(session) => format!("{}; recipe_lang={code}", session.cookie),
        None => format!("recipe_lang={code}"),
    };
    get_with(&fixture.router(), path, &[("cookie", cookie.as_str())]).await
}
