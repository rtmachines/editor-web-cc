use std::collections::BTreeSet;

use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, LeafClass, Machine, MachineCode, MachineId, MemberName,
    PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, TypeMember, TypeName,
    UnitId,
};
use recipe_domain::text::Text;
use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{PLC_TYPE_NAMED_UNIQUE, StorageError};
use crate::memory::{
    DERIVED_TYPE_LOCK_TABLE, JournalEntry, Lock, LockMode, MemoryClock, MemoryStore,
    MemoryTransaction, seed_type_id,
};
use crate::ports::{DerivedTypeLock, Isolation, Transaction};

const NOW_SECONDS: i64 = 1_000;

fn instant() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(NOW_SECONDS).unwrap()
}

fn now_text() -> String {
    instant().to_string()
}

fn mid(n: u128) -> MachineId {
    MachineId::from(Uuid::from_u128(n))
}

fn tid(n: u128) -> PlcTypeId {
    PlcTypeId::from(Uuid::from_u128(n))
}

fn twincat() -> BTreeSet<PlcPlatform> {
    BTreeSet::from([PlcPlatform::TwinCat3])
}

fn machine(n: u128, code: &str) -> Machine {
    Machine::create(
        mid(n),
        MachineCode::parse(code).unwrap(),
        Text::parse("Press").unwrap(),
        PlcPlatform::TwinCat3,
        (
            UnitId::from(Uuid::from_u128(
                n.checked_mul(100).unwrap().checked_add(1).unwrap(),
            )),
            Text::parse("Main").unwrap(),
        ),
    )
}

/// A store with the global primitive rows and the machines M1 and M2.
fn store() -> MemoryStore {
    let store = MemoryStore::with_clock(MemoryClock::new(instant()));
    store.seed_primitive_types();
    store.seed_machine(&machine(1, "M1"));
    store.seed_machine(&machine(2, "M2"));
    store
}

fn begin(store: &MemoryStore) -> MemoryTransaction {
    store.begin_memory(Isolation::ReadCommitted).unwrap()
}

fn int() -> PlcTypeId {
    seed_type_id(Primitive::Int, None).unwrap()
}

fn real() -> PlcTypeId {
    seed_type_id(Primitive::Real, None).unwrap()
}

fn string_record(id: u128, machine: u128, length: u16, size: u32) -> PlcTypeRecord {
    PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(machine)),
        platforms: twincat(),
        shape: RecordShape::Primitive {
            primitive: Primitive::String,
            leaf_class: LeafClass::String,
            size_bytes: size,
            string_length: Some(length),
        },
    }
}

fn array_record(id: u128, machine: u128, element: PlcTypeId) -> PlcTypeRecord {
    PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(machine)),
        platforms: twincat(),
        shape: RecordShape::Array {
            element,
            dimensions: ArrayDimensions::parse(vec![ArrayDimension::parse(1, 3).unwrap()]).unwrap(),
        },
    }
}

fn struct_record(
    id: u128,
    machine: u128,
    name: &str,
    revision: u32,
    members: &[(&str, PlcTypeId)],
) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(machine)),
        platforms: twincat(),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision,
            supersedes: None,
            members: members
                .iter()
                .map(|(member, type_id)| TypeMember {
                    name: MemberName::parse(member).unwrap(),
                    type_id: *type_id,
                })
                .collect(),
        },
    })
    .unwrap()
}

fn lock(key: String) -> JournalEntry {
    JournalEntry::Lock(Lock {
        table: DERIVED_TYPE_LOCK_TABLE,
        mode: LockMode::Advisory,
        key,
    })
}

#[tokio::test]
async fn type_rows_give_the_global_rows_and_the_rows_of_the_machine() {
    let store = store();
    store.seed_type(&PlcType::try_from(string_record(501, 1, 20, 22)).unwrap());
    store.seed_type(&PlcType::try_from(string_record(502, 2, 20, 22)).unwrap());
    let mut tx = begin(&store);
    let rows = tx.types().type_rows(mid(1)).await.unwrap();
    let ids: BTreeSet<PlcTypeId> = rows.iter().map(PlcType::id).collect();
    assert!(ids.contains(&tid(501)));
    assert!(!ids.contains(&tid(502)));
    let global = rows.iter().filter(|row| row.machine().is_none()).count();
    assert_eq!(global, 11);
    assert_eq!(rows.len(), 12);
    assert!(ids.contains(&int()));
}

#[tokio::test]
async fn the_seed_has_the_string_defaults_of_the_platforms() {
    let store = store();
    let mut tx = begin(&store);
    let rows = tx.types().type_rows(mid(1)).await.unwrap();
    let find = |primitive: Primitive, length: u16| {
        let id = seed_type_id(primitive, Some(length)).unwrap();
        rows.iter()
            .find(|row| row.id() == id)
            .unwrap()
            .platforms()
            .clone()
    };
    assert_eq!(
        find(Primitive::String, 254),
        BTreeSet::from([
            PlcPlatform::S7_300,
            PlcPlatform::S7_1200,
            PlcPlatform::S7_1500
        ])
    );
    assert_eq!(
        find(Primitive::Wstring, 80),
        BTreeSet::from([PlcPlatform::CodesysIec, PlcPlatform::TwinCat3])
    );
}

#[tokio::test]
async fn the_second_ensure_of_one_shape_returns_the_first_id_and_no_image() {
    let store = store();
    let mut tx = begin(&store);
    let first = tx
        .types()
        .ensure_derived(string_record(501, 1, 20, 22))
        .await
        .unwrap();
    assert_eq!(first.id, tid(501));
    let image = first.created.unwrap();
    assert_eq!(
        image.as_value(),
        &json!({
            "id": tid(501).to_string(),
            "kind": "primitive",
            "name": "STRING",
            "platforms": ["TwinCat3"],
            "size_bytes": 22,
            "string_length": 20,
            "leaf_class": "string",
            "element_type_id": Value::Null,
            "dim_lower": Value::Null,
            "dim_upper": Value::Null,
            "machine_id": mid(1).to_string(),
            "revision": 1,
            "supersedes_type_id": Value::Null,
            "created_at": now_text(),
        })
    );
    let second = tx
        .types()
        .ensure_derived(string_record(502, 1, 20, 22))
        .await
        .unwrap();
    assert_eq!(second.id, tid(501));
    assert!(second.created.is_none());
}

#[tokio::test]
async fn the_same_string_on_another_machine_creates_a_second_row() {
    let store = store();
    let mut tx = begin(&store);
    tx.types()
        .ensure_derived(string_record(501, 1, 20, 22))
        .await
        .unwrap();
    let other = tx
        .types()
        .ensure_derived(string_record(502, 2, 20, 22))
        .await
        .unwrap();
    assert_eq!(other.id, tid(502));
    assert!(other.created.is_some());
}

#[tokio::test]
async fn ensure_derived_reuses_an_array_row_of_the_same_shape() {
    let store = store();
    let mut tx = begin(&store);
    let first = tx
        .types()
        .ensure_derived(array_record(601, 1, int()))
        .await
        .unwrap();
    let image = first.created.unwrap();
    assert_eq!(image.as_value().get("kind"), Some(&json!("array")));
    assert_eq!(image.as_value().get("name"), Some(&Value::Null));
    assert_eq!(image.as_value().get("dim_lower"), Some(&json!([1])));
    assert_eq!(image.as_value().get("dim_upper"), Some(&json!([3])));
    assert_eq!(
        image.as_value().get("element_type_id"),
        Some(&json!(int().to_string()))
    );
    let again = tx
        .types()
        .ensure_derived(array_record(602, 1, int()))
        .await
        .unwrap();
    assert_eq!(again.id, tid(601));
    assert!(again.created.is_none());
    let other_element = tx
        .types()
        .ensure_derived(array_record(603, 1, real()))
        .await
        .unwrap();
    assert_eq!(other_element.id, tid(603));
}

#[tokio::test]
async fn ensure_derived_rejects_a_struct_row() {
    let store = store();
    let mut tx = begin(&store);
    let record = PlcTypeRecord {
        id: tid(700),
        machine: Some(mid(1)),
        platforms: twincat(),
        shape: RecordShape::Struct {
            name: TypeName::parse("ST_A").unwrap(),
            revision: 1,
            supersedes: None,
            members: vec![TypeMember {
                name: MemberName::parse("a").unwrap(),
                type_id: int(),
            }],
        },
    };
    let error = tx.types().ensure_derived(record).await.unwrap_err();
    assert!(
        matches!(error, StorageError::Constraint { .. }),
        "{error:?}"
    );
}

#[tokio::test]
async fn insert_struct_returns_an_image_with_the_members_in_order() {
    let store = store();
    let mut tx = begin(&store);
    let row = struct_record(800, 1, "ST_A", 1, &[("zeta", int()), ("alpha", real())]);
    let image = tx.types().insert_struct(&row).await.unwrap();
    assert_eq!(
        image.as_value(),
        &json!({
            "id": tid(800).to_string(),
            "kind": "struct",
            "name": "ST_A",
            "platforms": ["TwinCat3"],
            "size_bytes": Value::Null,
            "string_length": Value::Null,
            "leaf_class": Value::Null,
            "element_type_id": Value::Null,
            "dim_lower": Value::Null,
            "dim_upper": Value::Null,
            "machine_id": mid(1).to_string(),
            "revision": 1,
            "supersedes_type_id": Value::Null,
            "created_at": now_text(),
            "members": [
                {
                    "struct_type_id": tid(800).to_string(),
                    "created_at": now_text(),
                    "ordinal": 1,
                    "member_name": "zeta",
                    "member_type_id": int().to_string(),
                },
                {
                    "struct_type_id": tid(800).to_string(),
                    "created_at": now_text(),
                    "ordinal": 2,
                    "member_name": "alpha",
                    "member_type_id": real().to_string(),
                },
            ],
        })
    );
    let rows = tx.types().type_rows(mid(1)).await.unwrap();
    assert!(rows.iter().any(|stored| stored.id() == tid(800)));
}

#[tokio::test]
async fn insert_struct_of_an_existing_name_and_revision_returns_plc_type_named_unique() {
    let store = store();
    let mut tx = begin(&store);
    let first = struct_record(800, 1, "ST_A", 1, &[("a", int())]);
    tx.types().insert_struct(&first).await.unwrap();
    let again = struct_record(801, 1, "ST_A", 1, &[("a", int())]);
    let error = tx.types().insert_struct(&again).await.unwrap_err();
    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == PLC_TYPE_NAMED_UNIQUE),
        "{error:?}"
    );
    let next_revision = struct_record(802, 1, "ST_A", 2, &[("a", int())]);
    tx.types().insert_struct(&next_revision).await.unwrap();
    let other_machine = struct_record(803, 2, "ST_A", 1, &[("a", int())]);
    tx.types().insert_struct(&other_machine).await.unwrap();
}

#[tokio::test]
async fn insert_struct_with_an_unknown_member_type_returns_a_missing_reference() {
    let store = store();
    let mut tx = begin(&store);
    let row = struct_record(800, 1, "ST_A", 1, &[("a", tid(12_345))]);
    let error = tx.types().insert_struct(&row).await.unwrap_err();
    assert!(
        matches!(error, StorageError::MissingReference { .. }),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_rolled_back_type_insert_keeps_nothing() {
    let store = store();
    let mut tx = begin(&store);
    tx.types()
        .ensure_derived(string_record(501, 1, 20, 22))
        .await
        .unwrap();
    drop(tx);
    let mut tx = begin(&store);
    let rows = tx.types().type_rows(mid(1)).await.unwrap();
    assert_eq!(rows.len(), 11);
}

#[tokio::test]
async fn lock_derived_types_records_one_advisory_entry_per_key_in_order() {
    let store = store();
    let mut tx = begin(&store);
    let keys = [
        DerivedTypeLock::Struct(TypeName::parse("ST_A").unwrap()),
        DerivedTypeLock::String,
        DerivedTypeLock::Array,
    ];
    tx.types().lock_derived_types(mid(1), &keys).await.unwrap();
    let locks: Vec<JournalEntry> = store
        .journal()
        .into_iter()
        .filter(|line| matches!(line, JournalEntry::Lock(_)))
        .collect();
    assert_eq!(
        locks,
        vec![
            lock(format!("{}:ST_A", mid(1))),
            lock(format!("{}:STRING", mid(1))),
            lock(format!("{}:ARRAY", mid(1))),
        ]
    );
}

#[test]
fn derived_type_locks_sort_struct_names_then_string_wstring_array() {
    let name = |text: &str| DerivedTypeLock::Struct(TypeName::parse(text).unwrap());
    let mut keys = vec![
        DerivedTypeLock::Array,
        DerivedTypeLock::Wstring,
        name("ST_B"),
        DerivedTypeLock::String,
        name("ST_A"),
    ];
    keys.sort();
    assert_eq!(
        keys,
        vec![
            name("ST_A"),
            name("ST_B"),
            DerivedTypeLock::String,
            DerivedTypeLock::Wstring,
            DerivedTypeLock::Array,
        ]
    );
}

#[test]
fn derived_type_lock_key_text_is_the_name_or_the_keyword() {
    let name = DerivedTypeLock::Struct(TypeName::parse("ST_A").unwrap());
    assert_eq!(name.key_text(), "ST_A");
    assert_eq!(DerivedTypeLock::String.key_text(), "STRING");
    assert_eq!(DerivedTypeLock::Wstring.key_text(), "WSTRING");
    assert_eq!(DerivedTypeLock::Array.key_text(), "ARRAY");
}
