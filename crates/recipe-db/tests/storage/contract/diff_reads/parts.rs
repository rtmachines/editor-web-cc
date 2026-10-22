//! Contract cases of `DiffReads::change_flags`: one part of a parameter alone (task 7.1).
//!
//! The old version holds one parameter for each case. The new version changes one part of it.

use recipe_domain::recipe::{StoredLimit, StoredParameter, Translation};
use recipe_domain::text::{LongText, Text};
use serde_json::json;

use super::fixtures::{
    ADDED, Compact, DEFINITION, DiffPlant, LIMITS, TRANSLATIONS, UNCHANGED, VALUE, checked_flags,
    compact, diff_plant, on_new_side, seed_versions, typed,
};
use crate::contract::harness::Harness;
use crate::contract::recipe_seed::{range, translation};

/// The limit rows `[1]` to `[3]` of an array with the bounds 0 and 10.
fn element_rows() -> Vec<StoredLimit> {
    ["[1]", "[2]", "[3]"]
        .into_iter()
        .map(|path| range(path, "0", "10"))
        .collect()
}

/// The old side: `A` (unchanged), `B` (definition), `C` (value in a struct in a struct),
/// `D` (limit `[*]`), `E` (limit of one element), `F` (limit mode), `G` (translations).
fn old_side(plant: &DiffPlant) -> Vec<StoredParameter> {
    let table = json!(["1", "2", "3"]);
    let all = vec![range("[*]", "0", "10")];
    vec![
        StoredParameter {
            limits: vec![range("", "0", "10")],
            ..typed(plant, (1, "A"), plant.int, json!("5"))
        },
        typed(plant, (2, "B"), plant.int, json!("5")),
        StoredParameter {
            limits: vec![range(".zone.setpoint", "0", "100")],
            ..typed(
                plant,
                (3, "C"),
                plant.line,
                json!({"zone": {"setpoint": "1.5"}}),
            )
        },
        StoredParameter {
            limits: all.clone(),
            ..typed(plant, (4, "D"), plant.table, table.clone())
        },
        StoredParameter {
            limits: element_rows(),
            ..typed(plant, (5, "E"), plant.table, table.clone())
        },
        StoredParameter {
            limits: all,
            ..typed(plant, (6, "F"), plant.table, table)
        },
        StoredParameter {
            translations: vec![translation("pl", "Nazwa", None)],
            ..typed(plant, (7, "G"), plant.int, json!("5"))
        },
    ]
}

/// Return the translations of `G` on the new side: `pl` changed and `de` added.
fn new_translations() -> Vec<Translation> {
    vec![
        translation("pl", "Inna", None),
        translation("de", "Name", Some("Text")),
    ]
}

/// The new side: one change for each key of `old_side`, in the same order.
fn new_side(old: &[StoredParameter]) -> Vec<StoredParameter> {
    let copy = |index: usize| on_new_side(&old[index]);
    let mut element = copy(4);
    element.limits[1] = range("[2]", "0", "20");
    vec![
        copy(0),
        StoredParameter {
            name_en: Text::parse("Other").unwrap(),
            description_en: LongText::parse("Other text").unwrap(),
            ..copy(1)
        },
        StoredParameter {
            value: json!({"zone": {"setpoint": "2.5"}}).into(),
            ..copy(2)
        },
        StoredParameter {
            limits: vec![range("[*]", "0", "20")],
            ..copy(3)
        },
        element,
        StoredParameter {
            limits: element_rows(),
            ..copy(5)
        },
        StoredParameter {
            translations: new_translations(),
            ..copy(6)
        },
    ]
}

/// The rows of the case list: one flag for each part, in the order of the tags.
fn expected_parts() -> Vec<Compact> {
    vec![
        ("A".into(), UNCHANGED, (0, 0, 0)),
        ("B".into(), DEFINITION, (0, 2, 0)),
        ("C".into(), VALUE, (0, 0, 0)),
        ("D".into(), LIMITS, (1, 0, 0)),
        ("E".into(), LIMITS, (1, 0, 0)),
        ("F".into(), LIMITS, (4, 0, 0)),
        ("G".into(), TRANSLATIONS, (0, 0, 2)),
    ]
}

async fn each_part_of_a_parameter_sets_its_own_flag(h: &dyn Harness) {
    let plant = diff_plant(h, 70, "M70").await;
    let old = old_side(&plant);
    let versions = seed_versions(h, &plant.plant, 70, (&old, &new_side(&old))).await;

    let flags = checked_flags(h, plant.plant.machine, Some(versions.old), versions.new).await;

    assert_eq!(compact(&flags), expected_parts());
}

contract_case!(
    each_part_of_a_parameter_sets_its_own_flag,
    memory_each_part_of_a_parameter_sets_its_own_flag,
    postgres_each_part_of_a_parameter_sets_its_own_flag
);

async fn a_version_against_itself_gives_every_key_unchanged(h: &dyn Harness) {
    let plant = diff_plant(h, 72, "M72").await;
    let old = old_side(&plant);
    let versions = seed_versions(h, &plant.plant, 72, (&old, &[])).await;

    let flags = checked_flags(h, plant.plant.machine, Some(versions.old), versions.old).await;

    let tags = ["A", "B", "C", "D", "E", "F", "G"];
    let expected: Vec<Compact> = tags
        .into_iter()
        .map(|tag| (tag.to_string(), UNCHANGED, (0, 0, 0)))
        .collect();
    assert_eq!(compact(&flags), expected);
}

contract_case!(
    a_version_against_itself_gives_every_key_unchanged,
    memory_a_version_against_itself_gives_every_key_unchanged,
    postgres_a_version_against_itself_gives_every_key_unchanged
);

async fn a_first_approval_gives_every_key_only_in_the_new_version(h: &dyn Harness) {
    let plant = diff_plant(h, 74, "M74").await;
    let old = old_side(&plant);
    let versions = seed_versions(h, &plant.plant, 74, (&old, &[])).await;

    let flags = checked_flags(h, plant.plant.machine, None, versions.old).await;

    let bare = [false, false, false, false, false, true];
    let expected: Vec<Compact> = vec![
        ("A".into(), ADDED, (1, 0, 0)),
        ("B".into(), bare, (0, 0, 0)),
        ("C".into(), ADDED, (1, 0, 0)),
        ("D".into(), ADDED, (1, 0, 0)),
        ("E".into(), ADDED, (3, 0, 0)),
        ("F".into(), ADDED, (1, 0, 0)),
        ("G".into(), bare, (0, 0, 0)),
    ];
    assert_eq!(compact(&flags), expected);
}

contract_case!(
    a_first_approval_gives_every_key_only_in_the_new_version,
    memory_a_first_approval_gives_every_key_only_in_the_new_version,
    postgres_a_first_approval_gives_every_key_only_in_the_new_version
);
