//! Tests of `SessionStore::authenticate` on the in-memory store (design D10).

use std::time::Duration;

use recipe_domain::identity::Role;

use super::{IDLE, at, csrf, new_session, new_user, session_id, store, user_id};
use crate::ports::{SessionLimits, SessionStore as _};

fn limits() -> SessionLimits {
    SessionLimits {
        idle_timeout: IDLE,
        absolute_timeout: Duration::from_secs(12 * 60 * 60),
    }
}

#[tokio::test]
async fn a_valid_session_moves_the_times_and_returns_the_user_data() {
    let store = store();
    let mut user = new_user(1, "anna", Role::ProcessEngineer);
    user.must_change_password = true;
    store.seed_user(&user);
    store.seed_session(&new_session(1, user_id(1), 0));
    store.clock().advance(time::Duration::seconds(600));
    let row = store
        .authenticate(&session_id(1), &limits())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.user_id, user_id(1));
    assert_eq!(row.login.as_str(), "anna");
    assert_eq!(row.role, Role::ProcessEngineer);
    assert!(row.must_change_password);
    assert_eq!(row.epoch, 0);
    assert_eq!(row.csrf_token.to_hex(), csrf(1).to_hex());
    assert_eq!(row.created_at, at(0));
    assert_eq!(row.last_seen_at, at(600));
    let idle = i64::try_from(IDLE.as_secs()).unwrap();
    assert_eq!(row.expires_at, at(600_i64.checked_add(idle).unwrap()));
    let stored = store.state().sessions.get(&session_id(1)).unwrap().clone();
    assert_eq!(stored.last_seen_at, at(600));
    assert_eq!(stored.expires_at, row.expires_at);
}

#[tokio::test]
async fn a_missing_session_gives_none() {
    let store = store();
    let row = store.authenticate(&session_id(1), &limits()).await.unwrap();
    assert!(row.is_none());
}
