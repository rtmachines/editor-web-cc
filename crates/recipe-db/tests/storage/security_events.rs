//! Tests of `PgSecurityEventSink` (task 8.6, design D8).
//!
//! The tests go through the port `dyn SecurityEventSink` of `recipe-app`, as a use case does.
//! The adapter connects as `recipe_app`. The tests read the table as the superuser, because
//! `audit_owner` owns it.
//!
//! Since `0005` the sink never names `occurred_at` or `count` (ADR-0035). Every kind goes through
//! its domain constructor and the port, as a use case does. A test of the next minute is absent:
//! it would need a wait of up to 60 seconds, and the clock of the database is not controllable.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use recipe_app::ports::{Isolation, SecurityEventSink, UnitOfWork};
use recipe_db::Database;
use recipe_db::settings::PoolSettings;
use recipe_domain::audit::{LoginHmac, LoginSubject, RequestId, SecurityEvent, UserAgent};
use recipe_domain::identity::UserId;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures;
use crate::support::{Role, TestDatabase, wait_for_early_second};

/// The address of the events of these tests.
const CLIENT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9));
/// A second address, for the test that each address has its own row.
const OTHER_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 10));
const AGENT: &str = "storage-test";

/// One stored row, as the tests compare it.
#[derive(Debug, PartialEq, sqlx::FromRow)]
struct StoredEvent {
    kind: String,
    login_hmac: Option<Vec<u8>>,
    user_id: Option<Uuid>,
    client_ip: Option<String>,
    user_agent: Option<String>,
    request_id: Uuid,
    detail: Option<serde_json::Value>,
    count: i32,
}

/// A test database, a user, the adapters, and a session of the superuser.
struct Fixture {
    user: Uuid,
    unit_of_work: Arc<dyn UnitOfWork>,
    sink: Arc<dyn SecurityEventSink>,
    admin: PgConnection,
}

impl Fixture {
    async fn create() -> Self {
        let test_database = TestDatabase::create().await;
        let mut migrator = test_database.connect(Role::Migrator).await;
        let user = fixtures::user(&mut migrator).await;
        let database =
            Database::connect(&test_database.target(Role::App), &PoolSettings::default())
                .await
                .unwrap();
        let admin = test_database.connect(Role::Superuser).await;
        Self {
            user,
            unit_of_work: database.unit_of_work(),
            sink: database.security_events(),
            admin,
        }
    }

    fn forbidden(&self, operation: &str, request: Uuid) -> SecurityEvent {
        SecurityEvent::forbidden(
            UserId::from(self.user),
            operation,
            RequestId::from(request),
            Some(CLIENT_IP),
            Some(UserAgent::from(AGENT)),
        )
    }

    fn user_id(&self) -> UserId {
        UserId::from(self.user)
    }

    async fn rows(&mut self) -> Vec<StoredEvent> {
        sqlx::query_as(
            "SELECT kind, login_hmac, user_id, host(client_ip) AS client_ip, user_agent, \
                 request_id, detail, count FROM security_event ORDER BY id",
        )
        .fetch_all(&mut self.admin)
        .await
        .unwrap()
    }
}

#[tokio::test]
async fn an_event_survives_the_rollback_of_a_command_transaction() {
    let mut f = Fixture::create().await;
    let request = Uuid::new_v4();
    let tx = f
        .unit_of_work
        .begin(Isolation::ReadCommitted)
        .await
        .unwrap();

    f.sink
        .record(f.forbidden("RenameMachine", request))
        .await
        .unwrap();
    drop(tx);

    let rows = f.rows().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].request_id, request);
}

#[tokio::test]
async fn a_forbidden_event_is_one_row_with_all_its_columns() {
    let mut f = Fixture::create().await;
    let request = Uuid::new_v4();

    f.sink
        .record(f.forbidden("RenameMachine", request))
        .await
        .unwrap();

    let expected = StoredEvent {
        kind: "forbidden".to_owned(),
        login_hmac: None,
        user_id: Some(f.user),
        client_ip: Some("10.0.0.9".to_owned()),
        user_agent: Some("storage-test".to_owned()),
        request_id: request,
        detail: Some(json!({"operation": "RenameMachine"})),
        count: 1,
    };
    assert_eq!(f.rows().await, vec![expected]);
}

#[tokio::test]
async fn a_mapped_ipv4_address_is_stored_in_its_ipv4_form() {
    let mut f = Fixture::create().await;
    let mapped = IpAddr::V6(Ipv4Addr::new(10, 0, 0, 5).to_ipv6_mapped());
    let event = SecurityEvent::forbidden(
        f.user_id(),
        "RenameMachine",
        RequestId::from(Uuid::new_v4()),
        Some(mapped),
        Some(UserAgent::from(AGENT)),
    );

    f.sink.record(event).await.unwrap();

    let rows = f.rows().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].client_ip.as_deref(), Some("10.0.0.5"));
}

#[tokio::test]
async fn two_forbidden_events_of_one_address_in_one_minute_are_two_rows() {
    let mut f = Fixture::create().await;

    f.sink
        .record(f.forbidden("RenameMachine", Uuid::new_v4()))
        .await
        .unwrap();
    f.sink
        .record(f.forbidden("ArchiveMachine", Uuid::new_v4()))
        .await
        .unwrap();

    let rows = f.rows().await;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.count == 1));
}

fn agent() -> Option<UserAgent> {
    Some(UserAgent::from(AGENT))
}

fn login_failed_of(hmac: u8, ip: IpAddr) -> SecurityEvent {
    SecurityEvent::login_failed(
        LoginSubject::Unknown(LoginHmac::from([hmac; 32])),
        RequestId::from(Uuid::new_v4()),
        ip,
        agent(),
    )
}

#[tokio::test]
async fn three_login_failed_events_of_one_address_in_one_minute_are_one_row_with_count_3() {
    let mut f = Fixture::create().await;
    wait_for_early_second(&mut f.admin).await;

    for hmac in [1, 2, 3] {
        f.sink
            .record(login_failed_of(hmac, CLIENT_IP))
            .await
            .unwrap();
    }

    let rows = f.rows().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "login_failed");
    assert_eq!(rows[0].count, 3);
    assert_eq!(rows[0].login_hmac, Some(vec![1_u8; 32]));
}

#[tokio::test]
async fn login_failed_events_of_two_addresses_are_two_rows() {
    let mut f = Fixture::create().await;
    wait_for_early_second(&mut f.admin).await;

    f.sink.record(login_failed_of(1, CLIENT_IP)).await.unwrap();
    f.sink.record(login_failed_of(1, OTHER_IP)).await.unwrap();

    let rows = f.rows().await;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.count == 1));
}

#[tokio::test]
async fn a_login_failed_event_of_an_existing_account_holds_the_user_and_no_hmac() {
    let mut f = Fixture::create().await;
    let request = Uuid::new_v4();
    let event = SecurityEvent::login_failed(
        LoginSubject::User(f.user_id()),
        RequestId::from(request),
        CLIENT_IP,
        agent(),
    );

    f.sink.record(event).await.unwrap();

    let expected = StoredEvent {
        kind: "login_failed".to_owned(),
        login_hmac: None,
        user_id: Some(f.user),
        client_ip: Some("10.0.0.9".to_owned()),
        user_agent: Some(AGENT.to_owned()),
        request_id: request,
        detail: None,
        count: 1,
    };
    assert_eq!(f.rows().await, vec![expected]);
}

#[tokio::test]
async fn a_login_throttled_row_holds_the_hmac_bytes() {
    let mut f = Fixture::create().await;
    let request = Uuid::new_v4();
    let hmac = LoginHmac::from([5; 32]);

    f.sink
        .record(SecurityEvent::login_throttled(
            hmac,
            RequestId::from(request),
            CLIENT_IP,
            agent(),
        ))
        .await
        .unwrap();

    let expected = StoredEvent {
        kind: "login_throttled".to_owned(),
        login_hmac: Some(vec![5_u8; 32]),
        user_id: None,
        client_ip: Some("10.0.0.9".to_owned()),
        user_agent: Some(AGENT.to_owned()),
        request_id: request,
        detail: None,
        count: 1,
    };
    assert_eq!(f.rows().await, vec![expected]);
}

#[tokio::test]
async fn a_csrf_rejected_row_has_no_hmac_and_no_user() {
    let mut f = Fixture::create().await;
    let request = Uuid::new_v4();

    f.sink
        .record(SecurityEvent::csrf_rejected(
            RequestId::from(request),
            CLIENT_IP,
            agent(),
        ))
        .await
        .unwrap();

    let expected = StoredEvent {
        kind: "csrf_rejected".to_owned(),
        login_hmac: None,
        user_id: None,
        client_ip: Some("10.0.0.9".to_owned()),
        user_agent: Some(AGENT.to_owned()),
        request_id: request,
        detail: None,
        count: 1,
    };
    assert_eq!(f.rows().await, vec![expected]);
}

#[tokio::test]
async fn an_unlock_event_is_one_row_with_count_1_and_the_actor_in_detail() {
    let mut f = Fixture::create().await;
    let actor = UserId::from(Uuid::new_v4());

    f.sink
        .record(SecurityEvent::unlock(
            f.user_id(),
            actor,
            RequestId::from(Uuid::new_v4()),
            CLIENT_IP,
            agent(),
        ))
        .await
        .unwrap();

    let rows = f.rows().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "unlock");
    assert_eq!(rows[0].user_id, Some(f.user));
    assert_eq!(rows[0].count, 1);
    assert_eq!(
        rows[0].detail,
        Some(json!({"unlocked_by": actor.as_uuid().to_string()}))
    );
}

#[tokio::test]
async fn an_audit_chain_broken_row_has_no_client_address_and_holds_the_request_id_of_the_run() {
    let mut f = Fixture::create().await;
    let request = Uuid::new_v4();

    f.sink
        .record(SecurityEvent::audit_chain_broken(RequestId::from(request)))
        .await
        .unwrap();

    let expected = StoredEvent {
        kind: "audit_chain_broken".to_owned(),
        login_hmac: None,
        user_id: None,
        client_ip: None,
        user_agent: None,
        request_id: request,
        detail: None,
        count: 1,
    };
    assert_eq!(f.rows().await, vec![expected]);
}

#[tokio::test]
async fn two_audit_chain_broken_events_stay_two_rows() {
    let mut f = Fixture::create().await;

    for _ in 0..2 {
        f.sink
            .record(SecurityEvent::audit_chain_broken(RequestId::from(
                Uuid::new_v4(),
            )))
            .await
            .unwrap();
    }

    let rows = f.rows().await;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.count == 1));
}

#[tokio::test]
async fn a_login_rejected_inactive_event_is_one_row_with_count_1() {
    let mut f = Fixture::create().await;

    f.sink
        .record(SecurityEvent::login_rejected_inactive(
            f.user_id(),
            RequestId::from(Uuid::new_v4()),
            CLIENT_IP,
            agent(),
        ))
        .await
        .unwrap();

    let rows = f.rows().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "login_rejected_inactive");
    assert_eq!(rows[0].user_id, Some(f.user));
    assert_eq!(rows[0].login_hmac, None);
    assert_eq!(rows[0].count, 1);
}

#[tokio::test]
async fn two_unlock_and_two_inactive_events_stay_four_rows() {
    let mut f = Fixture::create().await;
    wait_for_early_second(&mut f.admin).await;

    for _ in 0..2 {
        f.sink
            .record(SecurityEvent::unlock(
                f.user_id(),
                f.user_id(),
                RequestId::from(Uuid::new_v4()),
                CLIENT_IP,
                agent(),
            ))
            .await
            .unwrap();
        f.sink
            .record(SecurityEvent::login_rejected_inactive(
                f.user_id(),
                RequestId::from(Uuid::new_v4()),
                CLIENT_IP,
                agent(),
            ))
            .await
            .unwrap();
    }

    let rows = f.rows().await;
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|row| row.count == 1));
}
