//! Tests of `Machine::unit`, `rename`, `rename_unit`, and `archive`.

use super::add_unit_tests::new_machine;
use super::tests::{name, path, ts, uid};
use super::*;

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-100").unwrap()
}

/// Return a machine with U1, U1.2 (live), and U1.3 (archived).
fn machine_with_units() -> Machine {
    let mut machine = new_machine();
    machine
        .add_unit(uid(2), Some(&path("U1")), "2", name("Two"))
        .unwrap();
    machine
        .add_unit(uid(3), Some(&path("U1")), "3", name("Three"))
        .unwrap();
    machine.units.get_mut(&path("U1.3")).unwrap().archived_at = Some(ts(7));
    machine
}

fn archived(machine: &Machine) -> Machine {
    let mut copy = machine.clone();
    copy.archived_at = Some(ts(5));
    copy
}

#[test]
fn unit_returns_the_unit_of_a_known_path() {
    let machine = machine_with_units();
    let unit = machine.unit(&path("U1.2")).unwrap();
    assert_eq!(unit.id(), uid(2));
}

#[test]
fn unit_returns_an_archived_unit_too() {
    let machine = machine_with_units();
    assert!(!machine.unit(&path("U1.3")).unwrap().is_live());
}

#[test]
fn unit_returns_none_for_an_unknown_path() {
    assert!(machine_with_units().unit(&path("U1.9")).is_none());
}

#[test]
fn rename_sets_the_new_name() {
    let mut machine = new_machine();
    machine.rename(name("Press 2")).unwrap();
    assert_eq!(machine.name().as_str(), "Press 2");
}

#[test]
fn rename_of_an_archived_machine_fails_with_machine_archived() {
    let mut machine = archived(&new_machine());
    let result = machine.rename(name("Press 2"));
    assert!(matches!(result, Err(PlantError::MachineArchived { code }) if code.number() == 1));
    assert_eq!(machine.name().as_str(), "Press");
}

#[test]
fn rename_unit_sets_the_new_name_of_a_live_unit() {
    let mut machine = machine_with_units();
    machine.rename_unit(&path("U1.2"), name("Renamed")).unwrap();
    assert_eq!(
        machine.unit(&path("U1.2")).unwrap().name().as_str(),
        "Renamed"
    );
}

#[test]
fn rename_unit_of_an_archived_unit_fails_with_unit_archived() {
    let mut machine = machine_with_units();
    let before = machine.clone();
    let result = machine.rename_unit(&path("U1.3"), name("Renamed"));
    assert!(matches!(
        result,
        Err(PlantError::UnitArchived { path: p }) if p == path("U1.3")
    ));
    assert_eq!(machine, before);
}

#[test]
fn rename_unit_of_a_missing_unit_fails_with_unit_not_found() {
    let mut machine = machine_with_units();
    let before = machine.clone();
    let result = machine.rename_unit(&path("U1.9"), name("Renamed"));
    assert!(matches!(
        result,
        Err(PlantError::UnitNotFound { path: p }) if p == path("U1.9")
    ));
    assert_eq!(machine, before);
}

#[test]
fn archive_sets_archived_at_and_keeps_the_units() {
    let mut machine = machine_with_units();
    let units_before: Vec<Unit> = machine.units().cloned().collect();
    machine.archive(ts(9), None).unwrap();
    assert_eq!(machine.archived_at(), Some(ts(9)));
    assert!(!machine.is_live());
    let units_after: Vec<Unit> = machine.units().cloned().collect();
    assert_eq!(units_after, units_before);
}

#[test]
fn archive_with_a_live_draft_fails_with_machine_has_draft_and_changes_nothing() {
    let mut machine = machine_with_units();
    let before = machine.clone();
    let result = machine.archive(ts(9), Some(&reference()));
    assert!(matches!(
        result,
        Err(PlantError::MachineHasDraft { code, reference: r })
            if code.number() == 1 && r == reference()
    ));
    assert_eq!(machine, before);
}

#[test]
fn archive_of_an_archived_machine_fails_with_machine_archived_and_keeps_the_time() {
    let mut machine = archived(&new_machine());
    let result = machine.archive(ts(9), None);
    assert!(matches!(result, Err(PlantError::MachineArchived { .. })));
    assert_eq!(machine.archived_at(), Some(ts(5)));
}

#[test]
fn archive_checks_the_archived_state_before_the_draft() {
    let mut machine = archived(&new_machine());
    let result = machine.archive(ts(9), Some(&reference()));
    assert!(matches!(result, Err(PlantError::MachineArchived { .. })));
}
