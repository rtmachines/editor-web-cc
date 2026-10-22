//! Fixtures of the contract cases of `DiffReads` (design D4, D13).
//!
//! Every seed goes through the ports and `UnitOfWork::commit` on both adapters. The reads open a
//! `RepeatableRead` transaction, as `DiffVersions` and `GetApprovalPreview` do (ADR-0050 (f)).

use recipe_app::ports::{ChangeFlags, DiffSides, Isolation, Transaction};
use recipe_domain::plant::{
    Machine, MachineId, PlatformTypes, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive,
    TypeTrees,
};
use recipe_domain::recipe::{
    DiffKind, DiffPair, DiffRows, Parameter, ParameterId, ParameterKey, RecipeVersionId,
    StoredParameter, StoredValue, diff_pairs,
};
use serde_json::Value;
use uuid::Uuid;

use crate::contract::harness::{Harness, begin, commit};
use crate::contract::recipe_seed::{
    Plant, approve, create_recipe, create_version, parameter, plant, seed, version_id,
};
use crate::contract::seed::{array_record, ensure, machine, seed_type, struct_record, type_id};

/// The offset of the identifier of the copy of a parameter on the new side.
const NEW_SIDE: u128 = 0x100;

/// The flags of one row in the order `definition`, `value`, `limits`, `translations`,
/// `only_old`, `only_new`.
pub type Flags = [bool; 6];

/// No flag set: the key is on both sides without a difference.
pub const UNCHANGED: Flags = [false; 6];
/// Only `definition`.
pub const DEFINITION: Flags = [true, false, false, false, false, false];
/// Only `value`.
pub const VALUE: Flags = [false, true, false, false, false, false];
/// Only `limits`.
pub const LIMITS: Flags = [false, false, true, false, false, false];
/// Only `translations`.
pub const TRANSLATIONS: Flags = [false, false, false, true, false, false];
/// A type change with a limit row: `definition`, `value`, and `limits`.
pub const RETYPED: Flags = [true, true, true, false, false, false];
/// A type change without a limit row: `definition` and `value`.
pub const RETYPED_BARE: Flags = [true, true, false, false, false, false];
/// A key only in the new version with a limit row.
pub const ADDED: Flags = [false, false, true, false, false, true];
/// A key only in the old version with a limit row.
pub const REMOVED: Flags = [false, false, true, false, true, false];

/// The tag, the flags, and the counts of limits, definitions, and translations of one row.
pub type Compact = (String, Flags, (u32, u32, u32));

/// The types of the cases on one machine of the platform `TwinCat3`.
#[derive(Clone, Copy, Debug)]
pub struct DiffPlant {
    /// The machine, its unit `U1`, and `DINT`.
    pub plant: Plant,
    /// The global type `INT`.
    pub int: PlcTypeId,
    /// `ST_Line` with the member `zone` of type `ST_Zone`, which has `setpoint` of type `REAL`.
    pub line: PlcTypeId,
    /// `ARRAY[1..3] OF INT`.
    pub table: PlcTypeId,
    /// `ARRAY[1..3] OF DINT`.
    pub dint_table: PlcTypeId,
}

/// Insert a struct row in its own committed transaction and return its identifier.
async fn insert_struct(h: &dyn Harness, built: &Machine, record: PlcTypeRecord) -> PlcTypeId {
    let row = PlcType::try_from(record).unwrap();
    let mut tx = begin(h).await;
    tx.types().insert_struct(&row).await.unwrap();
    commit(h, tx, built.id()).await;
    row.id()
}

/// Insert the machine number `n` with the types of the cases.
pub async fn diff_plant(h: &dyn Harness, n: u128, code: &str) -> DiffPlant {
    let plant = plant(h, n, code).await;
    let built = machine(n, code);
    let real = seed_type(h, built.id(), Primitive::Real).await;
    let zone = struct_record(
        &built,
        type_id(1),
        "ST_Zone",
        1,
        None,
        &[("setpoint", real)],
    );
    let zone = insert_struct(h, &built, zone).await;
    let line = struct_record(&built, type_id(2), "ST_Line", 1, None, &[("zone", zone)]);
    let line = insert_struct(h, &built, line).await;
    let int = seed_type(h, built.id(), Primitive::Int).await;
    let table = array_record(&built, type_id(3), int, &[(1, 3)]);
    let dint_table = array_record(&built, type_id(4), plant.dint, &[(1, 3)]);
    DiffPlant {
        plant,
        int,
        line,
        table: ensure(h, built.id(), table).await.id,
        dint_table: ensure(h, built.id(), dint_table).await.id,
    }
}

/// A parameter of `plc_type` with `value` and no limit. The number `n` sets the key.
pub fn typed(
    plant: &DiffPlant,
    (n, tag): (u128, &str),
    plc_type: PlcTypeId,
    value: Value,
) -> StoredParameter {
    StoredParameter {
        plc_type,
        value: StoredValue::from(value),
        ..parameter(&plant.plant, n, tag)
    }
}

/// Return a copy of `old` with the identifier of the new side and the same key.
pub fn on_new_side(old: &StoredParameter) -> StoredParameter {
    let id = old.id.as_uuid().as_u128().checked_add(NEW_SIDE).unwrap();
    StoredParameter {
        id: ParameterId::from(Uuid::from_u128(id)),
        ..old.clone()
    }
}

/// The approved old version and the draft new version of one recipe.
#[derive(Clone, Copy, Debug)]
pub struct Versions {
    /// The approved version 1.
    pub old: RecipeVersionId,
    /// The draft version 2. It is based on no version, so it holds only the seeded parameters.
    pub new: RecipeVersionId,
}

/// Seed the recipe number `n`: `old` into the approved version 1, `new` into the draft version 2.
pub async fn seed_versions(
    h: &dyn Harness,
    plant: &Plant,
    n: u128,
    (old, new): (&[StoredParameter], &[StoredParameter]),
) -> Versions {
    let recipe = create_recipe(h, plant, n, &format!("REF-{n}")).await;
    let old_id = recipe.versions()[0].id();
    for parameter in old {
        seed(h, old_id, parameter).await;
    }
    approve(h, old_id).await;
    let new_id = version_id(n.checked_add(1).unwrap());
    create_version(h, (recipe.id(), new_id), 2, None).await;
    for parameter in new {
        seed(h, new_id, parameter).await;
    }
    Versions {
        old: old_id,
        new: new_id,
    }
}

/// Open the transaction of a diff query.
pub async fn begin_read(h: &dyn Harness) -> Box<dyn Transaction> {
    h.unit_of_work()
        .begin(Isolation::RepeatableRead)
        .await
        .unwrap()
}

/// Read the pairs of `keys` in a transaction that nothing commits.
pub async fn read_pairs(
    h: &dyn Harness,
    old: Option<RecipeVersionId>,
    new: Option<RecipeVersionId>,
    keys: &[ParameterKey],
) -> DiffSides {
    let mut tx = begin_read(h).await;
    tx.diff_reads().pairs(old, new, keys).await.unwrap()
}

/// Return the tag, the flags, and the counts of each row.
pub fn compact(rows: &[ChangeFlags]) -> Vec<Compact> {
    rows.iter()
        .map(|row| {
            let flags = [
                row.definition,
                row.value,
                row.limits,
                row.translations,
                row.only_old,
                row.only_new,
            ];
            let counts = (row.limit_rows, row.definition_rows, row.translation_rows);
            (row.tag.to_string(), flags, counts)
        })
        .collect()
}

/// Run the domain diff on the pairs of `keys` with no cut of the rows.
fn domain_rows(
    keys: &[ParameterKey],
    sides: DiffSides,
    type_rows: Vec<PlcType>,
    (machine, platform): (MachineId, PlcPlatform),
) -> DiffRows {
    let types = PlatformTypes::collect(machine, platform, type_rows).unwrap();
    let old: Vec<Parameter> = sides.old.into_iter().map(Parameter::from).collect();
    let new: Vec<Parameter> = sides.new.into_iter().map(Parameter::from).collect();
    let ids = old.iter().chain(new.iter()).map(Parameter::plc_type);
    let trees = TypeTrees::resolve(ids, &types);
    let pairs = DiffPair::match_by_key(keys, &old, &new);
    diff_pairs(&pairs, &trees, u32::MAX)
}

/// Return the flags and the counts that the rows of the domain diff give for each flag row.
fn domain_compact(flags: &[ChangeFlags], rows: &DiffRows) -> Vec<Compact> {
    flags
        .iter()
        .map(|flag| {
            let count = |kind| {
                let section = rows.section(kind).iter();
                section.filter(|row| row.key() == flag.key).count()
            };
            let has = |kind| count(kind) > 0;
            let number = |kind| u32::try_from(count(kind)).unwrap();
            let flags = [
                has(DiffKind::Definitions),
                has(DiffKind::Values),
                has(DiffKind::Limits),
                has(DiffKind::Translations),
                has(DiffKind::Removed),
                has(DiffKind::Added),
            ];
            let counts = (
                number(DiffKind::Limits),
                number(DiffKind::Definitions),
                number(DiffKind::Translations),
            );
            (flag.tag.to_string(), flags, counts)
        })
        .collect()
}

/// Read the flags of `old` against `new` and the pairs of every key in one transaction.
///
/// The function asserts that the flags equal the rows of the domain diff on these pairs. It
/// returns the flags in the order of the adapter.
pub async fn checked_flags(
    h: &dyn Harness,
    machine: MachineId,
    old: Option<RecipeVersionId>,
    new: RecipeVersionId,
) -> Vec<ChangeFlags> {
    let mut tx = begin_read(h).await;
    let flags = tx.diff_reads().change_flags(old, new).await.unwrap();
    let keys: Vec<ParameterKey> = flags.iter().map(|row| row.key).collect();
    let sides = tx.diff_reads().pairs(old, Some(new), &keys).await.unwrap();
    let type_rows = tx.types().type_rows(machine).await.unwrap();
    let stored = tx.plant().machine_by_id(machine).await.unwrap().unwrap();
    drop(tx);
    let rows = domain_rows(&keys, sides, type_rows, (machine, stored.platform()));
    assert_eq!(
        compact(&flags),
        domain_compact(&flags, &rows),
        "domain diff"
    );
    let unchanged = flags.iter().filter(|row| row.is_unchanged()).count();
    assert_eq!(unchanged, rows.unchanged(), "unchanged keys");
    flags
}
