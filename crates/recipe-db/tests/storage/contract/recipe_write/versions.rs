//! Contract cases of `insert_version`.

use std::collections::BTreeSet;

use recipe_app::error::{RECIPE_VERSION_ONE_LIVE_DRAFT, StorageError};
use recipe_domain::recipe::{ParameterId, StoredParameter};
use uuid::Uuid;

use crate::contract::harness::{Harness, begin, commit};
use crate::contract::images::{Stage, assert_shape, field, text, version_shape};
use crate::contract::recipe_seed::{
    approve, create_recipe, describe, length, new_version, parameter, plant, range, recipe_id,
    seed, translation, version_id,
};

/// Two parameters, one with limits and translations and one without.
fn source_parameters(plant: &crate::contract::recipe_seed::Plant) -> Vec<StoredParameter> {
    let mut full = parameter(plant, 1, "FULL");
    full.limits = vec![range("", "0", "10.25"), length(".a", 12)];
    full.translations = vec![
        translation("de", "Name", Some("Beschreibung")),
        translation("pl", "Nazwa", None),
    ];
    vec![full, parameter(plant, 2, "PLAIN")]
}

/// Return the parameters with the identifier `nil`, so that two reads compare by content.
fn without_ids(parameters: &[StoredParameter]) -> Vec<String> {
    let cleared: Vec<StoredParameter> = parameters
        .iter()
        .cloned()
        .map(|mut p| {
            p.id = ParameterId::from(Uuid::nil());
            p
        })
        .collect();
    describe(&cleared)
}

async fn insert_version_copies_the_parameters_with_new_ids_and_the_same_keys(h: &dyn Harness) {
    let plant = plant(h, 10, "M10").await;
    let created = create_recipe(h, &plant, 10, "REF-10").await;
    let first = created.versions()[0].id();
    let source = source_parameters(&plant);
    for parameter in &source {
        seed(h, first, parameter).await;
    }
    approve(h, first).await;
    let mut tx = begin(h).await;

    let image = tx
        .recipes()
        .insert_version(new_version(
            h,
            (created.id(), version_id(1)),
            2,
            Some(first),
        ))
        .await
        .unwrap();
    commit(h, tx, plant.machine).await;

    assert_shape(&image, &version_shape(true, Stage::Draft));
    assert_eq!(text(&image, "id"), version_id(1).to_string());
    assert_eq!(text(&image, "based_on_version_id"), first.to_string());
    assert_eq!(text(&image, "status"), "draft");
    assert_eq!(field(&image, "number").as_i64(), Some(2));
    assert_eq!(field(&image, "row_version").as_i64(), Some(1));
    let mut tx = begin(h).await;
    let original = tx.recipes().version_parameters(first).await.unwrap();
    let copy = tx
        .recipes()
        .version_parameters(version_id(1))
        .await
        .unwrap();
    assert_eq!(describe(&original), describe(&source));
    assert_eq!(copy.len(), 2);
    assert_eq!(without_ids(&copy), without_ids(&source));
    let keys: Vec<_> = copy.iter().map(|p| p.key).collect();
    let source_keys: Vec<_> = source.iter().map(|p| p.key).collect();
    assert_eq!(keys, source_keys);
    let old_ids: BTreeSet<_> = original.iter().map(|p| p.id).collect();
    let new_ids: BTreeSet<_> = copy.iter().map(|p| p.id).collect();
    assert_eq!(new_ids.len(), 2);
    assert!(old_ids.is_disjoint(&new_ids));
    assert_eq!(copy[0].limits.len(), 2);
    assert_eq!(copy[0].translations.len(), 2);
}

async fn insert_version_without_a_base_copies_nothing(h: &dyn Harness) {
    let plant = plant(h, 11, "M11").await;
    let created = create_recipe(h, &plant, 11, "REF-11").await;
    let first = created.versions()[0].id();
    seed(h, first, &parameter(&plant, 1, "ONLY")).await;
    approve(h, first).await;
    let mut tx = begin(h).await;

    let image = tx
        .recipes()
        .insert_version(new_version(h, (created.id(), version_id(1)), 2, None))
        .await
        .unwrap();
    let copy = tx
        .recipes()
        .version_parameters(version_id(1))
        .await
        .unwrap();

    assert_shape(&image, &version_shape(false, Stage::Draft));
    assert!(copy.is_empty());
}

async fn a_second_live_draft_gives_the_constraint_of_the_live_draft_index(h: &dyn Harness) {
    let plant = plant(h, 12, "M12").await;
    let created = create_recipe(h, &plant, 12, "REF-12").await;
    let first = created.versions()[0].id();
    let mut tx = begin(h).await;

    let error = tx
        .recipes()
        .insert_version(new_version(
            h,
            (created.id(), version_id(1)),
            2,
            Some(first),
        ))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint }
            if constraint == RECIPE_VERSION_ONE_LIVE_DRAFT),
        "{error:?}"
    );
}

async fn a_version_of_a_missing_recipe_gives_a_missing_reference(h: &dyn Harness) {
    let mut tx = begin(h).await;

    let error = tx
        .recipes()
        .insert_version(new_version(h, (recipe_id(99), version_id(1)), 1, None))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::MissingReference { constraint }
            if constraint == "recipe_version_recipe_platform"),
        "{error:?}"
    );
}

contract_case!(
    insert_version_copies_the_parameters_with_new_ids_and_the_same_keys,
    insert_version_copies_the_parameters_with_new_ids_and_the_same_keys_memory,
    insert_version_copies_the_parameters_with_new_ids_and_the_same_keys_postgres
);
contract_case!(
    insert_version_without_a_base_copies_nothing,
    insert_version_without_a_base_copies_nothing_memory,
    insert_version_without_a_base_copies_nothing_postgres
);
contract_case!(
    a_second_live_draft_gives_the_constraint_of_the_live_draft_index,
    a_second_live_draft_gives_the_constraint_of_the_live_draft_index_memory,
    a_second_live_draft_gives_the_constraint_of_the_live_draft_index_postgres
);
contract_case!(
    a_version_of_a_missing_recipe_gives_a_missing_reference,
    a_version_of_a_missing_recipe_gives_a_missing_reference_memory,
    a_version_of_a_missing_recipe_gives_a_missing_reference_postgres
);
