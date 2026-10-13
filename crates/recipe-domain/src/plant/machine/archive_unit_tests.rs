//! Tests of `Machine::archive_unit` and `UnitUsage` (ADR-0021).

use super::add_unit_tests::new_machine;
use super::tests::{name, path, ts, uid};
use super::*;

fn reference(text: &str) -> ReferenceNumber {
    ReferenceNumber::parse(text).unwrap()
}

/// Return a machine with these units. The ids differ in order from the paths:
/// U1 = 10, U1.2 = 30, U1.2.1 = 20, U1.3 = 40, U2 = 50.
fn machine() -> Machine {
    let mut machine = new_machine();
    machine.units.get_mut(&path("U1")).unwrap().id = uid(10);
    for (id, parent, segment) in [
        (30, Some("U1"), "2"),
        (20, Some("U1.2"), "1"),
        (40, Some("U1"), "3"),
        (50, None, "2"),
    ] {
        let parent = parent.map(path);
        machine
            .add_unit(uid(id), parent.as_ref(), segment, name("x"))
            .unwrap();
    }
    machine
}

fn archived_paths(machine: &Machine) -> Vec<String> {
    machine
        .units()
        .filter(|u| !u.is_live())
        .map(|u| u.path().to_string())
        .collect()
}

#[test]
fn archive_unit_cascades_to_the_descendants_and_keeps_the_parent_live() {
    let mut machine = machine();
    machine
        .archive_unit(&path("U1.2"), ts(9), &UnitUsage::default())
        .unwrap();
    assert_eq!(archived_paths(&machine), vec!["U1.2", "U1.2.1"]);
    assert_eq!(
        machine.unit(&path("U1.2")).unwrap().archived_at(),
        Some(ts(9))
    );
    assert!(machine.unit(&path("U1")).unwrap().is_live());
    assert!(machine.unit(&path("U1.3")).unwrap().is_live());
}

#[test]
fn archive_unit_returns_the_ids_in_id_order_not_path_order() {
    let mut machine = machine();
    let ids = machine
        .archive_unit(&path("U1.2"), ts(9), &UnitUsage::default())
        .unwrap();
    assert_eq!(ids, vec![uid(20), uid(30)]);
}

#[test]
fn archive_unit_keeps_the_old_time_of_an_archived_descendant() {
    let mut machine = machine();
    machine
        .archive_unit(&path("U1.2.1"), ts(5), &UnitUsage::default())
        .unwrap();
    let ids = machine
        .archive_unit(&path("U1.2"), ts(9), &UnitUsage::default())
        .unwrap();
    assert_eq!(ids, vec![uid(30)]);
    assert_eq!(
        machine.unit(&path("U1.2.1")).unwrap().archived_at(),
        Some(ts(5))
    );
    assert_eq!(
        machine.unit(&path("U1.2")).unwrap().archived_at(),
        Some(ts(9))
    );
}

#[test]
fn archive_unit_of_an_archived_subtree_returns_an_empty_result_and_changes_nothing() {
    let mut machine = machine();
    machine
        .archive_unit(&path("U1.2"), ts(5), &UnitUsage::default())
        .unwrap();
    let before = machine.clone();
    let ids = machine
        .archive_unit(&path("U1.2"), ts(9), &UnitUsage::default())
        .unwrap();
    assert!(ids.is_empty());
    assert_eq!(machine, before);
}

#[test]
fn archive_unit_of_a_missing_unit_fails_with_unit_not_found() {
    let mut machine = machine();
    let before = machine.clone();
    let result = machine.archive_unit(&path("U1.9"), ts(9), &UnitUsage::default());
    assert!(matches!(
        result,
        Err(PlantError::UnitNotFound { path: p }) if p == path("U1.9")
    ));
    assert_eq!(machine, before);
}

#[test]
fn archive_unit_with_a_draft_on_a_descendant_fails_with_the_path_of_the_descendant() {
    let mut machine = machine();
    let before = machine.clone();
    let mut usage = UnitUsage::default();
    usage.add_draft(uid(20), reference("R-1"));
    let result = machine.archive_unit(&path("U1.2"), ts(9), &usage);
    assert!(matches!(
        result,
        Err(PlantError::UnitReferencedByDraft { path: p, reference: r })
            if p == path("U1.2.1") && r == reference("R-1")
    ));
    assert_eq!(machine, before);
}

#[test]
fn archive_unit_with_an_approved_version_fails_with_unit_referenced_by_approved() {
    let mut machine = machine();
    let before = machine.clone();
    let mut usage = UnitUsage::default();
    usage.add_approved(uid(30), reference("R-2"));
    let result = machine.archive_unit(&path("U1.2"), ts(9), &usage);
    assert!(matches!(
        result,
        Err(PlantError::UnitReferencedByApproved { path: p, reference: r })
            if p == path("U1.2") && r == reference("R-2")
    ));
    assert_eq!(machine, before);
}

#[test]
fn archive_unit_with_a_draft_and_an_approved_version_reports_the_draft() {
    let mut machine = machine();
    let mut usage = UnitUsage::default();
    usage.add_approved(uid(30), reference("R-2"));
    usage.add_draft(uid(20), reference("R-1"));
    let result = machine.archive_unit(&path("U1.2"), ts(9), &usage);
    assert!(matches!(
        result,
        Err(PlantError::UnitReferencedByDraft { .. })
    ));
}

#[test]
fn archive_unit_reports_the_lowest_path_first() {
    let mut machine = machine();
    let mut usage = UnitUsage::default();
    usage.add_draft(uid(20), reference("R-1"));
    usage.add_draft(uid(30), reference("R-3"));
    let result = machine.archive_unit(&path("U1.2"), ts(9), &usage);
    assert!(matches!(
        result,
        Err(PlantError::UnitReferencedByDraft { path: p, .. }) if p == path("U1.2")
    ));
}

#[test]
fn archive_unit_ignores_the_use_of_units_outside_the_set() {
    let mut machine = machine();
    let mut usage = UnitUsage::default();
    usage.add_draft(uid(40), reference("R-1"));
    usage.add_approved(uid(10), reference("R-2"));
    assert!(machine.archive_unit(&path("U1.2"), ts(9), &usage).is_ok());
}

#[test]
fn archive_unit_ignores_the_use_of_an_already_archived_unit() {
    let mut machine = machine();
    machine
        .archive_unit(&path("U1.2.1"), ts(5), &UnitUsage::default())
        .unwrap();
    let mut usage = UnitUsage::default();
    usage.add_approved(uid(20), reference("R-2"));
    let ids = machine.archive_unit(&path("U1.2"), ts(9), &usage).unwrap();
    assert_eq!(ids, vec![uid(30)]);
}

#[test]
fn archive_unit_of_the_last_main_unit_fails_with_last_main_unit() {
    let mut machine = new_machine();
    let before = machine.clone();
    let result = machine.archive_unit(&path("U1"), ts(9), &UnitUsage::default());
    assert!(matches!(
        result,
        Err(PlantError::LastMainUnit { code, path: p })
            if code.number() == 1 && p == path("U1")
    ));
    assert_eq!(machine, before);
}

#[test]
fn archive_unit_of_a_main_unit_next_to_a_live_main_unit_archives_its_subtree() {
    let mut machine = machine();
    let ids = machine
        .archive_unit(&path("U1"), ts(9), &UnitUsage::default())
        .unwrap();
    assert_eq!(ids, vec![uid(10), uid(20), uid(30), uid(40)]);
    assert_eq!(
        archived_paths(&machine),
        vec!["U1", "U1.2", "U1.2.1", "U1.3"]
    );
    assert!(machine.unit(&path("U2")).unwrap().is_live());
}

#[test]
fn archive_unit_checks_the_use_before_the_last_main_unit() {
    let mut machine = new_machine();
    let mut usage = UnitUsage::default();
    usage.add_approved(uid(1), reference("R-2"));
    let result = machine.archive_unit(&path("U1"), ts(9), &usage);
    assert!(matches!(
        result,
        Err(PlantError::UnitReferencedByApproved { .. })
    ));
}

#[test]
fn unit_usage_keeps_the_smallest_reference_of_a_unit() {
    let mut usage = UnitUsage::default();
    usage.add_draft(uid(1), reference("R-9"));
    usage.add_draft(uid(1), reference("R-2"));
    usage.add_draft(uid(1), reference("R-5"));
    assert_eq!(usage.draft_of(uid(1)), Some(&reference("R-2")));
    assert_eq!(usage.approved_of(uid(1)), None);
}
