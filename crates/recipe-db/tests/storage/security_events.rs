//! Tests of `PgSecurityEventSink` (task 8.6, design D8).
//!
//! The tests go through the port `dyn SecurityEventSink` of `recipe-app`, as a use case does.
//! The adapter connects as `recipe_app`. The tests read the table as the superuser, because
//! `audit_owner` owns it.
//!
//! The domain type `SecurityEvent` has one public constructor today: `SecurityEvent::forbidden`.
//! The constructors of the kinds `login_failed`, `login_throttled`, `csrf_rejected`, and
//! `audit_chain_broken` belong to the login work of P2.5. The aggregation test therefore runs
//! the statement of `data-model.md` section 3.8 as `recipe_app`. It proves the unique index, the
//! column grant `UPDATE (count)`, and the conflict target that the sink uses.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use recipe_app::ports::{Isolation, SecurityEventSink, UnitOfWork};
use recipe_db::Database;
use recipe_db::settings::PoolSettings;
use recipe_domain::audit::{RequestId, SecurityEvent, UserAgent};
use recipe_domain::identity::UserId;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures;
use crate::support::{Role, TestDatabase};

/// The address of the events of these tests.
const CLIENT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9));

/// The aggregation statement of `data-model.md` section 3.8 for one `login_failed` event.
const AGGREGATE_LOGIN_FAILED: &str = "\
INSERT INTO security_event (kind, login_hmac, client_ip, request_id) \
VALUES ('login_failed', $1, '10.0.0.9'::inet, $2) \
ON CONFLICT (client_ip, kind, (date_trunc('minute', occurred_at AT TIME ZONE 'UTC'))) \
    WHERE kind IN ('login_failed', 'login_throttled', 'csrf_rejected') \
DO UPDATE SET count = security_event.count + 1";

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
    test_database: TestDatabase,
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
            test_database,
            admin,
        }
    }

    fn forbidden(&self, operation: &str, request: Uuid) -> SecurityEvent {
        SecurityEvent::forbidden(
            UserId::from(self.user),
            operation,
            RequestId::from(request),
            Some(CLIENT_IP),
            Some(UserAgent::from("storage-test")),
        )
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

#[tokio::test]
async fn the_aggregation_statement_gives_one_row_with_count_3_and_the_first_hmac() {
    let mut f = Fixture::create().await;
    let mut app = f.test_database.connect(Role::App).await;
    for hmac in [[1_u8; 32], [2_u8; 32], [3_u8; 32]] {
        sqlx::query(AGGREGATE_LOGIN_FAILED)
            .bind(hmac.as_slice())
            .bind(Uuid::new_v4())
            .execute(&mut app)
            .await
            .unwrap();
    }

    let rows = f.rows().await;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].count, 3);
    assert_eq!(rows[0].login_hmac, Some(vec![1_u8; 32]));
}
