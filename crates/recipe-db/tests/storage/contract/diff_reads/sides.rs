//! Contract cases of `DiffReads::change_flags`: keys on one side and type changes (task 7.1).
//!
//! A changed `plc_type_id` sets `value`. It sets `limits` when one side has a limit row, and the
//! limit row count is then the number of distinct leaf paths of both sides (design D4).

use recipe_domain::recipe::StoredParameter;
use serde_json::json;

use super::fixtures::{
    ADDED, Compact, DiffPlant, REMOVED, RETYPED, RETYPED_BARE, UNCHANGED, checked_flags, compact,
    diff_plant, on_new_side, seed_versions, typed,
};
use crate::contract::harness::Harness;
use crate::contract::recipe_seed::range;

/// The old side: `I` (old only), `J` (`INT` with an equal limit row after the change), `K`
/// (`ARRAY OF INT`), `L` (`INT` without a limit), and `M` (unchanged).
///
/// The memory seed has no `LTIME`, so the type changes of the case are `INT` to `DINT`.
fn old_side(plant: &DiffPlant) -> Vec<StoredParameter> {
    vec![
        StoredParameter {
            limits: vec![range(".zone.setpoint", "0", "100")],
            ..typed(
                plant,
                (12, "I"),
                plant.line,
                json!({"zone": {"setpoint": "1.5"}}),
            )
        },
        StoredParameter {
            limits: vec![range("", "0", "1000")],
            ..typed(plant, (13, "J"), plant.int, json!("500"))
        },
        StoredParameter {
            limits: vec![range("[1]", "0", "10"), range("[2]", "0", "10")],
            ..typed(plant, (14, "K"), plant.table, json!(["1", "2", "3"]))
        },
        typed(plant, (15, "L"), plant.int, json!("5")),
        StoredParameter {
            limits: vec![range("", "0", "10")],
            ..typed(plant, (16, "M"), plant.int, json!("5"))
        },
    ]
}

/// The new side: `H` (new only) and the changes of `J`, `K`, and `L`. `M` stays the same.
fn new_side(plant: &DiffPlant, old: &[StoredParameter]) -> Vec<StoredParameter> {
    let copy = |index: usize| on_new_side(&old[index]);
    vec![
        StoredParameter {
            limits: vec![range("", "0", "10")],
            ..typed(plant, (11, "H"), plant.int, json!("5"))
        },
        StoredParameter {
            plc_type: plant.plant.dint,
            ..copy(1)
        },
        StoredParameter {
            plc_type: plant.dint_table,
            limits: vec![range("[2]", "0", "10"), range("[3]", "0", "10")],
            ..copy(2)
        },
        StoredParameter {
            plc_type: plant.plant.dint,
            ..copy(3)
        },
        copy(4),
    ]
}

async fn keys_on_one_side_and_type_changes_follow_the_count_rule(h: &dyn Harness) {
    let plant = diff_plant(h, 76, "M76").await;
    let old = old_side(&plant);
    let new = new_side(&plant, &old);
    let versions = seed_versions(h, &plant.plant, 76, (&old, &new)).await;

    let flags = checked_flags(h, plant.plant.machine, Some(versions.old), versions.new).await;

    let expected: Vec<Compact> = vec![
        ("H".into(), ADDED, (1, 0, 0)),
        ("I".into(), REMOVED, (1, 0, 0)),
        ("J".into(), RETYPED, (1, 1, 0)),
        ("K".into(), RETYPED, (3, 1, 0)),
        ("L".into(), RETYPED_BARE, (0, 1, 0)),
        ("M".into(), UNCHANGED, (0, 0, 0)),
    ];
    assert_eq!(compact(&flags), expected);
}

contract_case!(
    keys_on_one_side_and_type_changes_follow_the_count_rule,
    memory_keys_on_one_side_and_type_changes_follow_the_count_rule,
    postgres_keys_on_one_side_and_type_changes_follow_the_count_rule
);
