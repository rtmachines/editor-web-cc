//! Audit entries, probes, and reads of the audit tables for the tests of the unit of work.
//!
//! `recipe_app` writes the audit rows. The tests read them as the superuser of the harness,
//! because `audit_owner` owns the audit tables (design D12). A test trigger on `audit_log` comes
//! from the same superuser (design D14).

use std::net::{IpAddr, Ipv4Addr};

use recipe_app::ports::NonEmpty;
use recipe_domain::audit::{AuditContext, AuditEntry, RequestId, RowAction, RowChange, SessionRef};
use recipe_domain::identity::UserId;
use recipe_domain::plant::MachineId;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use super::{Role, TestDatabase};

/// The session reference of every test entry.
const SESSION_REF: &str = "0123456789abcdef";
/// The address of every test entry.
const CLIENT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7));

/// A `MachineCreated` entry for the machine `machine`, written by `actor`.
pub fn entry(actor: Option<Uuid>, machine: Uuid) -> AuditEntry {
    entry_from(actor, machine, CLIENT_IP)
}

/// A `MachineCreated` entry for the machine `machine`, written by `actor` from `client_ip`.
pub fn entry_from(actor: Option<Uuid>, machine: Uuid, client_ip: IpAddr) -> AuditEntry {
    AuditEntry::row(
        RowAction::MachineCreated,
        recipe_domain::audit::AuditTarget::machine(MachineId::from(machine)),
        RowChange::Create {
            after: json!({"id": machine.to_string(), "name": "machine"}),
        },
        AuditContext {
            actor: actor.map(UserId::from),
            request_id: RequestId::from(Uuid::new_v4()),
            client_ip: Some(client_ip),
            session_ref: SessionRef::parse(SESSION_REF).ok(),
        },
    )
}

/// A list of `count` entries with new machine identifiers, and the identifiers in list order.
pub fn entries(actor: Option<Uuid>, count: usize) -> (NonEmpty<AuditEntry>, Vec<Uuid>) {
    let machines: Vec<Uuid> = (0..count).map(|_| Uuid::new_v4()).collect();
    let list: Vec<AuditEntry> = machines
        .iter()
        .map(|machine| entry(actor, *machine))
        .collect();
    (NonEmpty::try_from(list).unwrap(), machines)
}

/// Create a trigger on `audit_log` as the superuser of the harness.
///
/// `timing` is `BEFORE` or `AFTER`. `body` is the PL/pgSQL block between `BEGIN` and `END`.
/// The name and the text come from the test, not from input.
pub async fn install_trigger(database: &TestDatabase, timing: &str, body: &str) {
    let mut admin = database.connect(Role::Superuser).await;
    let statements = format!(
        "CREATE FUNCTION test_audit_probe() RETURNS trigger LANGUAGE plpgsql AS $probe$ \
         BEGIN {body} RETURN NEW; END $probe$; \
         CREATE TRIGGER test_audit_probe {timing} INSERT ON audit_log \
         FOR EACH ROW EXECUTE FUNCTION test_audit_probe();"
    );
    sqlx::raw_sql(&statements)
        .execute(&mut admin)
        .await
        .unwrap();
}

/// The number of rows in `audit_log`.
pub async fn audit_count(conn: &mut PgConnection) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(conn)
        .await
        .unwrap()
}

/// The `last_seq` of the chain head.
pub async fn head_seq(conn: &mut PgConnection) -> i64 {
    sqlx::query_scalar("SELECT last_seq FROM audit_chain_head WHERE singleton")
        .fetch_one(conn)
        .await
        .unwrap()
}

/// The stored columns of one audit row, as the tests compare them.
#[derive(Debug, PartialEq, sqlx::FromRow)]
pub struct StoredRow {
    pub chain_seq: i64,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub before_state: Option<serde_json::Value>,
    pub after_state: Option<serde_json::Value>,
    pub request_id: Uuid,
    pub client_ip: Option<String>,
    pub session_ref: Option<String>,
}

/// All audit rows in the order of the chain.
pub async fn stored_rows(conn: &mut PgConnection) -> Vec<StoredRow> {
    sqlx::query_as(
        "SELECT chain_seq, actor_user_id, action, entity_type, entity_id, aggregate_type, \
             aggregate_id, before_state, after_state, request_id, host(client_ip) AS client_ip, \
             session_ref \
         FROM audit_log ORDER BY chain_seq",
    )
    .fetch_all(conn)
    .await
    .unwrap()
}
