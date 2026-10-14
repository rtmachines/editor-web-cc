//! Tests of the use case `Logout` (task 4.2, `data-model.md` section 6).

use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::{AuditAction, AuditTarget, RequestId, SessionRef};
use recipe_domain::identity::SessionId;
use serde_json::json;
use uuid::Uuid;

use super::Logout;
use crate::auth::{Operation, RequestMeta};
use crate::context::UseCase;
use crate::error::AppError;
use crate::identity::authenticate;
use crate::identity::fixtures::{IdentityFixture, anna_id};
use crate::memory::Fault;

const PASSWORD: &str = "correct-horse-battery";

fn meta(session: &SessionId) -> RequestMeta {
    RequestMeta::new(
        RequestId::from(Uuid::from_u128(0x88)),
        Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))),
        None,
        Some(SessionRef::of(session)),
    )
}

fn fixture() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.seed_anna(PASSWORD, false);
    fixture
}

#[test]
fn the_operation_is_logout() {
    assert_eq!(<Logout as UseCase>::OPERATION, Operation::Logout);
}

#[tokio::test]
async fn logout_deletes_the_current_session_and_keeps_the_others() {
    let fixture = fixture();
    let current = fixture.seed_anna_session(1);
    let other = fixture.seed_anna_session(2);
    let auth = fixture.authenticated(&current).await;
    Logout::new(fixture.context.clone())
        .execute(&auth, &meta(auth.session_id()))
        .await
        .unwrap();
    let state = fixture.store.state();
    assert!(!state.sessions.contains_key(&SessionId::of(&current)));
    assert!(state.sessions.contains_key(&SessionId::of(&other)));
}

#[tokio::test]
async fn the_session_cannot_authenticate_after_the_logout() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    Logout::new(fixture.context.clone())
        .execute(&auth, &meta(auth.session_id()))
        .await
        .unwrap();
    assert!(
        authenticate(&fixture.context, &token)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn logout_writes_user_logged_out_with_the_session_ref_in_before() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    Logout::new(fixture.context.clone())
        .execute(&auth, &meta(auth.session_id()))
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::UserLoggedOut);
    assert_eq!(entry.target(), &AuditTarget::app_user(anna_id()));
    assert_eq!(entry.actor(), Some(anna_id()));
    assert_eq!(
        entry.before(),
        Some(&json!({"session_ref": auth.session_ref().as_str()}))
    );
    assert_eq!(entry.after(), None);
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn a_failed_commit_keeps_the_session_and_writes_no_entry() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    fixture.store.fail_finish(Fault::Unavailable);
    let error = Logout::new(fixture.context.clone())
        .execute(&auth, &meta(auth.session_id()))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
    assert!(
        fixture
            .store
            .state()
            .sessions
            .contains_key(&SessionId::of(&token))
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_failed_audit_write_gives_e_aud_001_and_keeps_the_session() {
    let fixture = fixture();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    fixture.store.fail_append_audit(Fault::Unavailable);
    let error = Logout::new(fixture.context.clone())
        .execute(&auth, &meta(auth.session_id()))
        .await
        .unwrap_err();
    assert!(
        matches!(error, AppError::AuditWriteFailed { .. }),
        "{error:?}"
    );
    assert!(
        fixture
            .store
            .state()
            .sessions
            .contains_key(&SessionId::of(&token))
    );
}
