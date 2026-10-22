//! Development server of the recipe editor for the test in a browser.
//!
//! This example runs the router of `recipe-web` on the in-memory adapters.
//! It has no database. The process loses all data when it stops.
//! Use it only for the test in a browser. Never use it in production.
//!
//! To start the server, do these steps:
//!
//! 1. Set the variable `RECIPE_DEV_PASSWORD`. The password has 12 to 128 characters.
//! 2. Set the variable `RECIPE_DEV_PORT` to change the port. The default port is 8080.
//! 3. Run `cargo run -p recipe-web --example dev_server`.
//! 4. Open `http://127.0.0.1:8080/login` in Chromium.
//!
//! The server listens on `127.0.0.1` only. It seeds the primitive types and no machine.
//! It seeds the users `admin` (Administrator), `engineer` (Process engineer), and `operator`
//! (Operator). All three users have the password of `RECIPE_DEV_PASSWORD`.
//! The server never writes the password to a log.
//! Each start makes new signing keys, so every browser must log in again after a restart.
//! A debug build may need several seconds to hash the password. Add `--release` to start faster.

#![forbid(unsafe_code)]

use std::fmt;
use std::net::{Ipv4Addr, SocketAddr};
use std::num::NonZeroU16;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use recipe_app::blocking::AppSettings;
use recipe_app::context::AppContext;
use recipe_app::identity::{
    Argon2Hasher, Argon2Params, HashFailure, HashGate, IdentityContext, IdentitySettings,
    NONCE_BYTES, PasswordHasher as _, random_nonce,
};
use recipe_app::memory::{MemoryClock, MemoryStore};
use recipe_app::ports::NewUser;
use recipe_domain::identity::{
    KeyRing, KeyRingReason, Login, LoginReason, NewPassword, PasswordReason, Role, UserId,
};
use recipe_domain::timestamp::Timestamp;
use recipe_web::{BundleReason, WebSettings, WebState, router};
use time::OffsetDateTime;
use uuid::Uuid;

/// The variable that holds the port.
const PORT_VARIABLE: &str = "RECIPE_DEV_PORT";
/// The variable that holds the password of the three seeded users.
const PASSWORD_VARIABLE: &str = "RECIPE_DEV_PASSWORD";
/// The port when `RECIPE_DEV_PORT` is not set.
const DEFAULT_PORT: u16 = 8080;
/// The time between two moves of the clock to the wall time.
const CLOCK_STEP: Duration = Duration::from_secs(1);
/// The logins, the roles, and the identifiers of the seeded users.
const SEEDED_USERS: [(&str, Role, u128); 3] = [
    ("admin", Role::Administrator, 1),
    ("engineer", Role::ProcessEngineer, 2),
    ("operator", Role::Operator, 3),
];

/// Why the dev server does not start.
///
/// The type solves one problem: `main` must stop with a message that names the cause. No variant
/// holds a password, a hash, or a key.
#[derive(thiserror::Error)]
enum DevServerError {
    /// A required variable of the environment is not set.
    #[error("the environment variable {name} is not set")]
    MissingVariable {
        /// The name of the variable.
        name: &'static str,
    },
    /// The variable `RECIPE_DEV_PORT` does not hold a port number.
    #[error("RECIPE_DEV_PORT must hold a port number from 1 to 65535")]
    BadPort,
    /// The password does not meet the password policy.
    #[error("RECIPE_DEV_PASSWORD is not accepted because {reason}")]
    PasswordPolicy {
        /// The reason, without the password.
        reason: PasswordReason,
    },
    /// The hasher could not hash the password.
    #[error("the password hash failed: {reason}")]
    Hash {
        /// The reason of the hasher.
        reason: HashFailure,
    },
    /// The system gave no random bytes.
    #[error("the system gave no random bytes")]
    Random,
    /// A seeded login is not a valid login.
    #[error("a seeded login is not valid: {reason}")]
    Login {
        /// The reason of the login grammar.
        reason: LoginReason,
    },
    /// A signing key is not a valid key.
    #[error("a signing key is not valid: {reason}")]
    KeyRing {
        /// The reason of the key file grammar.
        reason: KeyRingReason,
    },
    /// The text bundles do not load.
    #[error("the text bundles do not load: {reason}")]
    Web {
        /// The reason of the bundle that failed.
        reason: BundleReason,
    },
    /// Another log subscriber is already set.
    #[error("the log subscriber is already set")]
    Logging,
    /// The async runtime does not start.
    #[error("cannot start the async runtime: {0}")]
    Runtime(#[source] std::io::Error),
    /// The server cannot listen on the address.
    #[error("cannot listen on {address}: {source}")]
    Bind {
        /// The address that the server tried to bind.
        address: SocketAddr,
        /// The error of the system.
        #[source]
        source: std::io::Error,
    },
    /// The server stopped with an error.
    #[error("the server stopped: {0}")]
    Serve(#[source] std::io::Error),
}

impl fmt::Debug for DevServerError {
    /// Print the message. `main` prints the `Debug` text of its error, so this text must read as a
    /// sentence. It holds no secret.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Return the address to bind: the loopback address and the port of `RECIPE_DEV_PORT`.
///
/// The function solves one problem: the dev server must never listen on another address. The
/// variable holds a port only, so no value can change the host.
fn bind_address(lookup: &impl Fn(&str) -> Option<String>) -> Result<SocketAddr, DevServerError> {
    let port = match lookup(PORT_VARIABLE) {
        None => DEFAULT_PORT,
        Some(text) => text
            .parse::<NonZeroU16>()
            .map_err(|_not_a_port| DevServerError::BadPort)?
            .get(),
    };
    Ok(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
}

/// Read the password of the seeded users from `RECIPE_DEV_PASSWORD`.
///
/// An error names the variable and the broken rule. It never holds the password.
fn dev_password(lookup: &impl Fn(&str) -> Option<String>) -> Result<NewPassword, DevServerError> {
    let text = lookup(PASSWORD_VARIABLE).ok_or(DevServerError::MissingVariable {
        name: PASSWORD_VARIABLE,
    })?;
    NewPassword::parse(&text).map_err(|reason| DevServerError::PasswordPolicy { reason })
}

/// Read a nonce of 128 bits from the system.
fn nonce() -> Result<[u8; NONCE_BYTES], DevServerError> {
    random_nonce().map_err(|_no_bytes| DevServerError::Random)
}

/// Make the text of a new signing key: 256 random bits as 64 lower-case hex characters.
///
/// The function solves one problem: a fixed key in the source would let anyone forge the cookie.
/// The key comes from the random bytes of the system and lives in memory only.
fn signing_key_text() -> Result<String, DevServerError> {
    let first = nonce()?;
    let second = nonce()?;
    Ok(first
        .iter()
        .chain(&second)
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Make a key ring with one new key.
fn fresh_key_ring() -> Result<KeyRing, DevServerError> {
    KeyRing::parse(&signing_key_text()?).map_err(|reason| DevServerError::KeyRing { reason })
}

/// Move the clock of the store to the wall time.
///
/// The function solves one problem: a clock that never moves keeps a login delay until the next
/// start and never ends a session.
fn sync_clock(clock: &MemoryClock) {
    clock.set(Timestamp::from(OffsetDateTime::now_utc()));
}

/// Keep the clock of the store at the wall time. The task never ends.
async fn follow_wall_clock(clock: MemoryClock) {
    let mut ticker = tokio::time::interval(CLOCK_STEP);
    loop {
        ticker.tick().await;
        sync_clock(&clock);
    }
}

/// Insert the primitive types and the three users into the store.
///
/// The three users share one hash, so the start hashes the password once.
fn seed_store(
    store: &MemoryStore,
    hasher: &Argon2Hasher,
    password: &NewPassword,
) -> Result<(), DevServerError> {
    store.seed_primitive_types();
    let hash = hasher
        .hash(password)
        .map_err(|reason| DevServerError::Hash { reason })?;
    for (login, role, number) in SEEDED_USERS {
        store.seed_user(&NewUser {
            id: UserId::from(Uuid::from_u128(number)),
            login: Login::parse(login).map_err(|reason| DevServerError::Login { reason })?,
            password_hash: hash.clone(),
            role,
            must_change_password: false,
        });
    }
    Ok(())
}

/// Build the state of the router over a new in-memory store.
///
/// The function wires the same adapters as the unit tests of `recipe-web`. Each call makes new
/// signing keys for the pre-session cookie and for the login HMAC.
fn build_state(
    password: &NewPassword,
    params: Argon2Params,
    clock: &MemoryClock,
) -> Result<WebState, DevServerError> {
    let settings = IdentitySettings {
        argon2: params,
        ..IdentitySettings::default()
    };
    let hasher =
        Arc::new(Argon2Hasher::new(params).map_err(|reason| DevServerError::Hash { reason })?);
    let store = MemoryStore::with_clock(clock.clone());
    seed_store(&store, &hasher, password)?;
    let app = AppContext::new(
        Arc::new(store.clone()),
        Arc::new(store.sink()),
        Arc::new(clock.clone()),
        AppSettings::default(),
    );
    let gate = HashGate::new(
        settings.argon2_permits,
        settings.argon2_acquire_timeout,
        hasher,
    );
    let identity = IdentityContext::new(
        app,
        Arc::new(store.clone()),
        Arc::new(store),
        gate,
        fresh_key_ring()?,
        settings,
    );
    WebState::new(identity, WebSettings::default(), fresh_key_ring()?)
        .map_err(|reason| DevServerError::Web { reason })
}

/// Listen on `address` and serve `app` until the process stops.
async fn run(address: SocketAddr, app: Router, clock: MemoryClock) -> Result<(), DevServerError> {
    tokio::spawn(follow_wall_clock(clock));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|source| DevServerError::Bind { address, source })?;
    tracing::info!(%address, "the dev server listens");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(DevServerError::Serve)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .try_init()
        .map_err(|_already_set| DevServerError::Logging)?;
    let lookup = |name: &str| std::env::var(name).ok();
    let address = bind_address(&lookup)?;
    let password = dev_password(&lookup)?;
    let clock = MemoryClock::new(OffsetDateTime::now_utc());
    let state = build_state(&password, Argon2Params::default(), &clock)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(DevServerError::Runtime)?;
    runtime.block_on(run(address, router(state), clock))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::{Method, Request, Response, StatusCode};
    use recipe_app::memory::seed_type_id;
    use recipe_app::ports::Clock as _;
    use recipe_domain::plant::Primitive;
    use recipe_web::issue_pre_session;
    use tower::ServiceExt as _;

    use super::*;

    const PASSWORD: &str = "correct horse battery";
    const PASSWORD_FIELD: &str = "correct+horse+battery";
    const CHEAP_ARGON2: Argon2Params = Argon2Params {
        m_cost_kib: 8,
        t_cost: 1,
        p_cost: 1,
    };
    const SESSION_COOKIE: &str = "__Host-recipe_session";
    const BODY_LIMIT: usize = 1_048_576;
    const PEER_PORT: u16 = 40_000;

    /// Build a lookup that knows one variable, so a test needs no `set_var`.
    fn lookup_of(name: &'static str, value: &str) -> impl Fn(&str) -> Option<String> {
        let value = value.to_owned();
        move |asked| (asked == name).then(|| value.clone())
    }

    fn password() -> NewPassword {
        NewPassword::parse(PASSWORD).unwrap()
    }

    // --- Bind address ----------------------------------------------------------------------------

    #[test]
    fn bind_address_defaults_to_loopback_port_8080() {
        let address = bind_address(&|_name: &str| None).unwrap();
        assert_eq!(address, SocketAddr::from((Ipv4Addr::LOCALHOST, 8080)));
    }

    #[test]
    fn bind_address_takes_the_port_of_the_variable_and_stays_on_loopback() {
        for (text, port) in [("1", 1_u16), ("80", 80), ("9090", 9090), ("65535", 65535)] {
            let address = bind_address(&lookup_of("RECIPE_DEV_PORT", text)).unwrap();
            assert_eq!(
                address,
                SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
                "value: {text:?}"
            );
        }
    }

    #[test]
    fn bind_address_refuses_a_value_that_is_not_a_port() {
        let values = [
            "",
            "abc",
            "0",
            "-1",
            "65536",
            " 80",
            "80 ",
            "0.0.0.0:8080",
            "127.0.0.1:8080",
            "8080/tcp",
        ];
        for text in values {
            let result = bind_address(&lookup_of("RECIPE_DEV_PORT", text));
            assert!(
                matches!(result, Err(DevServerError::BadPort)),
                "value: {text:?}"
            );
        }
    }

    // --- Password --------------------------------------------------------------------------------

    #[test]
    fn dev_password_reads_the_variable() {
        let password = dev_password(&lookup_of("RECIPE_DEV_PASSWORD", PASSWORD)).unwrap();
        assert_eq!(password.as_str(), PASSWORD);
    }

    #[test]
    fn missing_password_variable_stops_with_an_error_that_names_the_variable() {
        let error = dev_password(&|_name: &str| None).unwrap_err();
        assert!(matches!(
            error,
            DevServerError::MissingVariable {
                name: "RECIPE_DEV_PASSWORD"
            }
        ));
        assert!(error.to_string().contains("RECIPE_DEV_PASSWORD"), "{error}");
        assert_eq!(format!("{error:?}"), error.to_string());
    }

    #[test]
    fn a_rejected_password_never_appears_in_the_error() {
        let secret = "tiny-secret";
        let error = dev_password(&lookup_of("RECIPE_DEV_PASSWORD", secret)).unwrap_err();
        assert!(matches!(
            error,
            DevServerError::PasswordPolicy {
                reason: PasswordReason::TooShort
            }
        ));
        assert!(!format!("{error} {error:?}").contains(secret));
    }

    // --- Signing key -----------------------------------------------------------------------------

    #[test]
    fn signing_key_is_64_lower_case_hex_characters() {
        let text = signing_key_text().unwrap();
        assert_eq!(text.len(), 64);
        assert!(
            text.bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
            "{text}"
        );
    }

    #[test]
    fn two_starts_give_two_different_signing_keys() {
        assert_ne!(signing_key_text().unwrap(), signing_key_text().unwrap());
    }

    #[test]
    fn the_signing_key_is_not_a_fixed_key_of_test_support() {
        // test_support.rs holds the keys 0101..., 0202..., and 0303... as literals.
        let text = signing_key_text().unwrap();
        for fixed in ["01", "02", "03"] {
            assert_ne!(text, fixed.repeat(32));
        }
    }

    #[test]
    fn two_key_rings_sign_one_message_differently() {
        let first = fresh_key_ring().unwrap().sign(b"probe");
        let second = fresh_key_ring().unwrap().sign(b"probe");
        let fixed = KeyRing::parse(&"02".repeat(32)).unwrap().sign(b"probe");
        assert_ne!(first, second);
        assert_ne!(first, fixed);
    }

    // --- Clock -----------------------------------------------------------------------------------

    #[test]
    fn sync_clock_moves_the_clock_to_the_wall_time() {
        let clock = MemoryClock::default();
        sync_clock(&clock);
        let moved = clock.now().as_offset_date_time().unix_timestamp();
        assert!(moved.abs_diff(OffsetDateTime::now_utc().unix_timestamp()) <= 5);
    }

    // --- Seed ------------------------------------------------------------------------------------

    #[test]
    fn seed_store_inserts_every_primitive_type() {
        let store = MemoryStore::default();
        let hasher = Argon2Hasher::new(CHEAP_ARGON2).unwrap();
        seed_store(&store, &hasher, &password()).unwrap();
        let rows = [
            (Primitive::Bool, None),
            (Primitive::Int, None),
            (Primitive::Dint, None),
            (Primitive::Real, None),
            (Primitive::Lreal, None),
            (Primitive::Time, None),
            (Primitive::Ltime, None),
            (Primitive::String, Some(254)),
            (Primitive::String, Some(80)),
            (Primitive::Wstring, Some(254)),
            (Primitive::Wstring, Some(80)),
        ];
        let state = store.state();
        for (primitive, length) in rows {
            let id = seed_type_id(primitive, length).unwrap();
            assert!(state.plc_type(id).is_some(), "{primitive:?} {length:?}");
        }
    }

    // --- Users and router ------------------------------------------------------------------------

    /// Build the router of the dev server with cheap Argon2 values. Return the state too.
    fn app() -> (Router, WebState) {
        let clock = MemoryClock::new(OffsetDateTime::now_utc());
        let state = build_state(&password(), CHEAP_ARGON2, &clock).unwrap();
        let peer = SocketAddr::from((Ipv4Addr::LOCALHOST, PEER_PORT));
        (router(state.clone()).layer(MockConnectInfo(peer)), state)
    }

    /// Send `POST /login` with a valid pre-session token.
    async fn log_in(app: &(Router, WebState), login: &str, password_field: &str) -> Response<Body> {
        let (router, state) = app;
        let pre = issue_pre_session(state.csrf_keys()).unwrap();
        let cookie = pre
            .set_cookie()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let body = format!(
            "login={login}&password={password_field}&csrf_pre={}",
            pre.nonce()
        );
        let request = Request::builder()
            .method(Method::POST)
            .uri("/login")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("sec-fetch-site", "same-origin")
            .header("cookie", cookie)
            .body(Body::from(body))
            .unwrap();
        router.clone().oneshot(request).await.unwrap()
    }

    /// Return the pair `name=value` of the session cookie that a response sets.
    fn session_pair(response: &Response<Body>) -> Option<String> {
        response
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find(|text| text.starts_with(SESSION_COOKIE))
            .and_then(|text| text.split(';').next())
            .map(str::to_owned)
    }

    /// Send `GET path` in English with the session. Return the status and the body.
    async fn get_english(router: &Router, path: &str, session: &str) -> (StatusCode, String) {
        let request = Request::builder()
            .uri(path)
            .header("cookie", format!("{session}; recipe_lang=en"))
            .body(Body::empty())
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), BODY_LIMIT).await.unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[tokio::test]
    async fn each_seeded_user_logs_in_with_the_dev_password_and_holds_the_role_of_the_login() {
        let app = app();
        let users = [
            ("admin", "Administrator"),
            ("engineer", "Process engineer"),
            ("operator", "Operator"),
        ];
        for (login, label) in users {
            let response = log_in(&app, login, PASSWORD_FIELD).await;
            assert_eq!(response.status(), StatusCode::SEE_OTHER, "login: {login}");
            let session = session_pair(&response).unwrap();
            let (status, body) = get_english(&app.0, "/me", &session).await;
            assert_eq!(status, StatusCode::OK, "login: {login}");
            assert!(
                body.contains(&format!("<dd class=\"data\">{login}</dd>")),
                "{body}"
            );
            assert!(body.contains(&format!("<dd>{label}</dd>")), "{body}");
        }
    }

    #[tokio::test]
    async fn a_wrong_password_gives_no_session() {
        let app = app();
        let response = log_in(&app, "admin", "wrong+password+here").await;
        assert_ne!(response.status(), StatusCode::SEE_OTHER);
        assert!(session_pair(&response).is_none());
    }

    #[tokio::test]
    async fn the_server_seeds_no_machine() {
        let app = app();
        let response = log_in(&app, "admin", PASSWORD_FIELD).await;
        let session = session_pair(&response).unwrap();
        let (status, body) = get_english(&app.0, "/machines", &session).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("No machine matches"), "{body}");
    }
}
