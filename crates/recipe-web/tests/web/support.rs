//! Support code of the HTTP tests: a router over an in-memory store.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, Request};
use axum::response::Response;
use recipe_app::blocking::AppSettings;
use recipe_app::context::AppContext;
use recipe_app::identity::{
    Argon2Hasher, Argon2Params, HashGate, IdentityContext, IdentitySettings,
};
use recipe_app::memory::{MemoryClock, MemorySink, MemoryStore};
use recipe_domain::identity::KeyRing;
use recipe_web::{WebSettings, WebState, router};
use time::OffsetDateTime;
use tower::ServiceExt as _;

/// A key file with one valid key of 64 lower-case hex characters.
const LOGIN_KEY: &str = "0101010101010101010101010101010101010101010101010101010101010101";
/// The key file of the pre-session CSRF cookie.
const CSRF_KEY: &str = "0202020202020202020202020202020202020202020202020202020202020202";
/// The socket address of the test client.
pub const PEER: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)), 40_000);
/// The start of the clock, in Unix seconds.
const START_SECONDS: i64 = 1_000_000;
/// The longest response body that a test reads.
const BODY_LIMIT: usize = 1024 * 1024;
/// Cheap Argon2 values, so a test does not wait for a real hash.
const TEST_PARAMS: Argon2Params = Argon2Params {
    m_cost_kib: 8,
    t_cost: 1,
    p_cost: 1,
};

/// The state of a test and the adapters behind it.
pub struct Harness {
    /// The state of the router.
    pub state: WebState,
    /// The store that the use cases write to.
    pub store: MemoryStore,
    /// The sink that records the security events.
    pub sink: MemorySink,
}

impl Harness {
    /// Build a harness with the given web settings.
    pub fn with_settings(web: WebSettings) -> Self {
        let start = OffsetDateTime::from_unix_timestamp(START_SECONDS).unwrap();
        let clock = MemoryClock::new(start);
        let store = MemoryStore::with_clock(clock.clone());
        let sink = MemorySink::default();
        let settings = IdentitySettings::default();
        let app = AppContext::new(
            Arc::new(store.clone()),
            Arc::new(sink.clone()),
            Arc::new(clock),
            AppSettings::default(),
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
        let state = WebState::new(identity, web, KeyRing::parse(CSRF_KEY).unwrap()).unwrap();
        Self { state, store, sink }
    }

    /// Build a harness with the default web settings.
    pub fn new() -> Self {
        Self::with_settings(WebSettings::default())
    }

    /// Build the router over the state. The client socket is `PEER`.
    pub fn router(&self) -> Router {
        router(self.state.clone()).layer(MockConnectInfo(PEER))
    }
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a router over a fresh state with the default settings.
pub fn app() -> Router {
    Harness::new().router()
}

/// Build a router over a fresh state with the given web settings.
pub fn router_with(web: WebSettings) -> Router {
    Harness::with_settings(web).router()
}

/// Send one request without a body and return the response.
pub async fn send(router: &Router, method: Method, path: &str) -> Response {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .unwrap();
    router.clone().oneshot(request).await.unwrap()
}

/// Send a `GET` request.
pub async fn get(router: &Router, path: &str) -> Response {
    send(router, Method::GET, path).await
}

/// Send a form with the method `POST` and the given extra headers.
pub async fn post_form(
    router: &Router,
    path: &str,
    body: &str,
    headers: &[(&str, &str)],
) -> Response {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("content-type", "application/x-www-form-urlencoded");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let request = builder.body(Body::from(body.to_owned())).unwrap();
    router.clone().oneshot(request).await.unwrap()
}

/// A pre-session token for the router of `Harness`: the cookie pair and the form nonce.
pub struct PreSession {
    /// The pair `__Host-recipe_csrf=<value>` for the `Cookie` header.
    pub cookie: String,
    /// The nonce for the form field `csrf_pre`.
    pub nonce: String,
}

/// Build a valid pre-session token without the code of the crate.
///
/// The cookie value is `hex(nonce) "." hex(HMAC-SHA256(nonce))` under the first key.
pub fn pre_session() -> PreSession {
    let nonce_bytes = [0x5a_u8; 16];
    let hex = |bytes: &[u8]| -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() };
    let tag = KeyRing::parse(CSRF_KEY).unwrap().sign(&nonce_bytes);
    let nonce = hex(&nonce_bytes);
    PreSession {
        cookie: format!("__Host-recipe_csrf={nonce}.{}", hex(&tag)),
        nonce,
    }
}

/// Send `POST /lang` with a valid pre-session token and a same-origin marker.
///
/// `fields` holds the form fields without the token. `headers` come last, so a test can add a
/// cookie. The `Cookie` header of the token joins the header of the test when both exist.
pub async fn post_lang(router: &Router, fields: &str, headers: &[(&str, &str)]) -> Response {
    let token = pre_session();
    let body = if fields.is_empty() {
        format!("csrf_pre={}", token.nonce)
    } else {
        format!("{fields}&csrf_pre={}", token.nonce)
    };
    let own_cookies: Vec<&str> = headers
        .iter()
        .filter(|(name, _value)| *name == "cookie")
        .map(|(_name, value)| *value)
        .collect();
    let cookie = std::iter::once(token.cookie.as_str())
        .chain(own_cookies)
        .collect::<Vec<_>>()
        .join("; ");
    let mut all: Vec<(&str, &str)> = vec![
        ("sec-fetch-site", "same-origin"),
        ("cookie", cookie.as_str()),
    ];
    all.extend(headers.iter().filter(|(name, _value)| *name != "cookie"));
    post_form(router, "/lang", &body, &all).await
}

/// Read a response body.
pub async fn body_bytes(response: Response) -> Vec<u8> {
    to_bytes(response.into_body(), BODY_LIMIT)
        .await
        .unwrap()
        .to_vec()
}

/// Return the text of a header, or an empty string when it is missing.
pub fn header(response: &Response, name: &str) -> String {
    response
        .headers()
        .get(name)
        .map(|value| value.to_str().unwrap().to_owned())
        .unwrap_or_default()
}
