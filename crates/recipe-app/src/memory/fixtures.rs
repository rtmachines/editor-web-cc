//! Fixtures of the tests of the recipe and draft adapters.

mod actors;
mod context;
mod parameter;
mod plant;
mod race;
pub(crate) mod scenarios;
mod seed;
#[cfg(test)]
mod tests;

pub(crate) use actors::{actor, meta, second_engineer};
pub(crate) use parameter::parameter_on;
pub(crate) use plant::machine_on;
pub(crate) use race::RaceStore;
pub(crate) use seed::Fixture;

use recipe_domain::identity::UserId;
use recipe_domain::plant::{
    LeafPath, Machine, MachineCode, MachineId, PlcPlatform, PlcTypeId, Primitive, UnitId, UnitPath,
};
use recipe_domain::recipe::{
    LangCode, Ordinal, ParameterId, ParameterKey, ParameterTag, Recipe, RecipeId, RecipeVersionId,
    ReferenceNumber, StoredLimit, StoredParameter, StoredRecipe, StoredValue, StoredVersionHeader,
    Translation, VersionState,
};
use recipe_domain::text::{LongText, Text};
use recipe_domain::timestamp::Timestamp;
use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{JournalEntry, Lock, LockMode, MemoryClock, MemoryStore, seed_type_id};

/// The instant of the clock in seconds after the epoch.
pub(crate) const NOW_SECONDS: i64 = 1_000;

/// The offset between the number of a parameter and the number of its key.
const KEY_OFFSET: u128 = 10_000;

pub(crate) fn instant() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(NOW_SECONDS).unwrap()
}

pub(crate) fn timestamp() -> Timestamp {
    Timestamp::from(instant())
}

pub(crate) fn now_text() -> String {
    instant().to_string()
}

pub(crate) fn uuid(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

pub(crate) fn mid(n: u128) -> MachineId {
    MachineId::from(uuid(n))
}

pub(crate) fn uid(n: u128) -> UnitId {
    UnitId::from(uuid(n))
}

pub(crate) fn tid(n: u128) -> PlcTypeId {
    PlcTypeId::from(uuid(n))
}

pub(crate) fn rid(n: u128) -> RecipeId {
    RecipeId::from(uuid(n))
}

pub(crate) fn vid(n: u128) -> RecipeVersionId {
    RecipeVersionId::from(uuid(n))
}

pub(crate) fn pid(n: u128) -> ParameterId {
    ParameterId::from(uuid(n))
}

pub(crate) fn pkey(n: u128) -> ParameterKey {
    ParameterKey::from(uuid(n.checked_add(KEY_OFFSET).unwrap()))
}

pub(crate) fn user(n: u128) -> UserId {
    UserId::from(uuid(n))
}

pub(crate) fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

pub(crate) fn long_text(value: &str) -> LongText {
    LongText::parse(value).unwrap()
}

pub(crate) fn reference(value: &str) -> ReferenceNumber {
    ReferenceNumber::parse(value).unwrap()
}

pub(crate) fn lang(value: &str) -> LangCode {
    LangCode::parse(value).unwrap()
}

/// A store with the primitive types and the machine M1.
///
/// M1 has the units U1 (id 101), U1.2 (id 102), and U1.3 (id 103).
pub(crate) fn store() -> MemoryStore {
    let store = MemoryStore::with_clock(MemoryClock::new(instant()));
    store.seed_primitive_types();
    let mut machine = machine(1, "M1");
    let root = UnitPath::parse("U1").unwrap();
    machine
        .add_unit(uid(102), Some(&root), "2", text("Two"))
        .unwrap();
    machine
        .add_unit(uid(103), Some(&root), "3", text("Three"))
        .unwrap();
    store.seed_machine(&machine);
    store
}

/// A `TwinCat3` machine with the unit U1 whose id is `n * 100 + 1`.
pub(crate) fn machine(n: u128, code: &str) -> Machine {
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

/// A recipe of machine `m` whose versions have the given numbers, states, and identifiers.
///
/// The headers have the numbers 1 to the length of `versions`, in the order of the slice.
pub(crate) fn recipe_with(
    (r, m): (u128, u128),
    reference_text: &str,
    versions: &[(u128, VersionState)],
) -> Recipe {
    let headers = (1_u32..)
        .zip(versions)
        .map(|(number, (v, state))| StoredVersionHeader {
            id: vid(*v),
            number,
            state: *state,
            row_version: 1,
        })
        .collect();
    Recipe::try_from(StoredRecipe {
        id: rid(r),
        machine: mid(m),
        reference: reference(reference_text),
        name: text("Recipe"),
        deleted_at: None,
        versions: headers,
    })
    .unwrap()
}

/// A recipe of machine `m` with one live draft.
pub(crate) fn draft_recipe((r, m): (u128, u128), reference_text: &str, v: u128) -> Recipe {
    Recipe::create(
        rid(r),
        mid(m),
        reference(reference_text),
        text("Recipe"),
        vid(v),
    )
}

/// The limit row of the parameter itself with the bounds 0 and 10.
pub(crate) fn limit() -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some("0".parse().unwrap()),
        max: Some("10".parse().unwrap()),
        max_length: None,
    }
}

/// The Polish translation of a parameter.
pub(crate) fn translation_pl() -> Translation {
    Translation {
        lang: lang("pl"),
        name: text("Nazwa"),
        description: Some(long_text("Opis")),
    }
}

/// An `INT` parameter on the unit `unit` with one limit row and a Polish translation.
pub(crate) fn param(n: u128, unit: u128, tag: &str) -> StoredParameter {
    StoredParameter {
        id: pid(n),
        key: pkey(n),
        unit: uid(unit),
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: text("Name"),
        description_en: long_text("Description"),
        translations: vec![translation_pl()],
        plc_type: seed_type_id(Primitive::Int, None).unwrap(),
        s7_address: None,
        limits: vec![limit()],
        value: StoredValue::from(json!(5)),
        ordinal: Ordinal::parse(0).unwrap(),
    }
}

/// Return the lock lines of the journal.
pub(crate) fn locks(store: &MemoryStore) -> Vec<JournalEntry> {
    store
        .journal()
        .into_iter()
        .filter(|line| matches!(line, JournalEntry::Lock(_)))
        .collect()
}

/// Build a lock line of the journal.
pub(crate) fn lock_line(table: &'static str, mode: LockMode, key: String) -> JournalEntry {
    JournalEntry::Lock(Lock { table, mode, key })
}

/// Return the field `name` of an image, or `Null` when the image has no such key.
pub(crate) fn field(image: &Value, name: &str) -> Value {
    image.get(name).cloned().unwrap_or(Value::Null)
}
