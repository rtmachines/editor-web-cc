use time::OffsetDateTime;
use uuid::Uuid;

use super::*;

pub(super) fn uid(n: u128) -> UnitId {
    UnitId::from(Uuid::from_u128(n))
}

pub(super) fn path(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

pub(super) fn name(text: &str) -> Text {
    Text::parse(text).unwrap()
}

pub(super) fn ts(seconds: i64) -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

fn stored_unit(id: u128, text: &str, archived: Option<i64>) -> StoredUnit {
    StoredUnit {
        id: uid(id),
        path: path(text),
        name: name(text),
        archived_at: archived.map(ts),
    }
}

fn stored(units: Vec<StoredUnit>) -> StoredMachine {
    StoredMachine {
        id: MachineId::from(Uuid::from_u128(900)),
        code: MachineCode::parse("M1").unwrap(),
        name: name("Press"),
        platform: PlcPlatform::S7_1500,
        archived_at: None,
        units,
    }
}

fn paths(machine: &Machine) -> Vec<String> {
    machine.units().map(|u| u.path().to_string()).collect()
}

#[test]
fn stored_machine_with_a_valid_tree_builds_and_lists_units_in_path_order() {
    let machine = Machine::try_from(stored(vec![
        stored_unit(3, "U1.2.1", None),
        stored_unit(1, "U1", None),
        stored_unit(2, "U1.2", None),
        stored_unit(4, "U1.10", None),
    ]))
    .unwrap();
    assert_eq!(paths(&machine), vec!["U1", "U1.2", "U1.2.1", "U1.10"]);
    assert_eq!(machine.code().number(), 1);
    assert_eq!(machine.platform(), PlcPlatform::S7_1500);
    assert!(machine.is_live());
    assert_eq!(machine.archived_at(), None);
    assert_eq!(machine.name().as_str(), "Press");
    assert_eq!(machine.id(), MachineId::from(Uuid::from_u128(900)));
    let first = machine.units().next().unwrap();
    assert_eq!(first.id(), uid(1));
    assert_eq!(first.name().as_str(), "U1");
    assert!(first.is_live());
    assert_eq!(first.archived_at(), None);
}

#[test]
fn stored_machine_with_a_duplicate_unit_path_fails_with_duplicate_unit_path() {
    let result = Machine::try_from(stored(vec![
        stored_unit(1, "U1", None),
        stored_unit(2, "U1.2", None),
        stored_unit(3, "U1.2", None),
    ]));
    assert!(matches!(
        result,
        Err(PlantError::DuplicateUnitPath { path: p }) if p == path("U1.2")
    ));
}

#[test]
fn stored_machine_with_a_duplicate_unit_id_fails_with_duplicate_unit_path_of_the_second_unit() {
    let result = Machine::try_from(stored(vec![
        stored_unit(1, "U1", None),
        stored_unit(1, "U2", None),
    ]));
    assert!(matches!(
        result,
        Err(PlantError::DuplicateUnitPath { path: p }) if p == path("U2")
    ));
}

#[test]
fn stored_machine_with_a_unit_without_its_parent_fails_with_parent_not_found() {
    let result = Machine::try_from(stored(vec![
        stored_unit(1, "U1", None),
        stored_unit(2, "U1.2.1", None),
    ]));
    assert!(matches!(
        result,
        Err(PlantError::ParentNotFound { path: p }) if p == path("U1.2.1")
    ));
}

#[test]
fn stored_machine_with_a_live_unit_under_an_archived_parent_fails_with_unit_archived() {
    let result = Machine::try_from(stored(vec![
        stored_unit(1, "U1", None),
        stored_unit(2, "U1.2", Some(10)),
        stored_unit(3, "U1.2.1", None),
    ]));
    assert!(matches!(
        result,
        Err(PlantError::UnitArchived { path: p }) if p == path("U1.2")
    ));
}

#[test]
fn stored_machine_without_a_live_main_unit_fails_with_last_main_unit() {
    let result = Machine::try_from(stored(vec![
        stored_unit(1, "U1", Some(10)),
        stored_unit(2, "U1.2", Some(10)),
    ]));
    assert!(matches!(
        result,
        Err(PlantError::LastMainUnit { code, path: p })
            if code.number() == 1 && p == path("U1")
    ));
}

#[test]
fn stored_machine_without_any_unit_fails_with_last_main_unit() {
    let result = Machine::try_from(stored(vec![]));
    assert!(matches!(result, Err(PlantError::LastMainUnit { .. })));
}

#[test]
fn stored_machine_with_an_archived_child_under_an_archived_parent_builds() {
    let machine = Machine::try_from(stored(vec![
        stored_unit(1, "U1", None),
        stored_unit(2, "U1.2", Some(10)),
        stored_unit(3, "U1.2.1", Some(10)),
    ]))
    .unwrap();
    assert_eq!(machine.units().filter(|u| u.is_live()).count(), 1);
}

#[test]
fn stored_machine_with_an_archived_main_unit_next_to_a_live_main_unit_builds() {
    let machine = Machine::try_from(stored(vec![
        stored_unit(1, "U1", Some(10)),
        stored_unit(2, "U2", None),
    ]))
    .unwrap();
    assert_eq!(paths(&machine), vec!["U1", "U2"]);
    assert_eq!(machine.units().next().unwrap().archived_at(), Some(ts(10)));
}

#[test]
fn duplicate_path_is_reported_before_a_missing_parent() {
    let result = Machine::try_from(stored(vec![
        stored_unit(1, "U1", None),
        stored_unit(2, "U1.5.1", None),
        stored_unit(3, "U1", None),
    ]));
    assert!(matches!(result, Err(PlantError::DuplicateUnitPath { .. })));
}

#[test]
fn missing_parent_is_reported_before_an_archived_parent() {
    let result = Machine::try_from(stored(vec![
        stored_unit(1, "U1", None),
        stored_unit(2, "U1.2", Some(10)),
        stored_unit(3, "U1.2.1", None),
        stored_unit(4, "U1.7.1", None),
    ]));
    assert!(matches!(
        result,
        Err(PlantError::ParentNotFound { path: p }) if p == path("U1.7.1")
    ));
}
