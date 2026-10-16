//! Tests of the new member leaves of `GetVersion` (design D4, ADR-0018, ADR-0043 (l)).
//!
//! Each list equals the leaves for which `RebindTypeRevision` without values gives `E-RCP-028`.

use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape,
    TypeName,
};
use recipe_domain::recipe::{RecipeError, StoredParameter, StoredValue};
use serde_json::{Value, json};

use super::super::test_world::seed_users;
use super::super::versions::{GetVersion, GetVersionInput, VersionView};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, actor, meta, mid, parameter_on, reference, tid, uid, vid};
use crate::recipe::rebind::fixtures::{primitive_id, typed_struct_row};
use crate::recipe::test_support::DRAFT_VERSION;
use crate::recipe::{RebindTypeRevision, RebindTypeRevisionInput, StructRevision};

/// `ST_Pair@1` with the members `a` and `b`.
const PAIR_1: u128 = 801;
/// `ST_Zone@1` with the members `setpoint`, `mode`, and `old_name`.
const ZONE_1: u128 = 802;
/// `ST_Zone@2`: `setpoint` stays, `mode` becomes `DINT`, `old_name` becomes `new_name`, and the
/// struct member `sub` of type `ST_Pair@1` is new.
const ZONE_2: u128 = 803;
/// `ARRAY[1..2] OF ST_Zone@1`.
const ZONES: u128 = 804;
/// `ST_Line@1` with the member `zone` of type `ST_Zone@1`.
const LINE: u128 = 805;
/// `ST_Cell@1` with the member `line` of type `ST_Line@1`.
const CELL: u128 = 806;
/// The new leaves of one zone, in member order.
const ZONE_LEAVES: [&str; 4] = [".mode", ".new_name", ".sub.a", ".sub.b"];

/// An array row of the machine 1 with the bounds `1..=2`.
fn array_row(id: u128, element: u128) -> PlcType {
    let dimensions = ArrayDimensions::parse(vec![ArrayDimension::parse(1, 2).unwrap()]).unwrap();
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PlcPlatform::TwinCat3]),
        shape: RecordShape::Array {
            element: tid(element),
            dimensions,
        },
    })
    .unwrap()
}

/// Store the rows of the test catalog.
fn seed_rows(fixture: &Fixture) {
    let platform = PlcPlatform::TwinCat3;
    let int = primitive_id(Primitive::Int);
    let dint = primitive_id(Primitive::Dint);
    let rows = [
        typed_struct_row(
            platform,
            (PAIR_1, "ST_Pair", 1),
            None,
            &[("a", int), ("b", int)],
        ),
        typed_struct_row(
            platform,
            (ZONE_1, "ST_Zone", 1),
            None,
            &[("setpoint", int), ("mode", int), ("old_name", int)],
        ),
        typed_struct_row(
            platform,
            (ZONE_2, "ST_Zone", 2),
            Some(ZONE_1),
            &[
                ("setpoint", int),
                ("mode", dint),
                ("new_name", int),
                ("sub", tid(PAIR_1)),
            ],
        ),
        array_row(ZONES, ZONE_1),
        typed_struct_row(
            platform,
            (LINE, "ST_Line", 1),
            None,
            &[("zone", tid(ZONE_1))],
        ),
    ];
    for row in &rows {
        fixture.store.seed_type(row);
    }
}

/// Store the parameter `n` with the tag, the type, and the value.
fn seed_parameter(fixture: &Fixture, (n, tag): (u128, &str), (plc_type, value): (u128, Value)) {
    let parameter = StoredParameter {
        plc_type: tid(plc_type),
        value: StoredValue::from(value),
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::TwinCat3, n, tag, uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &parameter);
}

/// A draft with two flat zones, an array of zones, and a line that holds a zone as a member.
fn draft() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    seed_users(&fixture);
    seed_rows(&fixture);
    let zone = json!({"setpoint": 1, "mode": 1, "old_name": 1});
    seed_parameter(&fixture, (11, "ZONE_A"), (ZONE_1, zone.clone()));
    seed_parameter(&fixture, (12, "ZONE_B"), (ZONE_1, zone.clone()));
    seed_parameter(
        &fixture,
        (13, "ZONES"),
        (ZONES, json!([zone.clone(), zone.clone()])),
    );
    seed_parameter(&fixture, (14, "LINE"), (LINE, json!({"zone": zone})));
    fixture
}

/// The bound `form.rebind_leaves` of `reliability.md` section 7.11.
const BOUND: usize = 16_384;

/// Return the view of `GetVersion` of the draft `R-1` for `role` with the bound `bound`.
async fn view_of(fixture: &Fixture, role: Role, bound: usize) -> VersionView {
    let input = GetVersionInput {
        reference: reference("R-1"),
        number: 1,
        rebind_bound: bound,
    };
    GetVersion::new(fixture.context())
        .execute(&actor(role), &meta(), input)
        .await
        .unwrap()
}

/// Return the new leaves of each outdated parameter of the draft, by tag.
async fn leaves_of_get_version(fixture: &Fixture) -> BTreeMap<String, Option<Vec<String>>> {
    let view = view_of(fixture, Role::ProcessEngineer, BOUND).await;
    view.outdated
        .iter()
        .map(|item| {
            let paths = item.rebind_leaves.as_ref().map(|found| {
                found
                    .leaves()
                    .iter()
                    .map(|leaf| leaf.path().to_string())
                    .collect()
            });
            (item.tag.to_string(), paths)
        })
        .collect()
}

/// Return the leaves of the `E-RCP-028` entries of a rebind without values, by tag.
async fn leaves_of_the_command(fixture: &Fixture) -> BTreeMap<String, Vec<String>> {
    let input = RebindTypeRevisionInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        from: tid(ZONE_1),
        to: tid(ZONE_2),
        new_member_values: BTreeMap::new(),
    };
    let error = RebindTypeRevision::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input)
        .await
        .unwrap_err();
    let AppError::Recipes(list) = error else {
        return BTreeMap::new();
    };
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in list.iter() {
        if let RecipeError::RebindNeedsValue { tag, leaf } = entry {
            found
                .entry(tag.to_string())
                .or_default()
                .push(leaf.to_string());
        }
    }
    found
}

#[tokio::test]
async fn get_version_gives_each_outdated_parameter_the_new_leaves_of_its_rebind() {
    let fixture = draft();

    let found = leaves_of_get_version(&fixture).await;

    let zone: Vec<String> = ZONE_LEAVES.iter().map(|path| (*path).to_owned()).collect();
    let zones: Vec<String> = ["[1]", "[2]"]
        .iter()
        .flat_map(|element| {
            ZONE_LEAVES
                .iter()
                .map(move |path| format!("{element}{path}"))
        })
        .collect();
    assert_eq!(found.get("ZONE_A"), Some(&Some(zone.clone())));
    assert_eq!(found.get("ZONE_B"), Some(&Some(zone)));
    assert_eq!(found.get("ZONES"), Some(&Some(zones)));
    assert_eq!(
        found.get("LINE"),
        Some(&None),
        "a struct member does not move"
    );
}

#[tokio::test]
async fn get_version_leaves_equal_the_leaves_that_the_command_asks_for() {
    let fixture = draft();

    let query = leaves_of_get_version(&fixture).await;
    let command = leaves_of_the_command(&fixture).await;

    let moved: BTreeMap<String, Vec<String>> = query
        .into_iter()
        .filter_map(|(tag, paths)| paths.map(|paths| (tag, paths)))
        .collect();
    assert_eq!(moved, command);
    assert_eq!(moved.len(), 3);
}

#[tokio::test]
async fn a_reader_without_parameter_edit_gets_the_outdated_parameters_without_new_leaves() {
    let fixture = draft();

    let view = view_of(&fixture, Role::Operator, BOUND).await;

    assert_eq!(view.outdated.len(), 4);
    assert!(
        view.outdated
            .iter()
            .all(|item| item.rebind_leaves.is_none())
    );
    assert_eq!((view.rebind_sum, view.rebind_over_bound), (0, false));
    assert!(fixture.sink.events().is_empty(), "no security event");
}

#[tokio::test]
async fn a_process_engineer_gets_the_sum_of_the_new_leaves_under_the_bound() {
    let fixture = draft();

    let view = view_of(&fixture, Role::ProcessEngineer, BOUND).await;

    // ZONE_A and ZONE_B have 4 new leaves each, ZONES has 8, and LINE does not move.
    assert_eq!((view.rebind_sum, view.rebind_over_bound), (16, false));
}

/// A draft with `count` outdated zones `Z00`, `Z01`, and so on. Each zone needs 4 new leaves.
fn many_zones(count: u128) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    seed_users(&fixture);
    seed_rows(&fixture);
    let zone = json!({"setpoint": 1, "mode": 1, "old_name": 1});
    for n in 0..count {
        let tag = format!("Z{n:02}");
        let key = n.checked_add(20).unwrap();
        seed_parameter(&fixture, (key, &tag), (ZONE_1, zone.clone()));
    }
    fixture
}

#[tokio::test]
async fn the_query_stops_at_the_first_parameter_that_passes_the_bound() {
    let fixture = many_zones(30);

    let view = view_of(&fixture, Role::ProcessEngineer, 5).await;

    // The query computes 4 leaves of Z00 and 4 of Z01. The sum 8 passes the bound 5, so the
    // query computes no leaves of the other 28 zones. The full sum would be 120.
    assert_eq!(view.outdated.len(), 30);
    assert_eq!((view.rebind_sum, view.rebind_over_bound), (8, true));
    assert_eq!(
        view.rebind_bound, 5,
        "the page shows the bound in the notice"
    );
    assert!(
        view.outdated
            .iter()
            .all(|item| item.rebind_leaves.is_none()),
        "over the bound, no form needs a leaf"
    );
}

#[tokio::test]
async fn a_sum_equal_to_the_bound_does_not_pass_it() {
    let fixture = many_zones(2);

    let view = view_of(&fixture, Role::ProcessEngineer, 8).await;

    assert_eq!((view.rebind_sum, view.rebind_over_bound), (8, false));
    assert!(
        view.outdated
            .iter()
            .all(|item| item.rebind_leaves.is_some())
    );
}

#[tokio::test]
async fn each_reader_gets_the_struct_that_holds_a_nested_outdated_struct() {
    let fixture = draft();
    fixture.store.seed_type(&typed_struct_row(
        PlcPlatform::TwinCat3,
        (CELL, "ST_Cell", 1),
        None,
        &[("line", tid(LINE))],
    ));
    let zone = json!({"setpoint": 1, "mode": 1, "old_name": 1});
    seed_parameter(
        &fixture,
        (15, "CELL"),
        (CELL, json!({"line": {"zone": zone}})),
    );

    let view = view_of(&fixture, Role::Operator, BOUND).await;

    let holders: BTreeMap<String, Option<StructRevision>> = view
        .outdated
        .iter()
        .map(|item| (item.tag.to_string(), item.holder.clone()))
        .collect();
    let line = StructRevision {
        name: TypeName::parse("ST_Line").unwrap(),
        revision: 1,
    };
    assert_eq!(holders.get("LINE"), Some(&Some(line.clone())));
    assert_eq!(holders.get("CELL"), Some(&Some(line)), "the nearest struct");
    for tag in ["ZONE_A", "ZONE_B", "ZONES"] {
        assert_eq!(
            holders.get(tag),
            Some(&None),
            "{tag} uses the struct directly"
        );
    }
}
