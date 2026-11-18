//! Helpers that the tests of several recipe use cases share.

use std::collections::BTreeSet;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, LeafPath, Machine, MemberName, PlcPlatform, PlcType,
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, RuleSet, TypeMember, TypeName, UnitPath,
    UnitUsage,
};
use recipe_domain::recipe::{
    LimitInput, ParameterKey, Recipe, RecipeDeleter, RecipeError, S7AccessReason, StoredLimit,
    StoredParameter, StoredValue,
};
use serde_json::{Value, json};

use crate::error::AppError;
use crate::memory::fixtures::{Fixture, mid, parameter_on, pkey, tid, timestamp, uid, user, vid};
use crate::memory::seed_type_id;
use crate::recipe::rebind::fixtures::typed_struct_row;

/// The number of the version that [`seed_speed_draft`] stores.
pub(super) const DRAFT_VERSION: u128 = 101;

/// The key of the parameter `SPEED` that [`seed_speed_draft`] stores.
pub(super) fn speed_key() -> ParameterKey {
    pkey(1)
}

/// Store the machine `M1` on `TwinCat3`, the draft `R-1` (version 101), and the parameter `SPEED`.
///
/// `SPEED` is an `INT` with the value 120, the limit 10 to 250 on the leaf path `""`, and a
/// Polish translation. It sits on the unit 101. The draft has the row version 1.
pub(super) fn seed_speed_draft(fixture: &Fixture) -> Machine {
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let speed = parameter_on(PlcPlatform::TwinCat3, 1, "SPEED", uid(101), 0);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    machine
}

/// The key of the parameter `ZONE` that [`seed_zone_on_int_struct`] stores.
pub(super) fn zone_key() -> ParameterKey {
    pkey(2)
}

/// Add the stored struct `Int@1` and the parameter `ZONE` of that type to the draft of
/// [`seed_speed_draft`].
///
/// `Int@1` has the member `x` of type `INT`. The current rule refuses the name `Int`, because it
/// is a primitive name in another case (ADR-0143). The struct is a row from before that rule, so
/// the fixture builds it with the earlier rule. `ZONE` has the value `{"x": 1}`, no limit, and
/// sits on the unit 101.
pub(super) fn seed_zone_on_int_struct(fixture: &Fixture) {
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let record = PlcTypeRecord {
        id: tid(701),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PlcPlatform::TwinCat3]),
        shape: RecordShape::Struct {
            name: TypeName::from_stored("Int", RuleSet::Earlier).unwrap(),
            revision: 1,
            supersedes: None,
            members: vec![TypeMember {
                name: MemberName::parse("x").unwrap(),
                type_id: int,
            }],
        },
    };
    fixture
        .store
        .seed_type(&PlcType::from_stored(record, RuleSet::Earlier).unwrap());
    let zone = StoredParameter {
        plc_type: tid(701),
        value: StoredValue::from(json!({"x": 1})),
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::TwinCat3, 2, "ZONE", uid(101), 1)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &zone);
}

/// Return the stored parameters of the draft that [`seed_speed_draft`] stores.
pub(super) fn stored_parameters(fixture: &Fixture) -> Vec<StoredParameter> {
    fixture.store.state().parameters_of(vid(DRAFT_VERSION))
}

/// Archive the unit at `path` of `machine`. The unit must hold no reference.
pub(super) fn archive_unit(machine: &mut Machine, path: &str) {
    machine
        .archive_unit(
            &UnitPath::parse(path).unwrap(),
            timestamp(),
            &UnitUsage::default(),
        )
        .unwrap();
}

/// A process limit on the leaf path `""` with the given bounds.
pub(super) fn bounds(min: &str, max: &str) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some(min.to_owned()),
        max: Some(max.to_owned()),
        max_length: None,
    }
}

/// A `TwinCat3` draft with the parameter `SPEED` and no S7 address.
pub(super) fn twincat_draft() -> Fixture {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    fixture
}

/// A draft of an `S7_300` machine with no parameter.
pub(super) fn s7_draft() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture
}

/// A draft of an `S7_300` machine with the `INT` parameter `SPEED` at `DB1.DBW0`.
pub(super) fn s7_draft_with_speed() -> Fixture {
    let fixture = s7_draft();
    let speed = parameter_on(PlcPlatform::S7_300, 1, "SPEED", uid(101), 0);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    fixture
}

/// The state of a draft that a rejected command must leave unchanged.
///
/// The type solves one problem: the table tests of several draft commands assert the same
/// "nothing was written" rule for the parameters, the row version, and the audit log.
pub(super) struct DraftSnapshot {
    parameters: String,
    row_version: Option<i64>,
}

impl DraftSnapshot {
    /// Capture the parameters and the row version of the draft.
    pub(super) fn capture(fixture: &Fixture) -> Self {
        Self {
            parameters: format!("{:?}", stored_parameters(fixture)),
            row_version: fixture
                .store
                .state()
                .version_row_version(vid(DRAFT_VERSION)),
        }
    }

    /// Assert that the draft, its editors, and the audit log did not change.
    pub(super) fn assert_unchanged(&self, fixture: &Fixture) {
        let now = Self::capture(fixture);
        assert_eq!(now.parameters, self.parameters);
        assert_eq!(now.row_version, self.row_version);
        assert!(fixture.store.audit_entries().is_empty());
        assert!(
            fixture
                .store
                .state()
                .editors_of(vid(DRAFT_VERSION))
                .is_empty()
        );
    }
}

/// Store recipe `n` of `machine` as a deleted recipe whose draft is discarded.
///
/// The recipe starts as the live draft of `Fixture::seed_draft`. The helper then deletes it
/// as an administrator and stores the rows again.
pub(super) fn seed_deleted(
    fixture: &Fixture,
    machine: &Machine,
    n: u128,
    reference_text: &str,
) -> Recipe {
    let (mut recipe, _) = fixture.seed_draft(machine, n, reference_text);
    recipe
        .delete(RecipeDeleter::Administrator, timestamp())
        .unwrap();
    fixture
        .store
        .seed_recipe(&recipe, machine.platform(), user(2));
    recipe
}

/// The global `BYTE` row that [`seed_standard_access_rows`] stores.
pub(super) const BYTE_TYPE: u128 = 960;
/// `ARRAY[32760..32770] OF BYTE`, a type with a bound above 32767.
pub(super) const WIDE_ARRAY: u128 = 961;
/// `ARRAY[40000..40001] OF BYTE`, the member of [`WIDE_STRUCT`].
pub(super) const WIDE_MEMBER: u128 = 962;
/// `ST_Buf@1` with the member `data` of type `ARRAY[40000..40001] OF BYTE`.
pub(super) const WIDE_STRUCT: u128 = 963;
/// `ARRAY[1..2,1..2,1..2] OF BOOL`, a `BOOL` array with three dimensions.
pub(super) const CUBE_ARRAY: u128 = 964;
/// `ST_Cube@1` with the member `bits` of type `ARRAY[1..2,1..2,1..2] OF BOOL`.
pub(super) const CUBE_STRUCT: u128 = 965;

/// An array row of the machine 1 with the bounds `(lower, upper)` of each dimension.
fn array_row_with_bounds(
    platform: PlcPlatform,
    id: u128,
    element: PlcTypeId,
    bounds: &[(i32, i32)],
) -> PlcType {
    let dimensions = bounds
        .iter()
        .map(|(lower, upper)| ArrayDimension::parse(*lower, *upper).unwrap())
        .collect();
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Array {
            element,
            dimensions: ArrayDimensions::parse(dimensions).unwrap(),
        },
    })
    .unwrap()
}

/// Store the rows that the tests of ADR-0178 and ADR-0187 need: a global `BYTE` row, the arrays
/// [`WIDE_ARRAY`] and [`CUBE_ARRAY`], and the structs [`WIDE_STRUCT`] and [`CUBE_STRUCT`].
///
/// The rows are rows from before the rules, so a stored parameter can hold them. The seed of the
/// memory store has `BOOL` but no `BYTE`.
pub(super) fn seed_standard_access_rows(fixture: &Fixture, platform: PlcPlatform) {
    let byte = fixture.seed_primitive_type(BYTE_TYPE, platform, Primitive::Byte);
    let bool_row = seed_type_id(Primitive::Bool, None).unwrap();
    let rows = [
        array_row_with_bounds(platform, WIDE_ARRAY, byte, &[(32_760, 32_770)]),
        array_row_with_bounds(platform, WIDE_MEMBER, byte, &[(40_000, 40_001)]),
        typed_struct_row(
            platform,
            (WIDE_STRUCT, "ST_Buf", 1),
            None,
            &[("data", tid(WIDE_MEMBER))],
        ),
        array_row_with_bounds(platform, CUBE_ARRAY, bool_row, &[(1, 2), (1, 2), (1, 2)]),
        typed_struct_row(
            platform,
            (CUBE_STRUCT, "ST_Cube", 1),
            None,
            &[("bits", tid(CUBE_ARRAY))],
        ),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
}

/// The value of [`WIDE_ARRAY`]: 11 bytes of 0.
pub(super) fn wide_array_value() -> Value {
    Value::Array(vec![json!(0); 11])
}

/// The value of [`WIDE_STRUCT`]: the member `data` with 2 bytes of 0.
pub(super) fn wide_struct_value() -> Value {
    json!({"data": [0, 0]})
}

/// A cube of booleans with `side` elements in each of the three dimensions.
pub(super) fn bool_cube(side: usize) -> Value {
    let row = Value::Array(vec![json!(false); side]);
    let plane = Value::Array(vec![row; side]);
    Value::Array(vec![plane; side])
}

/// The limit of every byte of a one-dimension array: 0 to 255 on the leaf path `[*]`.
pub(super) fn byte_elements_limit() -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("[*]").unwrap(),
        min: Some("0".parse().unwrap()),
        max: Some("255".parse().unwrap()),
        max_length: None,
    }
}

/// Assert that `error` is `E-RCP-058` for the tag `tag` at `DB1.DBB0` with `reason`, and that its
/// message names the tag, the address, and the reason (ADR-0178 (b), ADR-0187).
pub(super) fn assert_not_addressable(error: &AppError, tag: &str, reason: S7AccessReason) {
    assert_eq!(error.code(), ErrorCode::RCP_058);
    assert!(
        matches!(
            error,
            AppError::Recipe(RecipeError::S7TypeNotAddressable { reason: found, .. })
                if *found == reason
        ),
        "{error:?}"
    );
    let reason_text = match reason {
        S7AccessReason::ArrayBoundOutsideInt => "an array bound is outside -32768 to 32767",
        S7AccessReason::BoolArrayWithThreeDimensions => {
            "a BOOL array with three dimensions has no verified S7 layout"
        }
    };
    assert_eq!(
        error.to_string(),
        format!(
            "Parameter {tag} at DB1.DBB0 has a type that a data block with standard access cannot hold: {reason_text}. Change the type."
        )
    );
}
