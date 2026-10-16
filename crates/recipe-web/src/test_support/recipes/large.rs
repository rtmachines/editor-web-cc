//! The large draft: `Fixture::seed_large_draft` (design D12, task 8.3).
//!
//! The function solves one problem: the tests of the parameter table need a draft with more
//! parameters than one page holds. The shared fixture must stay small.
//! The draft has 4 000 parameters in 40 units. Each unit holds 100 parameters.

use recipe_app::memory::seed_type_id;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{
    Machine, MachineCode, MachineId, PlatformTypes, PlcPlatform, Primitive, TypeTree, UnitId,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, ParameterValue, Recipe, RecipeId,
    RecipeVersionId, ReferenceNumber, StoredParameter, StoredRecipe, StoredValue,
    StoredVersionHeader, VersionState,
};
use recipe_domain::text::LongText;
use uuid::Uuid;

use super::{CREATOR_NUMBER, text};
use crate::test_support::Fixture;

/// The reference number of the large recipe.
const REFERENCE: &str = "R-400";
/// The number of the large draft.
const NUMBER: u32 = 1;
/// The number of the units of the large machine.
const UNITS: u32 = 40;
/// The number of the parameters in each unit.
const PER_UNIT: u32 = 100;
/// The identifier of the machine `M40`.
const MACHINE_ID: Uuid = Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0000_0028);
/// The identifier of the recipe `R-400`.
const RECIPE_ID: Uuid = Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0000_0400);
/// The identifier of the draft of `R-400`.
const DRAFT_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0000_0401);
/// The identifier of the unit 0 of `M40`. The last field of a unit is its number.
const UNIT_BASE: u128 = 0x7357_0014_0000_0000_0000_0000_0028_0000;
/// The first field of the identifier of a parameter.
const PARAMETER_BASE: u128 = 0x7357_0016_0000_0000_0000_0000_0000_0000;
/// The first field of the key of a parameter.
const KEY_BASE: u128 = 0x7357_0017_0000_0000_0000_0000_0000_0000;

/// What `Fixture::seed_large_draft` seeds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LargeDraft {
    /// The reference number of the recipe.
    pub(crate) reference: &'static str,
    /// The number of the draft.
    pub(crate) number: u32,
    /// The machine `M40` with the units of the draft.
    pub(crate) machine: MachineId,
    /// The number of the parameters of the draft.
    pub(crate) parameters: usize,
    /// The number of the units that hold the parameters.
    pub(crate) units: usize,
}

impl Fixture {
    /// Seed a live draft with 4 000 parameters in 40 units and return what it holds.
    ///
    /// The draft belongs to the new recipe `R-400` of the new machine `M40` (`TwinCat3`). Each
    /// parameter has the type `INT`. The function does not call `seed_plant`, so a test can use it
    /// alone or after `seed_recipes`.
    pub(crate) fn seed_large_draft(&self) -> LargeDraft {
        self.store.seed_primitive_types();
        self.store.seed_machine(&large_machine());
        let creator = UserId::from(Uuid::from_u128(CREATOR_NUMBER));
        self.store
            .seed_recipe(&large_recipe(), PlcPlatform::TwinCat3, creator);
        let value = zero_value(self);
        for unit in 1..=UNITS {
            for slot in 0..PER_UNIT {
                let parameter = parameter(unit, slot, &value);
                self.store
                    .seed_parameter(RecipeVersionId::from(DRAFT_ID), &parameter);
            }
        }
        LargeDraft {
            reference: REFERENCE,
            number: NUMBER,
            machine: MachineId::from(MACHINE_ID),
            parameters: usize::try_from(UNITS.checked_mul(PER_UNIT).unwrap()).unwrap(),
            units: usize::try_from(UNITS).unwrap(),
        }
    }
}

/// Return the identifier of the unit with the number `unit` of `M40`.
fn unit_id(unit: u32) -> UnitId {
    UnitId::from(Uuid::from_u128(UNIT_BASE | u128::from(unit)))
}

/// Build the live machine `M40` with the top units `U1` to `U40`.
fn large_machine() -> Machine {
    let mut machine = Machine::create(
        MachineId::from(MACHINE_ID),
        MachineCode::parse("M40").unwrap(),
        text("Warehouse"),
        PlcPlatform::TwinCat3,
        (unit_id(1), text("Unit 1")),
    );
    for unit in 2..=UNITS {
        machine
            .add_unit(
                unit_id(unit),
                None,
                &unit.to_string(),
                text(&format!("Unit {unit}")),
            )
            .unwrap();
    }
    machine
}

/// Build the recipe `R-400` of `M40` with one live draft.
fn large_recipe() -> Recipe {
    Recipe::try_from(StoredRecipe {
        id: RecipeId::from(RECIPE_ID),
        machine: MachineId::from(MACHINE_ID),
        reference: ReferenceNumber::parse(REFERENCE).unwrap(),
        name: text("Warehouse recipe"),
        deleted_at: None,
        versions: vec![StoredVersionHeader {
            id: RecipeVersionId::from(DRAFT_ID),
            number: NUMBER,
            state: VersionState::LiveDraft,
            row_version: 1,
        }],
    })
    .unwrap()
}

/// Return the stored value 0 of an `INT` on the platform of `M40`.
fn zero_value(fixture: &Fixture) -> StoredValue {
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let state = fixture.store.state();
    let rows = vec![state.plc_type(int).cloned().unwrap()];
    let types =
        PlatformTypes::collect(MachineId::from(MACHINE_ID), PlcPlatform::TwinCat3, rows).unwrap();
    let tree = TypeTree::resolve(int, &types).unwrap();
    ParameterValue::parse("0", &tree).unwrap().to_stored()
}

/// Build the parameter in the slot `slot` of the unit `unit`. Its tag is `P` and four digits.
fn parameter(unit: u32, slot: u32, value: &StoredValue) -> StoredParameter {
    let number = unit
        .checked_sub(1)
        .and_then(|before| before.checked_mul(PER_UNIT))
        .and_then(|first| first.checked_add(slot))
        .unwrap();
    let tag = format!("P{number:04}");
    let n = u128::from(number).checked_add(1).unwrap();
    StoredParameter {
        id: ParameterId::from(Uuid::from_u128(PARAMETER_BASE | n)),
        key: ParameterKey::from(Uuid::from_u128(KEY_BASE | n)),
        unit: unit_id(unit),
        tag: ParameterTag::parse(&tag).unwrap(),
        name_en: text(&tag),
        description_en: LongText::parse("A parameter of the large draft").unwrap(),
        translations: Vec::new(),
        plc_type: seed_type_id(Primitive::Int, None).unwrap(),
        s7_address: None,
        limits: Vec::new(),
        value: value.clone(),
        ordinal: Ordinal::parse(slot).unwrap(),
    }
}
