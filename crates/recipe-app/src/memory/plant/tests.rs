use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
};
use recipe_domain::plant::{
    Machine, MachineCode, MachineId, PlcPlatform, UnitId, UnitPath, UnitUsage,
};
use recipe_domain::recipe::{ReferenceNumber, VersionState};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{MACHINE_CODE_LIVE_UNIQUE, StorageError, UNIT_PARENT_EXISTS, UNIT_PATH_UNIQUE};
use crate::memory::{
    JournalEntry, Lock, LockMode, MemoryClock, MemoryStore, MemoryTransaction, fixtures,
};
use crate::ports::{Isolation, NonEmpty, Transaction, UnitOfWork};

const NOW_SECONDS: i64 = 1_000;

fn instant() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(NOW_SECONDS).unwrap()
}

fn now_text() -> String {
    instant().to_string()
}

fn store() -> MemoryStore {
    MemoryStore::with_clock(MemoryClock::new(instant()))
}

fn mid(n: u128) -> MachineId {
    MachineId::from(Uuid::from_u128(n))
}

fn uid(n: u128) -> UnitId {
    UnitId::from(Uuid::from_u128(n))
}

fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

fn path(value: &str) -> UnitPath {
    UnitPath::parse(value).unwrap()
}

fn machine(n: u128, code: &str) -> Machine {
    Machine::create(
        mid(n),
        MachineCode::parse(code).unwrap(),
        text("Press"),
        PlcPlatform::TwinCat3,
        (
            uid(n.checked_mul(100).unwrap().checked_add(1).unwrap()),
            text("Main"),
        ),
    )
}

fn archived_machine(n: u128, code: &str) -> Machine {
    let mut archived = machine(n, code);
    archived
        .archive(Timestamp::from(OffsetDateTime::UNIX_EPOCH), None)
        .unwrap();
    archived
}

/// Machine M1 with the units U1, U1.2 (id 150), U1.2.1 (id 120), U1.3 (id 130), U1.20 (id 140).
fn tree() -> Machine {
    let mut tree = machine(1, "M1");
    tree.add_unit(uid(150), Some(&path("U1")), "2", text("Two"))
        .unwrap();
    tree.add_unit(uid(120), Some(&path("U1.2")), "1", text("TwoOne"))
        .unwrap();
    tree.add_unit(uid(130), Some(&path("U1")), "3", text("Three"))
        .unwrap();
    tree.add_unit(uid(140), Some(&path("U1")), "20", text("Twenty"))
        .unwrap();
    tree
}

fn begin(store: &MemoryStore, isolation: Isolation) -> MemoryTransaction {
    store.begin_memory(isolation).unwrap()
}

fn lock(table: &'static str, mode: LockMode, key: String) -> JournalEntry {
    JournalEntry::Lock(Lock { table, mode, key })
}

fn locks(store: &MemoryStore) -> Vec<JournalEntry> {
    store
        .journal()
        .into_iter()
        .filter(|line| matches!(line, JournalEntry::Lock(_)))
        .collect()
}

fn machine_image(n: u128, code: &str, name: &str, archived_at: Value) -> Value {
    json!({
        "id": mid(n).to_string(),
        "code": code,
        "name": name,
        "plc_platform": "TwinCat3",
        "archived_at": archived_at,
        "created_at": now_text(),
        "updated_at": now_text(),
    })
}

fn unit_image(id: u128, machine: u128, path: &str, name: &str, archived_at: Value) -> Value {
    let unit_path = UnitPath::parse(path).unwrap();
    let parent = unit_path
        .parent()
        .map_or(Value::Null, |p| json!(p.to_string()));
    json!({
        "id": uid(id).to_string(),
        "machine_id": mid(machine).to_string(),
        "path": path,
        "parent_path": parent,
        "depth": unit_path.depth(),
        "name": name,
        "archived_at": archived_at,
        "created_at": now_text(),
        "updated_at": now_text(),
    })
}

fn audit_entry() -> NonEmpty<AuditEntry> {
    NonEmpty::new(AuditEntry::row(
        RowAction::MachineCreated,
        AuditTarget::machine(mid(1)),
        RowChange::Create {
            after: json!({ "n": 1 }),
        },
        AuditContext {
            actor: None,
            request_id: RequestId::from(Uuid::from_u128(1)),
            client_ip: None,
            session_ref: None,
        },
    ))
}

#[tokio::test]
async fn machine_by_code_finds_the_live_machine_and_skips_the_archived_one() {
    let store = store();
    store.seed_machine(&archived_machine(1, "M1"));
    store.seed_machine(&machine(2, "M1"));
    let mut tx = begin(&store, Isolation::ReadOnly);
    let found = tx
        .plant()
        .machine_by_code(MachineCode::parse("M1").unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id(), mid(2));
    let none = tx
        .plant()
        .machine_by_code(MachineCode::parse("M9").unwrap())
        .await
        .unwrap();
    assert!(none.is_none());
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn archived_machine_by_code_finds_an_archived_machine_and_skips_a_live_one() {
    let store = store();
    store.seed_machine(&archived_machine(1, "M1"));
    store.seed_machine(&machine(2, "M2"));
    let mut tx = begin(&store, Isolation::ReadOnly);
    let found = tx
        .plant()
        .archived_machine_by_code(MachineCode::parse("M1").unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id(), mid(1));
    assert!(!found.is_live());
    let live = tx
        .plant()
        .archived_machine_by_code(MachineCode::parse("M2").unwrap())
        .await
        .unwrap();
    assert!(live.is_none());
    let none = tx
        .plant()
        .archived_machine_by_code(MachineCode::parse("M9").unwrap())
        .await
        .unwrap();
    assert!(none.is_none());
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn machine_by_id_finds_an_archived_machine_with_its_units() {
    let store = store();
    store.seed_machine(&archived_machine(1, "M1"));
    let mut tx = begin(&store, Isolation::ReadOnly);
    let found = tx.plant().machine_by_id(mid(1)).await.unwrap().unwrap();
    assert!(!found.is_live());
    assert!(found.unit(&path("U1")).is_some());
    assert!(tx.plant().machine_by_id(mid(2)).await.unwrap().is_none());
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn lock_machine_shared_returns_the_state_and_the_before_image() {
    let store = store();
    store.seed_machine(&machine(1, "M1"));
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let locked = tx
        .plant()
        .lock_machine_shared(mid(1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(locked.state.id(), mid(1));
    assert_eq!(
        locked.before.as_value(),
        &machine_image(1, "M1", "Press", Value::Null)
    );
    assert_eq!(
        locks(&store),
        vec![lock("machine", LockMode::ForShare, mid(1).to_string())]
    );
}

#[tokio::test]
async fn lock_machine_for_no_key_update_journals_its_mode() {
    let store = store();
    store.seed_machine(&machine(1, "M1"));
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let locked = tx
        .plant()
        .lock_machine_for_no_key_update(mid(1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(locked.state.code(), MachineCode::parse("M1").unwrap());
    assert_eq!(
        locks(&store),
        vec![lock(
            "machine",
            LockMode::ForNoKeyUpdate,
            mid(1).to_string()
        )]
    );
}

#[tokio::test]
async fn a_lock_of_a_missing_machine_returns_none_and_takes_no_lock() {
    let store = store();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    assert!(
        tx.plant()
            .lock_machine_shared(mid(1))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        tx.plant()
            .lock_machine_for_no_key_update(mid(1))
            .await
            .unwrap()
            .is_none()
    );
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn insert_machine_returns_the_images_of_the_machine_and_of_u1() {
    let store = store();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let created = tx.plant().insert_machine(&machine(1, "M1")).await.unwrap();
    assert_eq!(
        created.machine.as_value(),
        &machine_image(1, "M1", "Press", Value::Null)
    );
    assert_eq!(created.units.len(), 1);
    assert_eq!(
        created.units.first().unwrap().as_value(),
        &unit_image(101, 1, "U1", "Main", Value::Null)
    );
    let found = tx.plant().machine_by_id(mid(1)).await.unwrap();
    assert!(found.is_some());
}

#[tokio::test]
async fn a_duplicate_live_machine_code_returns_machine_code_live_unique() {
    let store = store();
    store.seed_machine(&machine(1, "M1"));
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let error = tx
        .plant()
        .insert_machine(&machine(2, "M1"))
        .await
        .unwrap_err();
    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == MACHINE_CODE_LIVE_UNIQUE),
        "{error:?}"
    );
    assert_eq!(MACHINE_CODE_LIVE_UNIQUE, "machine_code_live_unique");
}

#[tokio::test]
async fn an_archived_machine_frees_its_code() {
    let store = store();
    store.seed_machine(&archived_machine(1, "M1"));
    let mut tx = begin(&store, Isolation::ReadCommitted);
    tx.plant().insert_machine(&machine(2, "M1")).await.unwrap();
}

#[tokio::test]
async fn rename_machine_returns_the_image_after_the_change() {
    let store = store();
    store.seed_machine(&machine(1, "M1"));
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let after = tx
        .plant()
        .rename_machine(mid(1), &text("Press 2"))
        .await
        .unwrap();
    assert_eq!(
        after.as_value(),
        &machine_image(1, "M1", "Press 2", Value::Null)
    );
    let found = tx.plant().machine_by_id(mid(1)).await.unwrap().unwrap();
    assert_eq!(found.name().as_str(), "Press 2");
}

#[tokio::test]
async fn archive_machine_sets_archived_at_to_the_clock() {
    let store = store();
    store.seed_machine(&machine(1, "M1"));
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let after = tx.plant().archive_machine(mid(1)).await.unwrap();
    assert_eq!(
        after.as_value(),
        &machine_image(1, "M1", "Press", json!(now_text()))
    );
    let found = tx.plant().machine_by_id(mid(1)).await.unwrap().unwrap();
    assert!(!found.is_live());
}

#[tokio::test]
async fn insert_unit_returns_the_image_of_the_new_unit() {
    let store = store();
    store.seed_machine(&machine(1, "M1"));
    let mut grown = machine(1, "M1");
    let unit = grown
        .add_unit(uid(150), Some(&path("U1")), "2", text("Two"))
        .unwrap()
        .clone();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let image = tx.plant().insert_unit(mid(1), &unit).await.unwrap();
    assert_eq!(
        image.as_value(),
        &unit_image(150, 1, "U1.2", "Two", Value::Null)
    );
    let found = tx.plant().machine_by_id(mid(1)).await.unwrap().unwrap();
    assert!(found.unit(&path("U1.2")).is_some());
}

#[tokio::test]
async fn a_duplicate_unit_path_returns_unit_path_unique() {
    let store = store();
    store.seed_machine(&tree());
    let mut other = machine(1, "M1");
    let unit = other
        .add_unit(uid(999), Some(&path("U1")), "2", text("Again"))
        .unwrap()
        .clone();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let error = tx.plant().insert_unit(mid(1), &unit).await.unwrap_err();
    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == UNIT_PATH_UNIQUE),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_missing_parent_returns_unit_parent_exists() {
    let store = store();
    store.seed_machine(&machine(1, "M1"));
    let mut other = machine(1, "M1");
    other
        .add_unit(uid(150), Some(&path("U1")), "2", text("Two"))
        .unwrap();
    let child = other
        .add_unit(uid(120), Some(&path("U1.2")), "1", text("TwoOne"))
        .unwrap()
        .clone();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let error = tx.plant().insert_unit(mid(1), &child).await.unwrap_err();
    assert!(
        matches!(&error, StorageError::MissingReference { constraint } if constraint == UNIT_PARENT_EXISTS),
        "{error:?}"
    );
}

#[tokio::test]
async fn lock_unit_for_update_returns_the_unit_and_its_before_image() {
    let store = store();
    store.seed_machine(&tree());
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let locked = tx
        .plant()
        .lock_unit_for_update(mid(1), &path("U1.2"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(locked.state.id(), uid(150));
    assert_eq!(
        locked.before.as_value(),
        &unit_image(150, 1, "U1.2", "Two", Value::Null)
    );
    assert_eq!(
        locks(&store),
        vec![lock("unit", LockMode::ForUpdate, uid(150).to_string())]
    );
    let missing = tx
        .plant()
        .lock_unit_for_update(mid(1), &path("U9"))
        .await
        .unwrap();
    assert!(missing.is_none());
    assert_eq!(locks(&store).len(), 1);
}

#[tokio::test]
async fn rename_unit_returns_the_image_after_the_change() {
    let store = store();
    store.seed_machine(&tree());
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let after = tx
        .plant()
        .rename_unit(uid(150), &text("Zone"))
        .await
        .unwrap();
    assert_eq!(
        after.as_value(),
        &unit_image(150, 1, "U1.2", "Zone", Value::Null)
    );
    let found = tx.plant().machine_by_id(mid(1)).await.unwrap().unwrap();
    assert_eq!(found.unit(&path("U1.2")).unwrap().name().as_str(), "Zone");
}

#[tokio::test]
async fn lock_unit_tree_returns_the_unit_and_its_descendants_ordered_by_id() {
    let store = store();
    store.seed_machine(&tree());
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let tree = tx
        .plant()
        .lock_unit_tree_for_update(mid(1), &path("U1.2"))
        .await
        .unwrap();
    let paths: Vec<String> = tree.iter().map(|l| l.state.path().to_string()).collect();
    assert_eq!(paths, vec!["U1.2.1".to_owned(), "U1.2".to_owned()]);
    let before: Vec<&Value> = tree.iter().map(|l| l.before.as_value()).collect();
    assert_eq!(
        before,
        vec![
            &unit_image(120, 1, "U1.2.1", "TwoOne", Value::Null),
            &unit_image(150, 1, "U1.2", "Two", Value::Null),
        ]
    );
    assert_eq!(
        locks(&store),
        vec![
            lock("unit", LockMode::ForUpdate, uid(120).to_string()),
            lock("unit", LockMode::ForUpdate, uid(150).to_string()),
        ]
    );
}

#[tokio::test]
async fn archive_units_returns_the_images_after_the_change() {
    let store = store();
    store.seed_machine(&tree());
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let images = tx
        .plant()
        .archive_units(&[uid(120), uid(150)])
        .await
        .unwrap();
    let values: Vec<&Value> = images.iter().map(|i| i.as_value()).collect();
    assert_eq!(
        values,
        vec![
            &unit_image(120, 1, "U1.2.1", "TwoOne", json!(now_text())),
            &unit_image(150, 1, "U1.2", "Two", json!(now_text())),
        ]
    );
    let live = tx.units().live_units(mid(1)).await.unwrap().unwrap();
    assert!(!live.is_live(uid(150)));
    assert!(live.is_live(uid(130)));
}

#[tokio::test]
async fn unit_usage_is_empty_until_a_recipe_uses_a_unit() {
    let store = store();
    store.seed_machine(&tree());
    let mut tx = begin(&store, Isolation::ReadOnly);
    let usage = tx
        .plant()
        .unit_usage(mid(1), &[uid(150), uid(120)])
        .await
        .unwrap();
    assert_eq!(usage, UnitUsage::default());
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn unit_usage_reports_the_seeded_use_of_the_asked_units() {
    let store = store();
    store.seed_machine(&tree());
    let reference = ReferenceNumber::parse("R-1").unwrap();
    let platform = PlcPlatform::TwinCat3;
    let draft = fixtures::draft_recipe((1, 1), "R-1", 11);
    store.seed_recipe(&draft, platform, fixtures::user(9));
    store.seed_parameter(fixtures::vid(11), &fixtures::param(1, 150, "A"));
    let approved = fixtures::recipe_with((2, 1), "R-2", &[(21, VersionState::Approved)]);
    store.seed_recipe(&approved, platform, fixtures::user(9));
    store.seed_parameter(fixtures::vid(21), &fixtures::param(2, 130, "A"));
    let mut tx = begin(&store, Isolation::ReadOnly);
    let usage = tx
        .plant()
        .unit_usage(mid(1), &[uid(150), uid(120)])
        .await
        .unwrap();
    let mut expected = UnitUsage::default();
    expected.add_draft(uid(150), reference);
    assert_eq!(usage, expected);
}

#[tokio::test]
async fn a_rollback_keeps_no_plant_change() {
    let store = store();
    store.seed_machine(&machine(1, "M1"));
    let mut tx = begin(&store, Isolation::ReadCommitted);
    tx.plant().insert_machine(&machine(2, "M2")).await.unwrap();
    tx.plant()
        .rename_machine(mid(1), &text("Gone"))
        .await
        .unwrap();
    drop(tx);
    let state = store.state();
    assert!(state.machine(mid(2)).is_none());
    assert_eq!(state.machine(mid(1)).unwrap().name().as_str(), "Press");
}

#[tokio::test]
async fn a_commit_keeps_the_plant_changes() {
    let store = store();
    let mut tx = begin(&store, Isolation::ReadCommitted);
    tx.plant().insert_machine(&machine(1, "M1")).await.unwrap();
    let unit_of_work: &dyn UnitOfWork = &store;
    unit_of_work
        .commit(Box::new(tx), audit_entry())
        .await
        .unwrap();
    assert!(store.state().machine(mid(1)).is_some());
}

#[tokio::test]
async fn live_units_know_the_archived_units() {
    let store = store();
    let mut grown = tree();
    grown
        .archive_unit(
            &path("U1.3"),
            Timestamp::from(OffsetDateTime::UNIX_EPOCH),
            &UnitUsage::default(),
        )
        .unwrap();
    store.seed_machine(&grown);
    let mut tx = begin(&store, Isolation::ReadOnly);
    let live = tx.units().live_units(mid(1)).await.unwrap().unwrap();
    assert!(live.is_live(uid(150)));
    assert!(!live.is_live(uid(130)));
    assert_eq!(live.path_of(uid(130)), Some(&path("U1.3")));
    assert!(tx.units().live_units(mid(2)).await.unwrap().is_none());
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn lock_unit_shared_returns_the_unit_and_journals_for_share() {
    let store = store();
    store.seed_machine(&tree());
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let unit = tx
        .units()
        .lock_unit_shared(mid(1), &path("U1.2"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unit.id(), uid(150));
    assert_eq!(
        locks(&store),
        vec![lock("unit", LockMode::ForShare, uid(150).to_string())]
    );
    let missing = tx
        .units()
        .lock_unit_shared(mid(1), &path("U9"))
        .await
        .unwrap();
    assert!(missing.is_none());
    assert_eq!(locks(&store).len(), 1);
}

#[tokio::test]
async fn lock_units_shared_orders_the_units_by_id_and_skips_unknown_ids() {
    let store = store();
    store.seed_machine(&tree());
    let mut tx = begin(&store, Isolation::ReadCommitted);
    let units = tx
        .units()
        .lock_units_shared(&[uid(150), uid(999), uid(120), uid(130)])
        .await
        .unwrap();
    let ids: Vec<UnitId> = units.iter().map(|unit| unit.id()).collect();
    assert_eq!(ids, vec![uid(120), uid(130), uid(150)]);
    assert_eq!(
        locks(&store),
        vec![
            lock("unit", LockMode::ForShare, uid(120).to_string()),
            lock("unit", LockMode::ForShare, uid(130).to_string()),
            lock("unit", LockMode::ForShare, uid(150).to_string()),
        ]
    );
}
