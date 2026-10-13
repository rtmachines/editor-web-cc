//! Tests of `Machine::create` and `Machine::add_unit`.

use uuid::Uuid;

use super::tests::{name, path, ts, uid};
use super::*;

fn code() -> MachineCode {
    MachineCode::parse("M1").unwrap()
}

pub(super) fn new_machine() -> Machine {
    Machine::create(
        MachineId::from(Uuid::from_u128(900)),
        code(),
        name("Press"),
        PlcPlatform::TwinCat3,
        (uid(1), name("Main")),
    )
}

fn archived_machine() -> Machine {
    let mut stored = StoredMachine {
        id: MachineId::from(Uuid::from_u128(900)),
        code: code(),
        name: name("Press"),
        platform: PlcPlatform::TwinCat3,
        archived_at: Some(ts(5)),
        units: vec![],
    };
    stored.units.push(StoredUnit {
        id: uid(1),
        path: path("U1"),
        name: name("Main"),
        archived_at: None,
    });
    Machine::try_from(stored).unwrap()
}

/// Return a machine with U1, U1.2 (live) and U1.3 (archived).
fn tree_machine() -> Machine {
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

fn paths(machine: &Machine) -> Vec<String> {
    machine.units().map(|u| u.path().to_string()).collect()
}

#[test]
fn create_gives_a_live_machine_with_the_live_main_unit_u1() {
    let machine = new_machine();
    assert!(machine.is_live());
    assert_eq!(machine.platform(), PlcPlatform::TwinCat3);
    assert_eq!(machine.name().as_str(), "Press");
    assert_eq!(paths(&machine), vec!["U1"]);
    let main = machine.unit_for_test("U1");
    assert_eq!(main.id(), uid(1));
    assert_eq!(main.name().as_str(), "Main");
    assert!(main.is_live());
}

impl Machine {
    fn unit_for_test(&self, text: &str) -> &Unit {
        self.units.get(&path(text)).unwrap()
    }
}

#[test]
fn add_unit_adds_a_main_unit_without_a_parent() {
    let mut machine = new_machine();
    let added = machine.add_unit(uid(2), None, "2", name("Second")).unwrap();
    assert_eq!(added.path(), &path("U2"));
    assert_eq!(added.id(), uid(2));
    assert_eq!(added.name().as_str(), "Second");
    assert!(added.is_live());
    assert_eq!(paths(&machine), vec!["U1", "U2"]);
}

#[test]
fn add_unit_adds_a_child_under_a_live_parent() {
    let mut machine = new_machine();
    let added = machine
        .add_unit(uid(2), Some(&path("U1")), "2", name("Child"))
        .unwrap();
    assert_eq!(added.path(), &path("U1.2"));
    assert_eq!(paths(&machine), vec!["U1", "U1.2"]);
}

#[test]
fn add_unit_accepts_the_segments_1_10_and_64() {
    let mut machine = new_machine();
    for (id, segment) in [(2, "1"), (3, "10"), (4, "64")] {
        assert!(
            machine
                .add_unit(uid(id), Some(&path("U1")), segment, name("x"))
                .is_ok(),
            "{segment}"
        );
    }
    assert_eq!(paths(&machine), vec!["U1", "U1.1", "U1.10", "U1.64"]);
}

#[test]
fn add_unit_to_an_archived_machine_fails_with_machine_archived() {
    let mut machine = archived_machine();
    let result = machine.add_unit(uid(2), None, "2", name("x"));
    assert!(matches!(result, Err(PlantError::MachineArchived { code: c }) if c == code()));
    assert_eq!(paths(&machine), vec!["U1"]);
}

#[test]
fn add_unit_checks_the_machine_before_the_segment() {
    let mut machine = archived_machine();
    let result = machine.add_unit(uid(2), None, "65", name("x"));
    assert!(matches!(result, Err(PlantError::MachineArchived { .. })));
}

#[test]
fn add_unit_rejects_a_bad_segment_with_the_joined_text() {
    let cases = [
        (None, "65", "U65"),
        (None, "0", "U0"),
        (None, "01", "U01"),
        (None, "1.2", "U1.2"),
        (None, "a", "Ua"),
        (None, "", "U"),
        (Some("U1"), "65", "U1.65"),
        (Some("U1"), "1.2", "U1.1.2"),
        (Some("U1"), "", "U1."),
    ];
    for (parent, segment, expected) in cases {
        let mut machine = new_machine();
        let before = machine.clone();
        let parent = parent.map(path);
        let result = machine.add_unit(uid(9), parent.as_ref(), segment, name("x"));
        assert!(
            matches!(&result, Err(PlantError::InvalidUnitPath { text }) if text == expected),
            "{segment:?}"
        );
        assert_eq!(machine, before, "{segment:?}");
    }
}

#[test]
fn add_unit_below_five_levels_fails_with_unit_depth_exceeded() {
    let mut machine = new_machine();
    let mut parent = path("U1");
    for (id, segment) in [(2, "2"), (3, "3"), (4, "4"), (5, "5")] {
        let added = machine
            .add_unit(uid(id), Some(&parent), segment, name("x"))
            .unwrap();
        parent = added.path().clone();
    }
    assert_eq!(parent, path("U1.2.3.4.5"));
    let before = machine.clone();
    let result = machine.add_unit(uid(6), Some(&parent), "1", name("x"));
    assert!(matches!(
        &result,
        Err(PlantError::UnitDepthExceeded { path }) if path == "U1.2.3.4.5.1"
    ));
    assert_eq!(machine, before);
}

#[test]
fn add_unit_with_a_missing_parent_fails_with_parent_not_found_for_the_new_path() {
    let mut machine = tree_machine();
    let before = machine.clone();
    let result = machine.add_unit(uid(9), Some(&path("U1.4")), "1", name("x"));
    assert!(matches!(
        result,
        Err(PlantError::ParentNotFound { path: p }) if p == path("U1.4.1")
    ));
    assert_eq!(machine, before);
}

#[test]
fn add_unit_under_an_archived_parent_fails_with_unit_archived_for_the_parent() {
    let mut machine = tree_machine();
    let before = machine.clone();
    let result = machine.add_unit(uid(9), Some(&path("U1.3")), "1", name("x"));
    assert!(matches!(
        result,
        Err(PlantError::UnitArchived { path: p }) if p == path("U1.3")
    ));
    assert_eq!(machine, before);
}

#[test]
fn add_unit_with_a_duplicate_path_fails_with_duplicate_unit_path() {
    let mut machine = tree_machine();
    let before = machine.clone();
    let result = machine.add_unit(uid(9), Some(&path("U1")), "2", name("x"));
    assert!(matches!(
        result,
        Err(PlantError::DuplicateUnitPath { path: p }) if p == path("U1.2")
    ));
    assert_eq!(machine, before);
}

#[test]
fn add_unit_over_an_archived_unit_path_fails_with_duplicate_unit_path() {
    let mut machine = tree_machine();
    let result = machine.add_unit(uid(9), Some(&path("U1")), "3", name("x"));
    assert!(matches!(result, Err(PlantError::DuplicateUnitPath { .. })));
}

#[test]
fn add_unit_with_a_known_unit_id_fails_with_duplicate_unit_path() {
    let mut machine = new_machine();
    let before = machine.clone();
    let result = machine.add_unit(uid(1), None, "2", name("x"));
    assert!(matches!(
        result,
        Err(PlantError::DuplicateUnitPath { path: p }) if p == path("U2")
    ));
    assert_eq!(machine, before);
}
