//! The world of the recipe and version page tests (design D12). This module is test code.
//!
//! The module solves one problem: the recipe tests and the version tests need the same data. They
//! need `seed_recipes`, the three sessions of the plant world, and some extra recipes. One module
//! gives them, so no test file repeats the seed code.

use recipe_app::memory::seed_type_id;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{MachineId, PlatformTypes, PlcPlatform, Primitive, TypeTree, UnitId};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, ParameterValue, Recipe, RecipeId,
    RecipeVersionId, ReferenceNumber, StoredParameter, StoredRecipe, StoredValue,
    StoredVersionHeader, VersionState,
};
use recipe_domain::text::{LongText, Text};
use uuid::Uuid;

use crate::test_support::Fixture;
use crate::test_support::plant::{World, world};
use crate::test_support::recipes::M20_ID;

/// The first field of the identifier of an extra recipe. The last field is its number.
const RECIPE_BASE: u128 = 0x7357_0904_0000_0000_0000_0000_0000_0000;
/// The first field of the identifier of a version of an extra recipe.
const VERSION_BASE: u128 = 0x7357_0905_0000_0000_0000_0000_0000_0000;
/// The first field of the identifier and of the key of a parameter of an extra recipe.
const PARAMETER_BASE: u128 = 0x7357_0906_0000_0000_0000_0000_0000_0000;
/// The step between the versions of two extra recipes.
const VERSION_STEP: u128 = 0x1_0000;

/// Build the plant world with the recipe data of `seed_recipes`.
pub(crate) fn recipe_world() -> World {
    let world = world();
    world.fixture.seed_recipes();
    world
}

/// Parse a short text.
fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

/// Build the header of one version.
fn header(recipe: u32, number: u32, state: VersionState) -> StoredVersionHeader {
    let offset = u128::from(recipe)
        .checked_mul(VERSION_STEP)
        .and_then(|first| first.checked_add(u128::from(number)))
        .unwrap();
    StoredVersionHeader {
        id: RecipeVersionId::from(Uuid::from_u128(VERSION_BASE | offset)),
        number,
        state,
        row_version: 1,
    }
}

/// Seed the recipe `reference` of the machine `M20` with the version states `states`.
///
/// `n` makes the identifiers unique. The version numbers start at 1. Return the identifier of
/// each version in the order of `states`.
pub(crate) fn seed_recipe_of_m20(
    fixture: &Fixture,
    (n, reference): (u32, &str),
    states: &[VersionState],
) -> Vec<RecipeVersionId> {
    seed_recipe_of(fixture, MachineId::from(M20_ID), (n, reference), states)
}

/// Seed the recipe `reference` of the TwinCAT 3 machine `machine` with the states `states`.
///
/// The machine can be archived, for example `M12`. The other rules are those of
/// [`seed_recipe_of_m20`].
pub(crate) fn seed_recipe_of(
    fixture: &Fixture,
    machine: MachineId,
    (n, reference): (u32, &str),
    states: &[VersionState],
) -> Vec<RecipeVersionId> {
    let versions: Vec<StoredVersionHeader> = (1..)
        .zip(states)
        .map(|(number, state)| header(n, number, *state))
        .collect();
    let ids = versions.iter().map(|version| version.id).collect();
    let recipe = Recipe::try_from(StoredRecipe {
        id: RecipeId::from(Uuid::from_u128(RECIPE_BASE | u128::from(n))),
        machine,
        reference: ReferenceNumber::parse(reference).unwrap(),
        name: text(&format!("Extra recipe {n}")),
        deleted_at: None,
        versions,
    })
    .unwrap();
    let creator = UserId::from(Uuid::from_u128(1));
    fixture
        .store
        .seed_recipe(&recipe, PlcPlatform::TwinCat3, creator);
    ids
}

/// Seed `count` extra recipes `R-1000` and up of `M20`, each with one approved version.
pub(crate) fn seed_many_recipes(fixture: &Fixture, count: u32) {
    for n in 0..count {
        let number = n.checked_add(1000).unwrap();
        let reference = format!("R-{number}");
        seed_recipe_of_m20(fixture, (number, &reference), &[VersionState::Approved]);
    }
}

/// Return the stored value 0 of an `INT` on the TwinCAT 3 of `M20`.
fn zero_int(fixture: &Fixture) -> StoredValue {
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let rows = vec![fixture.store.state().plc_type(int).cloned().unwrap()];
    let types =
        PlatformTypes::collect(MachineId::from(M20_ID), PlcPlatform::TwinCat3, rows).unwrap();
    let tree = TypeTree::resolve(int, &types).unwrap();
    ParameterValue::parse("0", &tree).unwrap().to_stored()
}

/// Seed an `INT` parameter `tag` on a unit that the machine does not hold.
///
/// The snapshot check of `CreateVersion` reports the parameter with `E-PLT-020`.
pub(crate) fn seed_parameter_on_unknown_unit(
    fixture: &Fixture,
    version: RecipeVersionId,
    (n, tag): (u32, &str),
) {
    let id = Uuid::from_u128(PARAMETER_BASE | u128::from(n));
    let parameter = StoredParameter {
        id: ParameterId::from(id),
        key: ParameterKey::from(id),
        unit: UnitId::from(Uuid::from_u128(PARAMETER_BASE | 0xFFFF)),
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: text(tag),
        description_en: LongText::parse("A parameter on an unknown unit").unwrap(),
        translations: Vec::new(),
        plc_type: seed_type_id(Primitive::Int, None).unwrap(),
        s7_address: None,
        limits: Vec::new(),
        value: zero_int(fixture),
        ordinal: Ordinal::parse(n).unwrap(),
    };
    fixture.store.seed_parameter(version, &parameter);
}
