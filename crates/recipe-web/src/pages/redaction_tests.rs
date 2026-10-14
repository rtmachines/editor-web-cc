//! Redaction tests of the full router (task 6.11, design D18).
//!
//! Each test runs one scenario and collects every secret that the scenario makes. At the end the
//! captured log must hold none of them. A response body may hold only the one-time password of
//! its own command, the CSRF token of its own session, and its own `csrf_pre` nonce.

use axum::body::Body;
use axum::http::{Method, StatusCode};
use recipe_domain::identity::{Role, SessionId, SessionToken};

use crate::csrf::issue_pre_session;
use crate::test_support::{
    CaptureGuard, Fixture, LogBuffer, Reply, TestSession, call, capture_logs, cookie_pair,
    get_with, post_fields, request,
};

const PASSWORD: &str = "correct horse battery";
const WRONG: &str = "wrong password here";
const NEW_PASSWORD: &str = "another long passphrase";
const GHOST: &str = "ghostlogin7";
const HTMX: (&str, &str) = ("hx-request", "true");
const PRE_COOKIE: &str = "__Host-recipe_csrf";

/// Replace the spaces of a password with the plus signs of a form body.
fn form(text: &str) -> String {
    text.replace(' ', "+")
}

/// One scenario: a fixture, the captured log, the secrets, and the bodies to check.
struct Run {
    fixture: Fixture,
    logs: LogBuffer,
    _guard: CaptureGuard,
    secrets: Vec<String>,
    bodies: Vec<(String, String, Vec<String>)>,
}

impl Run {
    fn new() -> Self {
        let fixture = Fixture::new();
        let (logs, guard) = capture_logs();
        let mut run = Self {
            fixture,
            logs,
            _guard: guard,
            secrets: Vec::new(),
            bodies: Vec::new(),
        };
        for text in [PASSWORD, WRONG, NEW_PASSWORD, GHOST, "$argon2id$"] {
            run.learn(text);
            run.learn(&form(text));
        }
        run
    }

    fn learn(&mut self, secret: &str) {
        if !secret.is_empty() && !self.secrets.iter().any(|known| known == secret) {
            self.secrets.push(secret.to_owned());
        }
    }

    /// Learn the cookie value, the session id, and the CSRF token of a seeded session.
    fn learn_session(&mut self, session: &TestSession) {
        self.learn_cookie_value(&pair_value(&session.cookie));
        self.learn(&session.csrf);
    }

    /// Learn a cookie value. A session token also gives the session id.
    fn learn_cookie_value(&mut self, value: &str) {
        self.learn(value);
        if let Ok(token) = SessionToken::parse(value) {
            self.learn(SessionId::of(&token).as_str());
        }
    }

    /// Record a reply: learn its cookies and keep its body for the final check.
    ///
    /// `csrf` is the CSRF token of the session of the request. `otp` is the one-time password
    /// that the command of the request makes. Both may appear in the body.
    fn record(&mut self, label: &str, reply: &Reply, csrf: Option<&str>, otp: Option<&str>) {
        let mut allowed: Vec<String> = Vec::new();
        for text in reply.set_cookies() {
            let value = pair_value(&cookie_pair(&text));
            self.learn_cookie_value(&value);
            if text.starts_with(&format!("{PRE_COOKIE}=")) {
                let nonce = value.split('.').next().unwrap_or_default().to_owned();
                self.learn(&nonce);
                allowed.push(nonce);
            }
        }
        for own in [csrf, otp].into_iter().flatten() {
            self.learn(own);
            allowed.push(own.to_owned());
        }
        self.bodies
            .push((label.to_owned(), reply.body.clone(), allowed));
    }

    /// Send `POST /login` with a pre-session token that the run knows.
    async fn post_login(&mut self, fields: &str) -> Reply {
        let token = issue_pre_session(self.fixture.state.csrf_keys()).unwrap();
        let pair = cookie_pair(token.set_cookie().to_str().unwrap());
        self.learn(&pair_value(&pair));
        let nonce = token.nonce().to_string();
        self.learn(&nonce);
        let body = format!("{fields}&csrf_pre={nonce}");
        let builder = request(Method::POST, "/login")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("sec-fetch-site", "same-origin")
            .header("cookie", pair);
        call(
            &self.fixture.router(),
            builder.body(Body::from(body)).unwrap(),
        )
        .await
    }

    async fn post_as(&self, session: &TestSession, path: &str, body: &str) -> Reply {
        post_fields(&self.fixture.router(), path, Some(session), &[HTMX], body).await
    }

    /// Read the CSRF token of the session behind `cookie` from `GET /me`.
    async fn csrf_of(&mut self, label: &str, cookie: &str) -> String {
        let reply = get_with(&self.fixture.router(), "/me", &[("cookie", cookie)]).await;
        assert_eq!(reply.status, StatusCode::OK, "{label}: {}", reply.body);
        let marker = "\"X-CSRF-Token\":\"";
        let (_before, rest) = reply.body.split_once(marker).unwrap();
        let csrf = rest.split('"').next().unwrap().to_owned();
        self.record(label, &reply, Some(&csrf), None);
        csrf
    }

    /// Assert that no body holds a foreign secret and that the log holds no secret.
    fn finish(&self) {
        for (label, body, allowed) in &self.bodies {
            for secret in &self.secrets {
                if allowed.contains(secret) {
                    continue;
                }
                assert!(
                    !body.contains(secret.as_str()),
                    "{label}: body holds a secret"
                );
            }
        }
        let text = self.logs.text();
        for secret in &self.secrets {
            assert!(!text.contains(secret.as_str()), "the log holds a secret");
        }
    }
}

/// Return the value of a `name=value` pair.
fn pair_value(pair: &str) -> String {
    pair.split_once('=')
        .map(|(_name, value)| value.to_owned())
        .unwrap_or_default()
}

/// Return the one-time password that a page shows.
fn one_time_password(body: &str) -> String {
    let (_before, rest) = body.split_once("id=\"one-time-password\">").unwrap();
    rest.split('<').next().unwrap().to_owned()
}

// --- Scenarios ----------------------------------------------------------------------------------

#[tokio::test]
async fn redaction_good_login_leaves_no_secret_in_the_log() {
    let mut run = Run::new();
    run.fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let reply = run
        .post_login(&format!("login=user1&password={}", form(PASSWORD)))
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    run.record("login", &reply, None, None);
    let cookie = cookie_pair(
        &reply
            .set_cookies()
            .into_iter()
            .find(|text| text.starts_with("__Host-recipe_session="))
            .unwrap(),
    );
    run.csrf_of("me", &cookie).await;
    run.finish();
}

#[tokio::test]
async fn redaction_bad_login_of_an_existing_user_leaves_no_secret_in_the_log() {
    let mut run = Run::new();
    run.fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let reply = run
        .post_login(&format!("login=user1&password={}", form(WRONG)))
        .await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.body);
    run.record("bad login", &reply, None, None);
    assert!(run.logs.text().contains("E-IDN"), "the log shows the code");
    run.finish();
}

#[tokio::test]
async fn redaction_login_of_an_unknown_account_leaves_no_typed_login_in_the_log() {
    let mut run = Run::new();
    let reply = run
        .post_login(&format!("login={GHOST}&password={}", form(WRONG)))
        .await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.body);
    run.record("unknown login", &reply, None, None);
    assert!(run.logs.text().contains("E-IDN"), "the log shows the code");
    run.finish();
}

#[tokio::test]
async fn redaction_logout_leaves_no_secret_in_the_log() {
    let mut run = Run::new();
    let session = run.fixture.seed_session(1, Role::Operator, false);
    run.learn_session(&session);
    let reply = run.post_as(&session, "/logout", "").await;
    run.record("logout", &reply, Some(&session.csrf), None);
    assert!(
        reply.status == StatusCode::SEE_OTHER || reply.status == StatusCode::OK,
        "{}",
        reply.body
    );
    run.finish();
}

#[tokio::test]
async fn redaction_csrf_rejection_leaves_no_token_in_the_log() {
    let mut run = Run::new();
    let session = run.fixture.seed_session(1, Role::Administrator, false);
    run.learn_session(&session);
    let forged = "ab".repeat(32);
    run.learn(&forged);
    let builder = request(Method::POST, "/users")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", session.cookie.as_str())
        .header("x-csrf-token", forged.as_str());
    let reply = call(
        &run.fixture.router(),
        builder
            .body(Body::from("login=newuser&role=operator"))
            .unwrap(),
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-007"), "{}", reply.body);
    run.record("csrf with a session", &reply, Some(&session.csrf), None);
    // A login with a pre-session nonce that no cookie backs.
    let nonce = "cd".repeat(16);
    run.learn(&nonce);
    let body = format!("login=user1&password={}&csrf_pre={nonce}", form(WRONG));
    let builder = request(Method::POST, "/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin");
    let reply = call(
        &run.fixture.router(),
        builder.body(Body::from(body)).unwrap(),
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    run.record("csrf without a session", &reply, None, None);
    assert!(
        run.logs.text().contains("E-WEB-007"),
        "the log shows the code"
    );
    run.finish();
}

#[tokio::test]
async fn redaction_password_change_leaves_no_password_in_the_log() {
    let mut run = Run::new();
    run.fixture.seed_account(1, Role::Operator, PASSWORD, false);
    let session = run.fixture.session_of(1);
    run.learn_session(&session);
    let wrong = format!(
        "current_password={}&password={}",
        form(WRONG),
        form(NEW_PASSWORD)
    );
    let refused = run.post_as(&session, "/me/password", &wrong).await;
    run.record(
        "wrong current password",
        &refused,
        Some(&session.csrf),
        None,
    );
    assert!(run.logs.text().contains("E-IDN"), "the log shows the code");
    let right = format!(
        "current_password={}&password={}",
        form(PASSWORD),
        form(NEW_PASSWORD)
    );
    let reply = run.post_as(&session, "/me/password", &right).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    run.record("password change", &reply, Some(&session.csrf), None);
    run.finish();
}

#[tokio::test]
async fn redaction_password_reset_leaves_no_one_time_password_in_the_log() {
    let mut run = Run::new();
    let admin = run.fixture.seed_session(1, Role::Administrator, false);
    run.fixture.seed_account(2, Role::Operator, PASSWORD, false);
    let target = run.fixture.session_of(2);
    run.learn_session(&admin);
    run.learn_session(&target);
    let reply = run.post_as(&admin, "/users/user2/reset-password", "").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let otp = one_time_password(&reply.body);
    run.record("reset", &reply, Some(&admin.csrf), Some(&otp));
    let login = run.post_login(&format!("login=user2&password={otp}")).await;
    assert_eq!(login.status, StatusCode::SEE_OTHER, "{}", login.body);
    run.record("login with the one-time password", &login, None, None);
    run.finish();
}

#[tokio::test]
async fn redaction_create_user_leaves_no_one_time_password_in_the_log() {
    let mut run = Run::new();
    let admin = run.fixture.seed_session(1, Role::Administrator, false);
    run.learn_session(&admin);
    let reply = run
        .post_as(&admin, "/users", "login=newuser&role=operator")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let otp = one_time_password(&reply.body);
    run.record("create user", &reply, Some(&admin.csrf), Some(&otp));
    let login = run
        .post_login(&format!("login=newuser&password={otp}"))
        .await;
    assert_eq!(login.status, StatusCode::SEE_OTHER, "{}", login.body);
    run.record("login with the one-time password", &login, None, None);
    run.finish();
}
