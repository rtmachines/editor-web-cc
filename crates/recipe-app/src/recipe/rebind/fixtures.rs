//! Rows, parameters, and requests that the tests of `RebindTypeRevision` share.
//!
//! The tests of the other draft commands also use the typed struct rows, the leaf texts, and
//! the error lines of this file.

use std::collections::BTreeSet;

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, LeafClass, LeafPath, MemberName, PlcPlatform, PlcType,
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, S7Address, TypeMember, TypeName,
};
use recipe_domain::recipe::{
    LeafTexts, MemberValue, RecipeError, StoredLimit, StoredParameter, StoredValue, ValueInput,
};
use serde_json::{Value, json};

use super::{RebindTypeRevision, RebindTypeRevisionInput};
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, parameter_on, pkey, reference, tid, uid, vid,
};
use crate::memory::seed_type_id;
use crate::ports::{Isolation, Transaction};
use crate::recipe::DraftOutput;
use crate::recipe::test_support::{DRAFT_VERSION, stored_parameters};

/// `ST_Zone@1` with the member `setpoint`.
pub(super) const ZONE_1: u128 = 901;
/// `ST_Zone@2` with the members `setpoint` and `extra`.
pub(super) const ZONE_2: u128 = 902;
/// `ARRAY[1..4] OF ST_Zone@1`.
pub(super) const ZONES_1: u128 = 903;
/// `ST_Other@2`, a revision of another struct.
pub(super) const OTHER_2: u128 = 905;
/// `ST_Drop@1` with the members `setpoint` and `old`.
pub(super) const DROP_1: u128 = 906;
/// `ST_Drop@2` with the member `setpoint`.
pub(super) const DROP_2: u128 = 907;
/// `ARRAY[1..2] OF ST_Zone@1`.
pub(super) const PAIR_1: u128 = 909;

fn int() -> PlcTypeId {
    seed_type_id(Primitive::Int, None).unwrap()
}

/// The identifier of the seed row of `primitive` without a string length.
pub(in crate::recipe) fn primitive_id(primitive: Primitive) -> PlcTypeId {
    seed_type_id(primitive, None).unwrap()
}

/// A struct row of the machine 1 with `INT` members.
pub(super) fn struct_row(
    platform: PlcPlatform,
    header: (u128, &str, u32),
    supersedes: Option<u128>,
    members: &[&str],
) -> PlcType {
    let typed: Vec<(&str, PlcTypeId)> = members.iter().map(|name| (*name, int())).collect();
    typed_struct_row(platform, header, supersedes, &typed)
}

/// A struct row of the machine 1 with the given member names and member types.
pub(in crate::recipe) fn typed_struct_row(
    platform: PlcPlatform,
    (id, name, revision): (u128, &str, u32),
    supersedes: Option<u128>,
    members: &[(&str, PlcTypeId)],
) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision,
            supersedes: supersedes.map(tid),
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

/// A global `CHAR` row of `TwinCat3`. The seed of the memory store has no `CHAR`.
pub(in crate::recipe) fn char_row(id: u128) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: None,
        platforms: BTreeSet::from([PlcPlatform::TwinCat3]),
        shape: RecordShape::Primitive {
            primitive: Primitive::Char,
            leaf_class: LeafClass::Char,
            size_bytes: 1,
            string_length: None,
        },
    })
    .unwrap()
}

/// The value input of a form: one text per leaf path, in the given order.
pub(in crate::recipe) fn leaf_texts(pairs: &[(&str, &str)]) -> ValueInput {
    let pairs = pairs
        .iter()
        .map(|(path, text)| (LeafPath::parse(path).unwrap(), (*text).to_owned()))
        .collect();
    ValueInput::Texts(LeafTexts::parse(pairs).unwrap())
}

/// The code and the quoted leaf path of each entry of a list error, in the order of the list.
///
/// An error that is not a list gives no line, so an assertion on the lines fails.
pub(in crate::recipe) fn error_lines(error: &AppError) -> Vec<String> {
    let AppError::Recipes(list) = error else {
        return Vec::new();
    };
    list.iter()
        .map(|entry| format!("{} '{}'", entry.code(), entry_leaf(entry)))
        .collect()
}

/// The leaf path of one list entry, or an empty text for an entry without a leaf.
fn entry_leaf(entry: &RecipeError) -> String {
    match entry {
        RecipeError::ValueShapeMismatch { leaf, .. }
        | RecipeError::ValueOutOfLimit { leaf, .. }
        | RecipeError::LimitOutsideTypeRange { leaf, .. }
        | RecipeError::LeafPathNotFound { leaf, .. }
        | RecipeError::MinGreaterThanMax { leaf, .. }
        | RecipeError::LimitNotApplicable { leaf, .. }
        | RecipeError::RebindNeedsValue { leaf, .. }
        | RecipeError::RebindDropsLimit { leaf, .. } => leaf.to_string(),
        _ => String::new(),
    }
}

/// An array row of the machine 1 with the bounds `1..=upper`.
pub(super) fn array_row(
    platform: PlcPlatform,
    id: u128,
    element: PlcTypeId,
    upper: i32,
) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Array {
            element,
            dimensions: ArrayDimensions::parse(vec![ArrayDimension::parse(1, upper).unwrap()])
                .unwrap(),
        },
    })
    .unwrap()
}

/// Store the rows of `ST_Zone`, `ST_Other`, and `ST_Drop` and the arrays of `ST_Zone@1`.
pub(super) fn seed_rows(fixture: &Fixture, platform: PlcPlatform) {
    let rows = [
        struct_row(platform, (ZONE_1, "ST_Zone", 1), None, &["setpoint"]),
        struct_row(
            platform,
            (ZONE_2, "ST_Zone", 2),
            Some(ZONE_1),
            &["setpoint", "extra"],
        ),
        array_row(platform, ZONES_1, tid(ZONE_1), 4),
        struct_row(platform, (904, "ST_Other", 1), None, &["setpoint"]),
        struct_row(platform, (OTHER_2, "ST_Other", 2), Some(904), &["setpoint"]),
        struct_row(platform, (DROP_1, "ST_Drop", 1), None, &["setpoint", "old"]),
        struct_row(
            platform,
            (DROP_2, "ST_Drop", 2),
            Some(DROP_1),
            &["setpoint"],
        ),
        array_row(platform, PAIR_1, tid(ZONE_1), 2),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
}

/// Store a parameter `n` of the draft on the unit 101 with the given type, value, and limits.
pub(super) fn seed_parameter(
    fixture: &Fixture,
    platform: PlcPlatform,
    (n, tag): (u128, &str),
    (plc_type, value): (u128, Value),
    limits: Vec<StoredLimit>,
) -> StoredParameter {
    let parameter = StoredParameter {
        plc_type: tid(plc_type),
        value: StoredValue::from(value),
        limits,
        ..parameter_on(platform, n, tag, uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &parameter);
    parameter
}

/// A draft of a `TwinCat3` machine with the rows of [`seed_rows`] and no parameter.
pub(super) fn empty_draft() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    seed_rows(&fixture, PlcPlatform::TwinCat3);
    fixture
}

/// A draft with the parameter `ZONE1` of the type `ST_Zone@1`.
pub(super) fn zone_draft() -> Fixture {
    let fixture = empty_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::TwinCat3,
        (11, "ZONE1"),
        (ZONE_1, json!({"setpoint": 1})),
        Vec::new(),
    );
    fixture
}

/// The value of an array of `count` zones of `ST_Zone@1`.
pub(super) fn zones_value(count: usize) -> Value {
    Value::Array(vec![json!({"setpoint": 1}); count])
}

/// A draft with the parameter `ZONES` of the type `ARRAY[1..4] OF ST_Zone@1`.
pub(super) fn zones_draft() -> Fixture {
    let fixture = empty_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::TwinCat3,
        (12, "ZONES"),
        (ZONES_1, zones_value(4)),
        Vec::new(),
    );
    fixture
}

/// The request to move from `from` to `to` on the draft that the fixtures store.
pub(super) fn request(from: u128, to: u128) -> RebindTypeRevisionInput {
    RebindTypeRevisionInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        from: tid(from),
        to: tid(to),
        new_member_values: std::collections::BTreeMap::new(),
    }
}

/// The same request with the JSON value `value` for the leaf `path` of the parameter `n`.
pub(super) fn with_value(
    input: RebindTypeRevisionInput,
    at: (u128, &str),
    value: &Value,
) -> RebindTypeRevisionInput {
    with_member(input, at, MemberValue::Json(value.clone()))
}

/// The same request with the form text `text` for the leaf `path` of the parameter `n`.
pub(super) fn with_text(
    input: RebindTypeRevisionInput,
    at: (u128, &str),
    text: &str,
) -> RebindTypeRevisionInput {
    with_member(input, at, MemberValue::Text(text.to_owned()))
}

/// The same request with `value` for the leaf `path` of the parameter `n`.
fn with_member(
    mut input: RebindTypeRevisionInput,
    (n, path): (u128, &str),
    value: MemberValue,
) -> RebindTypeRevisionInput {
    input
        .new_member_values
        .insert((pkey(n), LeafPath::parse(path).unwrap()), value);
    input
}

/// The request `ST_Zone@1` to `ST_Zone@2` with a value for `extra` of the parameter `ZONE1`.
pub(super) fn zone_request() -> RebindTypeRevisionInput {
    with_value(request(ZONE_1, ZONE_2), (11, ".extra"), &json!(7))
}

/// The request `ST_Zone@1` to `ST_Zone@2` with a value for `extra` of each zone of `ZONES`.
pub(super) fn zones_request() -> RebindTypeRevisionInput {
    (1..=4).fold(request(ZONE_1, ZONE_2), |input, index| {
        with_value(input, (12, &format!("[{index}].extra")), &json!(7))
    })
}

/// The rows that belong to the machine 1. The global rows stay out.
pub(super) async fn machine_rows(fixture: &Fixture) -> Vec<PlcType> {
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let rows = tx.types().type_rows(mid(1)).await.unwrap();
    rows.into_iter()
        .filter(|row| row.machine().is_some())
        .collect()
}

/// The array rows of the machine 1 whose element is `element`.
pub(super) async fn arrays_of(fixture: &Fixture, element: u128) -> Vec<PlcType> {
    machine_rows(fixture)
        .await
        .into_iter()
        .filter(|row| matches!(row.shape(), RecordShape::Array { element: found, .. } if *found == tid(element)))
        .collect()
}

/// The stored parameter `n` of the draft.
pub(super) fn stored(fixture: &Fixture, n: u128) -> StoredParameter {
    stored_parameters(fixture)
        .into_iter()
        .find(|row| row.key == pkey(n))
        .unwrap()
}

/// `ST_Big@1` with one member.
pub(super) const BIG_1: u128 = 930;
/// `ST_Big@2` with a second member.
pub(super) const BIG_2: u128 = 931;
/// `ARRAY[1..8192] OF ST_Big@1`.
pub(super) const BIG_ARRAY: u128 = 932;
/// `ARRAY[1..16384] OF INT`.
pub(super) const FILL: u128 = 933;

/// A draft one leaf below the leaf budget: 15 fillers, `BIG` with 8 192 elements, and `SMALL`.
///
/// The rebind of `ST_Big@1` to `ST_Big@2` adds 8 192 leaves and breaks the budget by one leaf.
pub(in crate::recipe) fn near_budget_draft() -> Fixture {
    let fixture = empty_draft();
    let platform = PlcPlatform::TwinCat3;
    let rows = [
        struct_row(platform, (BIG_1, "ST_Big", 1), None, &["a"]),
        struct_row(platform, (BIG_2, "ST_Big", 2), Some(BIG_1), &["a", "b"]),
        array_row(platform, BIG_ARRAY, tid(BIG_1), 8_192),
        array_row(platform, FILL, int(), 16_384),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
    for n in 100..115 {
        seed_parameter(
            &fixture,
            platform,
            (n, &format!("FILL_{n}")),
            (FILL, Value::Array(vec![json!(0); 16_384])),
            Vec::new(),
        );
    }
    seed_parameter(
        &fixture,
        platform,
        (40, "BIG"),
        (BIG_ARRAY, Value::Array(vec![json!({"a": 0}); 8_192])),
        Vec::new(),
    );
    let small = parameter_on(platform, 41, "SMALL", uid(101), 0);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &small);
    fixture
}

/// The request `ST_Big@1` to `ST_Big@2` with a value for `b` of each element of `BIG`.
pub(super) fn big_request() -> RebindTypeRevisionInput {
    (1..=8_192).fold(request(BIG_1, BIG_2), |input, index| {
        with_value(input, (40, &format!("[{index}].b")), &json!(0))
    })
}

/// A draft of an `S7_300` machine. `ZONES` sits at `DB1.DBB0` and `NEXT` follows it at `DB1.DBW8`.
///
/// `ZONES` has 4 elements of 2 bytes. A rebind to `ST_Zone@2` doubles the size and overlaps `NEXT`.
pub(super) fn s7_zones_draft() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    seed_rows(&fixture, PlcPlatform::S7_300);
    let zones = StoredParameter {
        plc_type: tid(ZONES_1),
        value: StoredValue::from(zones_value(4)),
        limits: Vec::new(),
        s7_address: Some(S7Address::parse("DB1.DBB0").unwrap()),
        ..parameter_on(PlcPlatform::S7_300, 12, "ZONES", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &zones);
    let next = parameter_on(PlcPlatform::S7_300, 13, "NEXT", uid(101), 8);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &next);
    fixture
}

/// Run the use case as a process engineer.
pub(super) async fn run(
    fixture: &Fixture,
    input: RebindTypeRevisionInput,
) -> Result<DraftOutput, AppError> {
    RebindTypeRevision::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input)
        .await
}
