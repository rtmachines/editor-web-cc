//! Tests of `authenticate` (task 3.5, design D10).

use recipe_domain::audit::SessionRef;
use recipe_domain::identity::{Role, SessionId, SessionToken, UserId, breached_password_count};
use recipe_domain::timestamp::Timestamp;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use super::authenticate;
use crate::error::AppError;
use crate::identity::fixtures::IdentityFixture;
use crate::memory::Fault;
use crate::ports::{NewSession, NewUser};
use recipe_domain::identity::{CsrfToken, Login, PasswordHash};

const HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0$ZG91YmxlZG91YmxlZG91YmxlZG91Ymxl";
const START_SECONDS: i64 = 1_000_000;

fn token() -> SessionToken {
    SessionToken::from_bytes([7; 16])
}

fn user_id() -> UserId {
    UserId::from(Uuid::from_u128(1))
}

fn at(seconds: i64) -> Timestamp {
    let start = OffsetDateTime::from_unix_timestamp(START_SECONDS).unwrap();
    Timestamp::from(start.checked_add(Duration::seconds(seconds)).unwrap())
}

/// A fixture with the user `anna` and one session of her.
fn fixture_with_session(role: Role) -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.store.seed_user(&NewUser {
        id: user_id(),
        login: Login::parse("anna").unwrap(),
        password_hash: PasswordHash::parse(HASH).unwrap(),
        role,
        must_change_password: false,
    });
    fixture.store.seed_session(&NewSession {
        id: SessionId::of(&token()),
        user_id: user_id(),
        epoch: 0,
        csrf_token: CsrfToken::from_bytes([9; 32]),
        idle_timeout: fixture.context.settings.session_idle_timeout,
    });
    fixture
}

fn advance(fixture: &IdentityFixture, span: Duration) {
    fixture.store.clock().advance(span);
}

#[test]
fn the_context_loads_the_breached_password_list() {
    let fixture = IdentityFixture::new();
    assert!(breached_password_count() > 0);
    drop(fixture);
}

#[tokio::test]
async fn a_missing_session_gives_none() {
    let fixture = IdentityFixture::new();
    let found = authenticate(&fixture.context, &token()).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn an_idle_session_gives_none() {
    let fixture = fixture_with_session(Role::Operator);
    advance(&fixture, Duration::minutes(31));
    let found = authenticate(&fixture.context, &token()).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn a_session_older_than_twelve_hours_gives_none() {
    let fixture = fixture_with_session(Role::Operator);
    for _ in 0..35 {
        advance(&fixture, Duration::minutes(20));
        let found = authenticate(&fixture.context, &token()).await.unwrap();
        assert!(
            found.is_some(),
            "the session is 11 hours 40 minutes old at most"
        );
    }
    advance(&fixture, Duration::minutes(21));
    let found = authenticate(&fixture.context, &token()).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn an_inactive_user_gives_none() {
    let fixture = fixture_with_session(Role::Operator);
    fixture.store.update_state(|state, _now| {
        state.users.get_mut(&user_id()).unwrap().is_active = false;
    });
    let found = authenticate(&fixture.context, &token()).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn another_epoch_gives_none() {
    let fixture = fixture_with_session(Role::Operator);
    fixture.store.update_state(|state, _now| {
        state.users.get_mut(&user_id()).unwrap().session_epoch = 1;
    });
    let found = authenticate(&fixture.context, &token()).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn a_valid_session_returns_its_data_and_moves_expires_at() {
    let fixture = fixture_with_session(Role::ProcessEngineer);
    advance(&fixture, Duration::minutes(10));
    let found = authenticate(&fixture.context, &token())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.user().user(), user_id());
    assert_eq!(found.user().login().as_str(), "anna");
    assert_eq!(found.user().role(), Role::ProcessEngineer);
    assert_eq!(found.session_id(), &SessionId::of(&token()));
    assert_eq!(
        found.session_ref(),
        &SessionRef::of(&SessionId::of(&token()))
    );
    assert_eq!(found.epoch(), 0);
    assert_eq!(
        found.csrf_token().to_hex(),
        CsrfToken::from_bytes([9; 32]).to_hex()
    );
    assert!(!found.must_change_password());
    let stored = fixture
        .store
        .state()
        .sessions
        .get(&SessionId::of(&token()))
        .unwrap()
        .clone();
    assert_eq!(stored.last_seen_at, at(600));
    assert_eq!(stored.expires_at, at(2_400));
}

#[tokio::test]
async fn the_role_comes_from_the_user_row_at_each_call() {
    let fixture = fixture_with_session(Role::Operator);
    let first = authenticate(&fixture.context, &token())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.user().role(), Role::Operator);
    fixture.store.update_state(|state, _now| {
        state.users.get_mut(&user_id()).unwrap().role = Role::Administrator;
    });
    let second = authenticate(&fixture.context, &token())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.user().role(), Role::Administrator);
}

#[tokio::test]
async fn a_user_who_must_change_the_password_is_flagged() {
    let fixture = fixture_with_session(Role::Operator);
    fixture.store.update_state(|state, _now| {
        state
            .users
            .get_mut(&user_id())
            .unwrap()
            .must_change_password = true;
    });
    let found = authenticate(&fixture.context, &token())
        .await
        .unwrap()
        .unwrap();
    assert!(found.must_change_password());
}

#[tokio::test]
async fn a_storage_failure_gives_an_app_error() {
    let fixture = fixture_with_session(Role::Operator);
    fixture.store.fail_begin(Fault::Unavailable);
    let error = authenticate(&fixture.context, &token()).await.unwrap_err();
    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
}

#[tokio::test]
async fn debug_of_authenticated_hides_the_session_id_and_the_csrf_token() {
    let fixture = fixture_with_session(Role::Operator);
    let found = authenticate(&fixture.context, &token())
        .await
        .unwrap()
        .unwrap();
    let shown = format!("{found:?}");
    assert!(!shown.contains(SessionId::of(&token()).as_str()), "{shown}");
    assert!(
        !shown.contains(&CsrfToken::from_bytes([9; 32]).to_hex()),
        "{shown}"
    );
    assert!(shown.contains("[redacted]"), "{shown}");
}
