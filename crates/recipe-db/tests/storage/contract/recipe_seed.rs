//! Fixtures of the contract cases of `RecipeRepository` (tasks 9.5 and 9.6).
//!
//! The fixtures go through the ports and `UnitOfWork::commit` on both adapters (design D13).
//! No fixture writes SQL. The port sequence of a seed is the sequence that its use case runs.

use recipe_app::error::StorageError;
use recipe_app::ports::NewVersion;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{LeafPath, MachineId, PlcTypeId, Primitive, UnitId};
use recipe_domain::recipe::{
    LangCode, Ordinal, Parameter, ParameterId, ParameterKey, ParameterTag, Recipe, RecipeId,
    RecipeVersionId, ReferenceNumber, StoredLimit, StoredParameter, StoredValue, Translation,
};
use recipe_domain::text::{LongText, Text};
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

use super::harness::{FIXTURE_NOTE, Harness, begin, commit, fixture_hash};
use super::seed::{insert, machine, seed_type};

/// The offset that separates the identifiers of versions from the identifiers of recipes.
const VERSION_ID_OFFSET: u128 = 0xA000;
/// The offset of the parameter identifiers.
const PARAMETER_ID_OFFSET: u128 = 0xB000;
/// The offset of the parameter keys.
const PARAMETER_KEY_OFFSET: u128 = 0xC000;

/// A committed machine with its main unit and the type `DINT`.
#[derive(Clone, Copy, Debug)]
pub struct Plant {
    /// The machine of the recipes.
    pub machine: MachineId,
    /// The unit `U1` of the machine.
    pub unit: UnitId,
    /// The global type `DINT`.
    pub dint: PlcTypeId,
}

/// Insert the machine number `n` with the code `code` and find its unit and `DINT`.
pub async fn plant(h: &dyn Harness, n: u128, code: &str) -> Plant {
    let built = machine(n, code);
    insert(h, &built).await;
    let unit = built.units().next().expect("the machine has a unit").id();
    Plant {
        machine: built.id(),
        unit,
        dint: seed_type(h, built.id(), Primitive::Dint).await,
    }
}

/// The identifier of the recipe number `n`.
pub fn recipe_id(n: u128) -> RecipeId {
    RecipeId::from(Uuid::from_u128(n))
}

/// The identifier of the version number `n`. A larger `n` sorts after a smaller `n`.
pub fn version_id(n: u128) -> RecipeVersionId {
    RecipeVersionId::from(Uuid::from_u128(VERSION_ID_OFFSET.checked_add(n).unwrap()))
}

/// The first version of a recipe as `insert_recipe` takes it.
pub fn first_version(h: &dyn Harness, recipe: &Recipe) -> NewVersion {
    new_version(h, (recipe.id(), recipe.versions()[0].id()), 1, None)
}

/// The version number `number` as `insert_version` takes it.
pub fn new_version(
    h: &dyn Harness,
    (recipe, version): (RecipeId, RecipeVersionId),
    number: u32,
    based_on: Option<RecipeVersionId>,
) -> NewVersion {
    NewVersion {
        recipe,
        version,
        number,
        based_on,
        change_note: LongText::parse(FIXTURE_NOTE).unwrap(),
        creator: h.user(),
    }
}

/// Assert that an error is the trigger error `E-RCP-033`.
pub fn assert_illegal_transition(error: &StorageError) {
    assert!(
        matches!(error, StorageError::Constraint { code, .. } if *code == ErrorCode::RCP_033),
        "{error:?}"
    );
}

/// Build the recipe number `n` with the first version `version_id(n)`. Nothing is stored.
pub fn build_recipe(plant: &Plant, n: u128, reference: &str) -> Recipe {
    Recipe::create(
        recipe_id(n),
        plant.machine,
        ReferenceNumber::parse(reference).unwrap(),
        Text::parse("Recipe").unwrap(),
        version_id(n),
    )
}

/// The machine that the audit entry of a seed names when the seed has no machine at hand.
fn no_machine() -> MachineId {
    MachineId::from(Uuid::nil())
}

/// Seed a recipe with its first version through `insert_recipe` and `UnitOfWork::commit`.
pub async fn create_recipe(h: &dyn Harness, plant: &Plant, n: u128, reference: &str) -> Recipe {
    let recipe = build_recipe(plant, n, reference);
    let mut tx = begin(h).await;
    tx.recipes()
        .insert_recipe(&recipe, first_version(h, &recipe))
        .await
        .unwrap();
    commit(h, tx, recipe.machine()).await;
    recipe
}

/// Seed a live draft with the number `number` through `insert_version`.
pub async fn create_version(
    h: &dyn Harness,
    ids: (RecipeId, RecipeVersionId),
    number: u32,
    based_on: Option<RecipeVersionId>,
) {
    let mut tx = begin(h).await;
    tx.recipes()
        .insert_version(new_version(h, ids, number, based_on))
        .await
        .unwrap();
    commit(h, tx, no_machine()).await;
}

/// Seed the approval of a live draft with the port sequence of `ApproveVersion`.
///
/// The sequence is `lock_version` and `approve_version` with the hash `fixture_hash`.
pub async fn approve(h: &dyn Harness, version: RecipeVersionId) {
    let mut tx = begin(h).await;
    tx.recipes()
        .lock_version(version)
        .await
        .unwrap()
        .expect("the version exists");
    tx.recipes()
        .approve_version(version, h.user(), fixture_hash())
        .await
        .unwrap();
    commit(h, tx, no_machine()).await;
}

/// Seed the retirement of an approved version with the port sequence of `RetireVersion`.
///
/// The sequence is `lock_version` and `retire_version`.
pub async fn retire(h: &dyn Harness, version: RecipeVersionId) {
    let mut tx = begin(h).await;
    tx.recipes()
        .lock_version(version)
        .await
        .unwrap()
        .expect("the version exists");
    tx.recipes().retire_version(version).await.unwrap();
    commit(h, tx, no_machine()).await;
}

/// Read a recipe by identifier in a transaction that nothing commits.
pub async fn read_recipe(h: &dyn Harness, id: RecipeId) -> Option<Recipe> {
    let mut tx = begin(h).await;
    tx.recipes().recipe_by_id(id).await.unwrap()
}

/// A parameter of the type `DINT` in the unit `U1`. The number `n` sets the identifiers.
pub fn parameter(plant: &Plant, n: u128, tag: &str) -> StoredParameter {
    StoredParameter {
        id: ParameterId::from(Uuid::from_u128(PARAMETER_ID_OFFSET.checked_add(n).unwrap())),
        key: ParameterKey::from(Uuid::from_u128(
            PARAMETER_KEY_OFFSET.checked_add(n).unwrap(),
        )),
        unit: plant.unit,
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: Text::parse("Name").unwrap(),
        description_en: LongText::parse("Description").unwrap(),
        translations: Vec::new(),
        plc_type: plant.dint,
        s7_address: None,
        limits: Vec::new(),
        value: StoredValue::from(json!("5")),
        ordinal: Ordinal::parse(0).unwrap(),
    }
}

/// A numeric limit row.
pub fn range(path: &str, min: &str, max: &str) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: Some(min.parse::<Decimal>().unwrap()),
        max: Some(max.parse::<Decimal>().unwrap()),
        max_length: None,
    }
}

/// A limit row with a length only.
pub fn length(path: &str, max_length: u16) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: None,
        max: None,
        max_length: Some(max_length),
    }
}

/// A translation row. `description` is `None` for an empty description.
pub fn translation(lang: &str, name: &str, description: Option<&str>) -> Translation {
    Translation {
        lang: LangCode::parse(lang).unwrap(),
        name: Text::parse(name).unwrap(),
        description: description.map(|text| LongText::parse(text).unwrap()),
    }
}

/// Seed a parameter into a committed live draft with `insert_parameter` and `upsert_translation`.
///
/// The limits go with `insert_parameter`. Each translation needs its own port call.
pub async fn seed(h: &dyn Harness, version: RecipeVersionId, parameter: &StoredParameter) {
    let mut tx = begin(h).await;
    tx.drafts()
        .insert_parameter(version, &Parameter::from(parameter.clone()))
        .await
        .unwrap();
    for translation in &parameter.translations {
        tx.drafts()
            .upsert_translation(version, parameter.key, translation)
            .await
            .unwrap();
    }
    commit(h, tx, no_machine()).await;
}

/// Describe parameters as text, so a case can compare them. The type has no `PartialEq`.
pub fn describe(parameters: &[StoredParameter]) -> Vec<String> {
    parameters.iter().map(|p| format!("{p:?}")).collect()
}
