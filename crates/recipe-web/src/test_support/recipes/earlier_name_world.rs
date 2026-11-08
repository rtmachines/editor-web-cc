//! A stored struct whose name the current rule refuses, and a parameter of that type
//! (ADR-0143 (i1), ADR-0120).
//!
//! The pages of a parameter read the type text of a stored parameter and send it again. The
//! tests of that path need a stored struct `Int@1`. The current rule refuses the name `Int`,
//! because it is a primitive name in another case. This module is test code.

use std::collections::BTreeSet;

use recipe_app::memory::seed_type_id;
use recipe_domain::plant::{
    MemberName, PlatformTypes, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive,
    RecordShape, RuleSet, S7Address, TypeMember, TypeName, TypeTree, UnitPath,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, ParameterValue, StoredParameter,
};
use recipe_domain::text::{LongText, Text};
use uuid::Uuid;

use crate::test_support::Fixture;
use crate::test_support::plant::PlantIds;

/// The identifier of the struct `Int@1` of `M7`.
const INT_STRUCT_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0e01);
/// The identifier of the parameter `ZONE`.
const ZONE_ID: Uuid = Uuid::from_u128(0x7357_0006_0000_0000_0000_0000_0000_0e01);
/// The key of the parameter `ZONE`, which has the type `Int@1`.
pub(crate) const ZONE_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0000_0e01);
/// The ordinal of the parameter `ZONE`. The seeded parameters have lower ordinals.
const ZONE_ORDINAL: u32 = 20;

/// Return the stored row of the struct `Int@1` of `M7` with the member `speed` of type `INT`.
fn int_struct_row(plant: &PlantIds, int: PlcTypeId) -> PlcType {
    let name = TypeName::from_stored("Int", RuleSet::Earlier).unwrap();
    PlcType::from_stored(
        PlcTypeRecord {
            id: PlcTypeId::from(INT_STRUCT_ID),
            machine: Some(plant.m7),
            platforms: BTreeSet::from([PlcPlatform::S7_300]),
            shape: RecordShape::Struct {
                name,
                revision: 1,
                supersedes: None,
                members: vec![TypeMember {
                    name: MemberName::parse("speed").unwrap(),
                    type_id: int,
                }],
            },
        },
        RuleSet::Earlier,
    )
    .unwrap()
}

/// Add the struct `Int@1` to `M7` and the parameter `ZONE` of that type to the draft 3 of `R-100`.
///
/// Call it after `seed_recipes`. `ZONE` stands on `U1` at `DB6.DBB0` with the value `{"speed":5}`.
pub(crate) fn seed_int_named_parameter(fixture: &Fixture) {
    let plant = fixture.seed_plant();
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let int_row = fixture.store.state().plc_type(int).cloned().unwrap();
    let row = int_struct_row(&plant, int);
    fixture.store.seed_type(&row);
    let types = PlatformTypes::collect(plant.m7, PlcPlatform::S7_300, vec![int_row, row]).unwrap();
    let tree = TypeTree::resolve(PlcTypeId::from(INT_STRUCT_ID), &types).unwrap();
    let value = ParameterValue::parse(r#"{"speed":5}"#, &tree).unwrap();
    let unit = fixture
        .store
        .state()
        .machine(plant.m7)
        .unwrap()
        .unit(&UnitPath::parse("U1").unwrap())
        .unwrap()
        .id();
    let parameter = StoredParameter {
        id: ParameterId::from(ZONE_ID),
        key: ParameterKey::from(ZONE_KEY),
        unit,
        tag: ParameterTag::parse("ZONE").unwrap(),
        name_en: Text::parse("ZONE").unwrap(),
        description_en: LongText::parse("A parameter of the press line").unwrap(),
        translations: Vec::new(),
        plc_type: PlcTypeId::from(INT_STRUCT_ID),
        s7_address: Some(S7Address::parse("DB6.DBB0").unwrap()),
        limits: Vec::new(),
        value: value.to_stored(),
        ordinal: Ordinal::parse(ZONE_ORDINAL).unwrap(),
    };
    fixture.store.seed_parameter(plant.draft, &parameter);
}
