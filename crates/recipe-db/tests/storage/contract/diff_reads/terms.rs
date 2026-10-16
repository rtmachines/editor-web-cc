//! Contract cases of `DiffReads::change_flags`: one key for each compare term of the flags.
//!
//! Each key changes one field that the statement of the flags compares. A dropped compare term
//! gives a flag or a count that differs from the table of the case and from the domain diff
//! (ADR-0050, Risks).

use recipe_domain::plant::{Machine, PlcPlatform, PlcTypeId, Primitive, S7Address, StringKind};
use recipe_domain::recipe::{Ordinal, ParameterTag, StoredParameter};
use serde_json::json;

use super::fixtures::{
    Compact, DEFINITION, DiffPlant, LIMITS, TRANSLATIONS, UNCHANGED, checked_flags, compact,
    diff_plant, on_new_side, seed_versions, typed,
};
use crate::contract::harness::Harness;
use crate::contract::recipe_seed::{Plant, length, parameter, range, translation};
use crate::contract::seed::{
    code, ensure, insert, machine, machine_id, name, seed_type, string_record, type_id, unit_id,
};

/// Insert `STRING(20)` for the machine number `n` and return its identifier.
async fn string_type(h: &dyn Harness, n: u128, code_text: &str) -> PlcTypeId {
    let built = machine(n, code_text);
    let record = string_record(&built, type_id(5), StringKind::String, 20);
    ensure(h, built.id(), record).await.id
}

/// The old side: `T` (tag and ordinal), `N` (minimum), `S` (length of a `STRING` leaf), `D`
/// (description of a translation), and `L` (a language that the new side removes).
fn old_side(plant: &DiffPlant, string: PlcTypeId) -> Vec<StoredParameter> {
    vec![
        typed(plant, (51, "T"), plant.int, json!("5")),
        StoredParameter {
            limits: vec![range("", "0", "10")],
            ..typed(plant, (52, "N"), plant.int, json!("5"))
        },
        StoredParameter {
            limits: vec![length("", 10)],
            ..typed(plant, (53, "S"), string, json!("abc"))
        },
        StoredParameter {
            translations: vec![translation("pl", "Nazwa", Some("Opis"))],
            ..typed(plant, (54, "D"), plant.int, json!("5"))
        },
        StoredParameter {
            translations: vec![
                translation("de", "Name", None),
                translation("pl", "Nazwa", None),
            ],
            ..typed(plant, (55, "L"), plant.int, json!("5"))
        },
    ]
}

/// The new side: one changed field for each key of `old_side`, in the same order.
fn new_side(old: &[StoredParameter]) -> Vec<StoredParameter> {
    let copy = |index: usize| on_new_side(&old[index]);
    vec![
        StoredParameter {
            tag: ParameterTag::parse("T2").unwrap(),
            ordinal: Ordinal::parse(3).unwrap(),
            ..copy(0)
        },
        StoredParameter {
            limits: vec![range("", "1", "10")],
            ..copy(1)
        },
        StoredParameter {
            limits: vec![length("", 20)],
            ..copy(2)
        },
        StoredParameter {
            translations: vec![translation("pl", "Nazwa", Some("Inny opis"))],
            ..copy(3)
        },
        StoredParameter {
            translations: vec![translation("pl", "Nazwa", None)],
            ..copy(4)
        },
    ]
}

async fn each_compared_field_sets_its_flag_and_its_count(h: &dyn Harness) {
    let plant = diff_plant(h, 86, "M86").await;
    let string = string_type(h, 86, "M86").await;
    let old = old_side(&plant, string);
    let versions = seed_versions(h, &plant.plant, 86, (&old, &new_side(&old))).await;

    let flags = checked_flags(h, plant.plant.machine, Some(versions.old), versions.new).await;

    let expected: Vec<Compact> = vec![
        ("D".into(), TRANSLATIONS, (0, 0, 1)),
        ("L".into(), TRANSLATIONS, (0, 0, 1)),
        ("N".into(), LIMITS, (1, 0, 0)),
        ("S".into(), LIMITS, (1, 0, 0)),
        ("T2".into(), DEFINITION, (0, 2, 0)),
    ];
    assert_eq!(compact(&flags), expected);
}

contract_case!(
    each_compared_field_sets_its_flag_and_its_count,
    memory_each_compared_field_sets_its_flag_and_its_count,
    postgres_each_compared_field_sets_its_flag_and_its_count
);

/// Insert the machine number `n` of the platform `S7_1500` with its unit `U1` and `DINT`.
async fn s7_plant(h: &dyn Harness, n: u128, code_text: &str) -> Plant {
    let unit = unit_id(n.checked_mul(10).unwrap());
    let built = Machine::create(
        machine_id(n),
        code(code_text),
        name("Press"),
        PlcPlatform::S7_1500,
        (unit, name("Main unit")),
    );
    insert(h, &built).await;
    Plant {
        machine: built.id(),
        unit,
        dint: seed_type(h, built.id(), Primitive::Dint).await,
    }
}

/// A parameter of `DINT` with the S7 address `address`. The number `n` sets the key.
fn addressed(plant: &Plant, (n, tag): (u128, &str), address: &str) -> StoredParameter {
    StoredParameter {
        s7_address: Some(S7Address::parse(address).unwrap()),
        ..parameter(plant, n, tag)
    }
}

async fn a_changed_s7_address_sets_the_definition(h: &dyn Harness) {
    let plant = s7_plant(h, 88, "M88").await;
    let old = vec![
        addressed(&plant, (61, "P"), "DB1.DBD0"),
        addressed(&plant, (62, "Q"), "DB1.DBD4"),
    ];
    let new = vec![
        StoredParameter {
            s7_address: Some(S7Address::parse("DB1.DBD8").unwrap()),
            ..on_new_side(&old[0])
        },
        on_new_side(&old[1]),
    ];
    let versions = seed_versions(h, &plant, 88, (&old, &new)).await;

    let flags = checked_flags(h, plant.machine, Some(versions.old), versions.new).await;

    let expected: Vec<Compact> = vec![
        ("P".into(), DEFINITION, (0, 1, 0)),
        ("Q".into(), UNCHANGED, (0, 0, 0)),
    ];
    assert_eq!(compact(&flags), expected);
}

contract_case!(
    a_changed_s7_address_sets_the_definition,
    memory_a_changed_s7_address_sets_the_definition,
    postgres_a_changed_s7_address_sets_the_definition
);
