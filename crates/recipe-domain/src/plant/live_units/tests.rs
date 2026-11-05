use time::OffsetDateTime;
use uuid::Uuid;

use crate::error_codes::ErrorCode;
use crate::plant::{
    Machine, MachineCode, MachineId, PlantError, PlcPlatform, StoredMachine, StoredUnit, UnitId,
    UnitPath,
};
use crate::text::Text;
use crate::timestamp::Timestamp;

fn uid(n: u128) -> UnitId {
    UnitId::from(Uuid::from_u128(n))
}

fn path(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

fn machine_id() -> MachineId {
    MachineId::from(Uuid::from_u128(900))
}

fn stored_unit(id: u128, text: &str, is_archived: bool) -> StoredUnit {
    StoredUnit {
        id: uid(id),
        path: path(text),
        name: Text::parse(text).unwrap(),
        archived_at: is_archived
            .then(|| Timestamp::from(OffsetDateTime::from_unix_timestamp(5).unwrap())),
    }
}

fn machine(is_archived: bool) -> Machine {
    Machine::try_from(StoredMachine {
        id: machine_id(),
        code: MachineCode::parse("M7").unwrap(),
        name: Text::parse("Press").unwrap(),
        platform: PlcPlatform::TwinCat3,
        archived_at: is_archived
            .then(|| Timestamp::from(OffsetDateTime::from_unix_timestamp(9).unwrap())),
        units: vec![
            stored_unit(1, "U1", false),
            stored_unit(2, "U1.2", false),
            stored_unit(3, "U1.3", true),
        ],
    })
    .unwrap()
}

#[test]
fn require_of_an_unknown_path_gives_unit_not_found() {
    let units = machine(false).live_units();
    let error = units.require(&path("U9")).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_020);
    assert!(matches!(error, PlantError::UnitNotFound { path: p } if p == path("U9")));
}

#[test]
fn require_of_an_archived_unit_gives_unit_archived() {
    let units = machine(false).live_units();
    let error = units.require(&path("U1.3")).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_009);
    assert!(matches!(error, PlantError::UnitArchived { path: p } if p == path("U1.3")));
}

#[test]
fn require_of_a_live_unit_knows_its_machine() {
    let units = machine(false).live_units();
    let live = units.require(&path("U1.2")).unwrap();
    assert_eq!(live.machine(), machine_id());
    assert_eq!(live.id(), uid(2));
    assert_eq!(live.path(), &path("U1.2"));
    assert_eq!(live.clone(), live);
}

#[test]
fn live_units_report_the_machine_and_its_state() {
    let units = machine(false).live_units();
    assert_eq!(units.machine(), machine_id());
    assert_eq!(units.machine_code().number(), 7);
    assert!(!units.is_machine_archived());
    assert!(machine(true).live_units().is_machine_archived());
}

#[test]
fn path_of_and_is_live_cover_live_and_archived_units() {
    let units = machine(false).live_units();
    assert_eq!(units.path_of(uid(2)), Some(&path("U1.2")));
    assert_eq!(units.path_of(uid(3)), Some(&path("U1.3")));
    assert_eq!(units.path_of(uid(99)), None);
    assert!(units.is_live(uid(2)));
    assert!(!units.is_live(uid(3)));
    assert!(!units.is_live(uid(99)));
}
