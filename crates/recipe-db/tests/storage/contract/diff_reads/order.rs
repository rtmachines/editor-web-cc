//! Contract case of the order of `DiffReads::change_flags` (design D4).
//!
//! The rows come in the order of the parameter table: the unit path by its numbers, the
//! ordinal, the tag, and the key. A key on both sides takes the order fields of the new side. A
//! key only in the old version takes the order fields of the old side. The tags sort by bytes.
//! The unit path of a key is the current path, also for an archived unit.

use recipe_app::ports::ChangeFlags;
use recipe_domain::plant::{Primitive, UnitId};
use recipe_domain::recipe::{Ordinal, StoredParameter};

use super::fixtures::{checked_flags, on_new_side, seed_versions};
use crate::contract::harness::{Harness, begin, commit};
use crate::contract::recipe_seed::{Plant, parameter, plant};
use crate::contract::seed::{
    U1_1_1, U1_2, U1_10, U2, insert, machine_with_tree, seed_type, tree_unit_id, unit_id,
};

/// Insert the machine number `n` with the unit tree of `machine_with_tree`.
async fn tree_plant(h: &dyn Harness, n: u128, code: &str) -> Plant {
    let built = machine_with_tree(n, code);
    insert(h, &built).await;
    Plant {
        machine: built.id(),
        unit: unit_id(n.checked_mul(10).unwrap()),
        dint: seed_type(h, built.id(), Primitive::Dint).await,
    }
}

/// A parameter in `unit` with `ordinal`. The number `n` sets the key.
fn placed(plant: &Plant, (n, tag): (u128, &str), unit: UnitId, ordinal: u32) -> StoredParameter {
    StoredParameter {
        unit,
        ordinal: Ordinal::parse(ordinal).unwrap(),
        ..parameter(plant, n, tag)
    }
}

async fn the_rows_come_in_the_order_of_the_parameter_table(h: &dyn Harness) {
    let plant = tree_plant(h, 78, "M78").await;
    let unit = |offset| tree_unit_id(78, offset);
    let placed_new = [
        placed(&plant, (21, "A"), unit(U1_10), 0),
        placed(&plant, (22, "Y"), unit(U1_2), 1),
        placed(&plant, (23, "B"), unit(U1_2), 1),
        placed(&plant, (24, "Z"), unit(U1_2), 0),
        placed(&plant, (25, "C"), plant.unit, 5),
    ];
    let moved = StoredParameter {
        unit: unit(U2),
        ..placed_new[2].clone()
    };
    let old = vec![moved, placed(&plant, (26, "OLD"), unit(U1_1_1), 0)];
    let new: Vec<StoredParameter> = placed_new.iter().map(on_new_side).collect();
    let versions = seed_versions(h, &plant, 78, (&old, &new)).await;

    let flags = checked_flags(h, plant.machine, Some(versions.old), versions.new).await;

    let order: Vec<(String, String, u32)> = flags
        .iter()
        .map(|row| {
            (
                row.tag.to_string(),
                row.unit_path.to_string(),
                row.ordinal.value(),
            )
        })
        .collect();
    let row = |tag: &str, path: &str, ordinal: u32| (tag.to_string(), path.to_string(), ordinal);
    assert_eq!(
        order,
        vec![
            row("C", "U1", 5),
            row("OLD", "U1.1.1", 0),
            row("Z", "U1.2", 0),
            row("B", "U1.2", 1),
            row("Y", "U1.2", 1),
            row("A", "U1.10", 0),
        ]
    );
}

contract_case!(
    the_rows_come_in_the_order_of_the_parameter_table,
    memory_the_rows_come_in_the_order_of_the_parameter_table,
    postgres_the_rows_come_in_the_order_of_the_parameter_table
);

/// The tag, `only_old`, and `only_new` of each row.
fn sides_by_tag(flags: &[ChangeFlags]) -> Vec<(String, bool, bool)> {
    flags
        .iter()
        .map(|row| (row.tag.to_string(), row.only_old, row.only_new))
        .collect()
}

async fn tags_sort_by_bytes_and_equal_tags_sort_by_key(h: &dyn Harness) {
    let plant = plant(h, 90, "M90").await;
    let both = [parameter(&plant, 71, "AB"), parameter(&plant, 72, "A_A")];
    let old = vec![both[0].clone(), both[1].clone(), parameter(&plant, 74, "R")];
    let mut new: Vec<StoredParameter> = both.iter().map(on_new_side).collect();
    new.push(parameter(&plant, 73, "R"));
    let versions = seed_versions(h, &plant, 90, (&old, &new)).await;

    let flags = checked_flags(h, plant.machine, Some(versions.old), versions.new).await;

    // Byte order: "AB" < "A_A". The new key 73 sorts before the old key 74 with the same tag.
    let row = |tag: &str, only_old, only_new| (tag.to_string(), only_old, only_new);
    assert_eq!(
        sides_by_tag(&flags),
        vec![
            row("AB", false, false),
            row("A_A", false, false),
            row("R", false, true),
            row("R", true, false),
        ]
    );
}

contract_case!(
    tags_sort_by_bytes_and_equal_tags_sort_by_key,
    memory_tags_sort_by_bytes_and_equal_tags_sort_by_key,
    postgres_tags_sort_by_bytes_and_equal_tags_sort_by_key
);

async fn a_key_in_an_archived_unit_keeps_the_path_of_the_unit(h: &dyn Harness) {
    let plant = tree_plant(h, 92, "M92").await;
    let archived = tree_unit_id(92, U1_1_1);
    let kept = parameter(&plant, 82, "K");
    let old = vec![kept.clone(), placed(&plant, (81, "OLD"), archived, 0)];
    let versions = seed_versions(h, &plant, 92, (&old, &[on_new_side(&kept)])).await;
    let mut tx = begin(h).await;
    tx.plant().archive_units(&[archived]).await.unwrap();
    commit(h, tx, plant.machine).await;

    let flags = checked_flags(h, plant.machine, Some(versions.old), versions.new).await;

    let paths: Vec<(String, String)> = flags
        .iter()
        .map(|row| (row.tag.to_string(), row.unit_path.to_string()))
        .collect();
    assert_eq!(
        paths,
        vec![
            ("K".to_string(), "U1".to_string()),
            ("OLD".to_string(), "U1.1.1".to_string()),
        ]
    );
}

contract_case!(
    a_key_in_an_archived_unit_keeps_the_path_of_the_unit,
    memory_a_key_in_an_archived_unit_keeps_the_path_of_the_unit,
    postgres_a_key_in_an_archived_unit_keeps_the_path_of_the_unit
);
