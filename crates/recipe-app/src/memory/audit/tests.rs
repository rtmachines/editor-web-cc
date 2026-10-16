//! Tests of the in-memory read model of the audit views (ADR-0051 (a), design D5).

use std::net::{IpAddr, Ipv4Addr};
use std::num::NonZeroU64;

use recipe_domain::audit::{
    AuditAction, AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
    SecurityEvent, SecurityEventKind,
};
use recipe_domain::identity::UserId;
use recipe_domain::recipe::RecipeId;
use recipe_domain::timestamp::Timestamp;
use serde_json::json;
use time::{Date, Month, OffsetDateTime, Time};
use uuid::Uuid;

use crate::memory::{MemoryClock, MemoryStore};
use crate::ports::{
    AuditEntryId, AuditFilter, AuditRow, AuditScope, CursorPage, Isolation, NonEmpty,
    SecurityEventFilter, SecurityEventRow, UnitOfWork,
};

/// Return the instant 2026-10-16 at `hour`:`minute`:`second` UTC.
pub(crate) fn at(hour: u8, minute: u8, second: u8) -> Timestamp {
    let date = Date::from_calendar_date(2026, Month::October, 16).unwrap();
    let time = Time::from_hms(hour, minute, second).unwrap();
    Timestamp::from(OffsetDateTime::new_utc(date, time))
}

/// Return a store whose clock starts at 10:00:00.
fn store() -> MemoryStore {
    MemoryStore::with_clock(MemoryClock::new(*at(10, 0, 0).as_offset_date_time()))
}

/// Return the audit context of a request of user `n`.
fn context(n: u128) -> AuditContext {
    AuditContext {
        actor: Some(UserId::from(Uuid::from_u128(n))),
        request_id: RequestId::from(Uuid::from_u128(n)),
        client_ip: None,
        session_ref: None,
    }
}

/// Return the entry `RecipeRenamed` of recipe `n`.
fn renamed(n: u128) -> AuditEntry {
    AuditEntry::row(
        RowAction::RecipeRenamed,
        AuditTarget::recipe(RecipeId::from(Uuid::from_u128(n))),
        RowChange::Update {
            before: json!({"name": "a"}),
            after: json!({"name": "b"}),
        },
        context(n),
    )
}

/// Commit `entries` in one transaction that begins at `instant`.
async fn commit_at(store: &MemoryStore, instant: Timestamp, entries: Vec<AuditEntry>) {
    store.clock().set(instant);
    let uow: &dyn UnitOfWork = store;
    let tx = uow.begin(Isolation::ReadCommitted).await.unwrap();
    uow.commit(tx, NonEmpty::try_from(entries).unwrap())
        .await
        .unwrap();
}

/// Read the newest ten entries of every scope.
async fn newest(store: &MemoryStore) -> CursorPage<AuditRow> {
    let mut tx = store.begin(Isolation::ReadOnly).await.unwrap();
    tx.audit_reads()
        .entries(&AuditFilter::default(), AuditScope::All, None, 10)
        .await
        .unwrap()
}

/// Read the newest ten security event rows.
async fn events(store: &MemoryStore) -> CursorPage<SecurityEventRow> {
    let mut tx = store.begin(Isolation::ReadOnly).await.unwrap();
    tx.audit_reads()
        .security_events(&SecurityEventFilter::default(), None, 10)
        .await
        .unwrap()
}

#[tokio::test]
async fn entries_get_the_ids_one_to_n_in_commit_order() {
    let store = store();
    commit_at(&store, at(10, 0, 0), vec![renamed(1), renamed(2)]).await;
    commit_at(&store, at(10, 5, 0), vec![renamed(3)]).await;

    let page = newest(&store).await;

    let ids: Vec<u64> = page.rows.iter().map(|row| row.id.get()).collect();
    assert_eq!(ids, [3, 2, 1]);
    let entities: Vec<u128> = page
        .rows
        .iter()
        .map(|row| row.entity_id.as_uuid().as_u128())
        .collect();
    assert_eq!(entities, [3, 2, 1]);
    assert!(
        page.rows
            .iter()
            .all(|row| row.action == AuditAction::RecipeRenamed)
    );
    assert!(!page.has_older);
}

#[tokio::test]
async fn the_entries_of_one_transaction_share_its_commit_time() {
    let store = store();
    commit_at(&store, at(10, 0, 0), vec![renamed(1), renamed(2)]).await;
    commit_at(&store, at(10, 5, 0), vec![renamed(3)]).await;

    let times: Vec<Timestamp> = newest(&store)
        .await
        .rows
        .iter()
        .map(|row| row.occurred_at)
        .collect();

    assert_eq!(times, [at(10, 5, 0), at(10, 0, 0), at(10, 0, 0)]);
}

#[tokio::test]
async fn a_rolled_back_transaction_takes_no_id() {
    let store = store();
    commit_at(&store, at(10, 0, 0), vec![renamed(1)]).await;
    let mut dropped = store.begin(Isolation::ReadCommitted).await.unwrap();
    dropped
        .append_audit(&NonEmpty::new(renamed(9)))
        .await
        .unwrap();
    drop(dropped);
    commit_at(&store, at(10, 1, 0), vec![renamed(2)]).await;

    let ids: Vec<u64> = newest(&store)
        .await
        .rows
        .iter()
        .map(|row| row.id.get())
        .collect();

    assert_eq!(ids, [2, 1]);
}

#[tokio::test]
async fn a_read_does_not_see_an_entry_that_commits_after_its_begin() {
    let store = store();
    commit_at(&store, at(10, 0, 0), vec![renamed(1)]).await;
    let mut tx = store.begin(Isolation::ReadOnly).await.unwrap();
    commit_at(&store, at(10, 5, 0), vec![renamed(2)]).await;

    let page = tx
        .audit_reads()
        .entries(&AuditFilter::default(), AuditScope::All, None, 10)
        .await
        .unwrap();
    let late = tx.audit_reads().entry(entry_id(2), AuditScope::All).await;

    let ids: Vec<u64> = page.rows.iter().map(|row| row.id.get()).collect();
    assert_eq!(
        ids,
        [1],
        "the list holds the committed entries of the begin"
    );
    assert!(
        late.unwrap().is_none(),
        "the entry of the later commit is absent"
    );
    assert_eq!(
        newest(&store).await.rows.len(),
        2,
        "a new read sees both entries"
    );
}

#[tokio::test]
async fn a_read_does_not_see_an_event_that_the_sink_records_after_its_begin() {
    let store = store();
    record_at(&store, at(10, 0, 0), forbidden(1, address(1))).await;
    let mut tx = store.begin(Isolation::ReadOnly).await.unwrap();
    record_at(&store, at(10, 3, 0), forbidden(2, address(1))).await;

    let page = tx
        .audit_reads()
        .security_events(&SecurityEventFilter::default(), None, 10)
        .await
        .unwrap();

    let ids: Vec<u64> = page.rows.iter().map(|row| row.id.get()).collect();
    assert_eq!(ids, [1], "the list holds the events of the begin");
    assert_eq!(
        events(&store).await.rows.len(),
        2,
        "a new read sees both events"
    );
}

/// Return the entry id `n`.
fn entry_id(n: u64) -> AuditEntryId {
    AuditEntryId::from(NonZeroU64::new(n).unwrap())
}

/// Return the address `192.0.2.<last>`.
fn address(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(192, 0, 2, last))
}

/// Return a `csrf_rejected` event of request `n` from `client`.
fn csrf(n: u128, client: IpAddr) -> SecurityEvent {
    SecurityEvent::csrf_rejected(RequestId::from(Uuid::from_u128(n)), client, None)
}

/// Return a `forbidden` event of request `n` from `client`.
fn forbidden(n: u128, client: IpAddr) -> SecurityEvent {
    SecurityEvent::forbidden(
        UserId::from(Uuid::from_u128(n)),
        "CreateMachine",
        RequestId::from(Uuid::from_u128(n)),
        Some(client),
        None,
    )
}

/// Record `event` through the sink of the store at `instant`.
async fn record_at(store: &MemoryStore, instant: Timestamp, event: SecurityEvent) {
    use crate::ports::SecurityEventSink as _;
    store.clock().set(instant);
    store.sink().record(event).await.unwrap();
}

#[tokio::test]
async fn security_events_get_an_id_and_the_time_of_the_store_clock() {
    let store = store();
    record_at(&store, at(10, 0, 0), forbidden(1, address(1))).await;
    record_at(&store, at(10, 3, 0), forbidden(2, address(1))).await;

    let page = events(&store).await;

    let rows: Vec<(u64, Timestamp, u32)> = page
        .rows
        .iter()
        .map(|row| (row.id.get(), row.occurred_at, row.count))
        .collect();
    assert_eq!(rows, [(2, at(10, 3, 0), 1), (1, at(10, 0, 0), 1)]);
    assert!(!page.has_older);
}

#[tokio::test]
async fn csrf_events_aggregate_per_address_kind_and_minute() {
    let store = store();
    record_at(&store, at(10, 0, 5), csrf(1, address(1))).await;
    record_at(&store, at(10, 0, 10), csrf(2, address(2))).await;
    record_at(&store, at(10, 0, 50), csrf(3, address(1))).await;
    record_at(&store, at(10, 1, 0), csrf(4, address(1))).await;

    let rows: Vec<(u64, Timestamp, u32, u128)> = events(&store)
        .await
        .rows
        .iter()
        .map(|row| {
            let request = row.request_id.as_uuid().as_u128();
            (row.id.get(), row.occurred_at, row.count, request)
        })
        .collect();

    assert_eq!(
        rows,
        [
            (4, at(10, 1, 0), 1, 4),
            (2, at(10, 0, 10), 1, 2),
            (1, at(10, 0, 5), 2, 1),
        ]
    );
}

#[tokio::test]
async fn forbidden_events_of_one_minute_stay_separate_rows() {
    let store = store();
    record_at(&store, at(10, 0, 5), forbidden(1, address(1))).await;
    record_at(&store, at(10, 0, 6), forbidden(2, address(1))).await;

    let counts: Vec<u32> = events(&store)
        .await
        .rows
        .iter()
        .map(|row| row.count)
        .collect();

    assert_eq!(counts, [1, 1]);
}

#[tokio::test]
async fn the_sink_of_the_store_keeps_a_flat_list_of_the_recorded_events() {
    let store = store();
    record_at(&store, at(10, 0, 5), csrf(1, address(1))).await;
    record_at(&store, at(10, 0, 6), csrf(2, address(1))).await;

    assert_eq!(
        store.sink().events(),
        vec![csrf(1, address(1)), csrf(2, address(1))]
    );
    assert_eq!(
        events(&store).await.rows.first().map(|row| row.kind),
        Some(SecurityEventKind::CsrfRejected)
    );
}
