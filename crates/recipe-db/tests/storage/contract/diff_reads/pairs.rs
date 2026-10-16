//! Contract cases of `DiffReads::pairs` (task 7.1, design D4).
//!
//! A side of `None` reads no row. The pairs hold only the given keys, in the order of the keys.
//! Each parameter equals what `RecipeReads::parameter_by_key` gives for the same key.

use recipe_domain::recipe::{ParameterKey, RecipeVersionId, StoredParameter};
use serde_json::json;
use uuid::Uuid;

use super::fixtures::{
    DiffPlant, begin_read, diff_plant, on_new_side, read_pairs, seed_versions, typed,
};
use crate::contract::harness::Harness;
use crate::contract::recipe_seed::{describe, range, translation};

/// Three parameters with limits and translations: `P`, `Q`, and `R`.
fn old_side(plant: &DiffPlant) -> Vec<StoredParameter> {
    vec![
        StoredParameter {
            limits: vec![range("", "0", "10")],
            translations: vec![translation("pl", "Nazwa", Some("Opis"))],
            ..typed(plant, (31, "P"), plant.int, json!("5"))
        },
        StoredParameter {
            limits: vec![range("[1]", "0", "10"), range("[3]", "1", "9")],
            translations: vec![
                translation("pl", "Tabela", None),
                translation("de", "Tabelle", None),
            ],
            ..typed(plant, (32, "Q"), plant.table, json!(["1", "2", "3"]))
        },
        typed(plant, (33, "R"), plant.int, json!("7")),
    ]
}

/// Read the parameter of `key` in `version` through `RecipeReads::parameter_by_key`.
async fn by_key(h: &dyn Harness, version: RecipeVersionId, key: ParameterKey) -> StoredParameter {
    let mut tx = begin_read(h).await;
    tx.recipe_reads()
        .parameter_by_key(version, key)
        .await
        .unwrap()
        .unwrap()
}

async fn pairs_give_the_parameters_of_the_given_keys_on_each_side(h: &dyn Harness) {
    let plant = diff_plant(h, 80, "M80").await;
    let old = old_side(&plant);
    let new: Vec<StoredParameter> = old.iter().map(on_new_side).collect();
    let versions = seed_versions(h, &plant.plant, 80, (&old, &new)).await;
    let missing = ParameterKey::from(Uuid::from_u128(0xdead));
    let keys = [old[1].key, missing, old[0].key];

    let sides = read_pairs(h, Some(versions.old), Some(versions.new), &keys).await;

    let old_expected = vec![
        by_key(h, versions.old, keys[0]).await,
        by_key(h, versions.old, keys[2]).await,
    ];
    let new_expected = vec![
        by_key(h, versions.new, keys[0]).await,
        by_key(h, versions.new, keys[2]).await,
    ];
    assert_eq!(describe(&sides.old), describe(&old_expected));
    assert_eq!(describe(&sides.new), describe(&new_expected));
    assert_eq!(sides.old[0].limits.len(), 2);
    assert_eq!(sides.new[1].translations.len(), 1);
}

contract_case!(
    pairs_give_the_parameters_of_the_given_keys_on_each_side,
    memory_pairs_give_the_parameters_of_the_given_keys_on_each_side,
    postgres_pairs_give_the_parameters_of_the_given_keys_on_each_side
);

async fn a_side_of_none_reads_no_parameter(h: &dyn Harness) {
    let plant = diff_plant(h, 82, "M82").await;
    let old = old_side(&plant);
    let new: Vec<StoredParameter> = old.iter().map(on_new_side).collect();
    let versions = seed_versions(h, &plant.plant, 82, (&old, &new)).await;
    let keys = [old[0].key, old[2].key];

    let old_only = read_pairs(h, Some(versions.old), None, &keys).await;
    let new_only = read_pairs(h, None, Some(versions.new), &keys).await;

    let ids = |side: &[StoredParameter]| side.iter().map(|p| p.id).collect::<Vec<_>>();
    assert_eq!(ids(&old_only.old), [old[0].id, old[2].id]);
    assert!(old_only.new.is_empty());
    assert!(new_only.old.is_empty());
    assert_eq!(ids(&new_only.new), [new[0].id, new[2].id]);
}

contract_case!(
    a_side_of_none_reads_no_parameter,
    memory_a_side_of_none_reads_no_parameter,
    postgres_a_side_of_none_reads_no_parameter
);
