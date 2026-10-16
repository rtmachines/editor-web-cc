//! The seven parameters that `seed_recipes` adds to the draft of `R-100` (design D12).
//!
//! The function `draft_parameters` resolves the type tree of each parameter from the rows of the
//! store, so a value always matches its tree.

use recipe_app::memory::{MemoryState, seed_type_id};
use recipe_domain::plant::{
    LeafPath, PlatformTypes, PlcPlatform, PlcType, PlcTypeId, Primitive, S7Address, TypeTree,
    UnitPath,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, ParameterValue, StoredLimit, StoredParameter,
};
use recipe_domain::text::LongText;
use uuid::Uuid;

use super::{
    CELL_ID, GRID_KEY, GRID_TYPE_ID, HOLD_KEY, LABEL_KEY, LINE_A_KEY, PAIR_ID, SPEED_KEY,
    SPEED_TABLE_KEY, SPEED_TABLE_TYPE_ID, STRING_20_ID, ZONE_V3_ID, ZONES_KEY, text,
};
use crate::test_support::Fixture;
use crate::test_support::plant::PlantIds;

/// The first field of the identifier of a parameter. The last field is the number of the parameter.
const PARAMETER_ID_BASE: u128 = 0x7357_0006_0000_0000_0000_0000_0000_0000;
/// The number of the parameter before the first one. The ordinal is the number minus this value.
const FIRST_NUMBER: u128 = 0x10;
/// The upper bound of the limits of `SPEED` and `SPEED_TABLE`.
const SPEED_MAX: &str = "3000";
/// The value of one zone: the speed and the maximum temperature.
const ZONE_VALUE: &str = r#"{"speed":0,"temp_max":0}"#;

/// One parameter of the draft: the fields that differ from parameter to parameter.
struct Spec {
    /// The number in the last field of the identifier.
    n: u128,
    key: Uuid,
    tag: &'static str,
    unit: &'static str,
    address: &'static str,
    plc_type: PlcTypeId,
    /// The value as JSON text.
    value: String,
    limits: Vec<StoredLimit>,
}

impl Spec {
    /// Build a spec without limits.
    fn new(
        (n, key, tag): (u128, Uuid, &'static str),
        (unit, address): (&'static str, &'static str),
        plc_type: PlcTypeId,
        value: &str,
    ) -> Self {
        Self {
            n,
            key,
            tag,
            unit,
            address,
            plc_type,
            value: value.to_owned(),
            limits: Vec::new(),
        }
    }

    /// Add a limit from 0 to `SPEED_MAX` on the leaf `path`.
    fn limited(mut self, path: &str) -> Self {
        self.limits.push(StoredLimit {
            leaf_path: LeafPath::parse(path).unwrap(),
            min: Some("0".parse().unwrap()),
            max: Some(SPEED_MAX.parse().unwrap()),
            max_length: None,
        });
        self
    }
}

/// Build the specs of the parameters `SPEED`, `LABEL`, and `HOLD`. They stand on `U1`.
fn scalar_specs() -> Vec<Spec> {
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let time = seed_type_id(Primitive::Time, None).unwrap();
    let string = PlcTypeId::from(STRING_20_ID);
    vec![
        Spec::new((0x11, SPEED_KEY, "SPEED"), ("U1", "DB2.DBW0"), int, "1200").limited(""),
        Spec::new(
            (0x12, LABEL_KEY, "LABEL"),
            ("U1", "DB2.DBB2"),
            string,
            "\"LINE A\"",
        ),
        Spec::new(
            (0x13, HOLD_KEY, "HOLD"),
            ("U1", "DB2.DBD24"),
            time,
            "\"T#1m30s\"",
        ),
    ]
}

/// Build the specs of the parameters `SPEED_TABLE`, `GRID`, `LINE_A`, and `ZONES`. They stand on
/// `U1.3`.
fn structured_specs(plant: &PlantIds) -> Vec<Spec> {
    let table = PlcTypeId::from(SPEED_TABLE_TYPE_ID);
    let grid = PlcTypeId::from(GRID_TYPE_ID);
    let cell = PlcTypeId::from(CELL_ID);
    let zones = format!("[{ZONE_VALUE},{ZONE_VALUE},{ZONE_VALUE},{ZONE_VALUE}]");
    vec![
        Spec::new(
            (0x14, SPEED_TABLE_KEY, "SPEED_TABLE"),
            ("U1.3", "DB3.DBB0"),
            table,
            "[100,200,300,400,500,600,700,800]",
        )
        .limited("[*]"),
        Spec::new(
            (0x15, GRID_KEY, "GRID"),
            ("U1.3", "DB3.DBB20"),
            grid,
            "[[0.5,1.5],[2.5,3.5]]",
        ),
        Spec::new(
            (0x16, LINE_A_KEY, "LINE_A"),
            ("U1.3", "DB4.DBB0"),
            cell,
            r#"{"head":{"a":1,"b":2.5},"count":3}"#,
        ),
        Spec::new(
            (0x17, ZONES_KEY, "ZONES"),
            ("U1.3", "DB5.DBB0"),
            plant.zones_array,
            &zones,
        ),
    ]
}

/// Resolve the type tree of `type_id` on the S7-300 of `M7` from the rows of the store.
fn tree_of(state: &MemoryState, plant: &PlantIds, type_id: PlcTypeId) -> TypeTree {
    let primitives = [Primitive::Int, Primitive::Real, Primitive::Time]
        .map(|primitive| seed_type_id(primitive, None).unwrap());
    let derived = [
        plant.zone_v1,
        plant.zone_v2,
        plant.zones_array,
        plant.line_v1,
        PlcTypeId::from(ZONE_V3_ID),
        PlcTypeId::from(STRING_20_ID),
        PlcTypeId::from(SPEED_TABLE_TYPE_ID),
        PlcTypeId::from(GRID_TYPE_ID),
        PlcTypeId::from(PAIR_ID),
        PlcTypeId::from(CELL_ID),
    ];
    let rows: Vec<PlcType> = primitives
        .into_iter()
        .chain(derived)
        .map(|id| state.plc_type(id).cloned().unwrap())
        .collect();
    let types = PlatformTypes::collect(plant.m7, PlcPlatform::S7_300, rows).unwrap();
    TypeTree::resolve(type_id, &types).unwrap()
}

/// Build the row of one parameter. The value is parsed against the tree of its type.
fn stored(spec: Spec, state: &MemoryState, plant: &PlantIds) -> StoredParameter {
    let machine = state.machine(plant.m7).unwrap();
    let unit = machine
        .unit(&UnitPath::parse(spec.unit).unwrap())
        .unwrap()
        .id();
    let tree = tree_of(state, plant, spec.plc_type);
    let value = ParameterValue::parse(&spec.value, &tree).unwrap();
    let ordinal = u32::try_from(spec.n.checked_sub(FIRST_NUMBER).unwrap()).unwrap();
    StoredParameter {
        id: ParameterId::from(Uuid::from_u128(PARAMETER_ID_BASE | spec.n)),
        key: ParameterKey::from(spec.key),
        unit,
        tag: ParameterTag::parse(spec.tag).unwrap(),
        name_en: text(spec.tag),
        description_en: LongText::parse("A parameter of the press line").unwrap(),
        translations: Vec::new(),
        plc_type: spec.plc_type,
        s7_address: Some(S7Address::parse(spec.address).unwrap()),
        limits: spec.limits,
        value: value.to_stored(),
        ordinal: Ordinal::parse(ordinal).unwrap(),
    }
}

/// Build the seven parameters. The type rows of `seed_recipes` must be in the store already.
pub(super) fn draft_parameters(fixture: &Fixture, plant: &PlantIds) -> Vec<StoredParameter> {
    let state = fixture.store.state();
    let mut specs = scalar_specs();
    specs.extend(structured_specs(plant));
    specs
        .into_iter()
        .map(|spec| stored(spec, &state, plant))
        .collect()
}
