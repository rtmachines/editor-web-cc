//! Tests of the data block number in `Machine::create` and `Machine::set_data_block`.

use uuid::Uuid;

use super::tests::{name, ts, uid};
use super::*;

fn number(text: &str) -> DataBlockNumber {
    DataBlockNumber::parse(text).unwrap()
}

fn create(
    platform: PlcPlatform,
    db_number: Option<DataBlockNumber>,
) -> Result<Machine, PlantError> {
    Machine::create(
        MachineId::from(Uuid::from_u128(900)),
        MachineCode::parse("M1").unwrap(),
        name("Press"),
        platform,
        db_number,
        (uid(1), name("Main")),
    )
}

fn archived(machine: &Machine) -> Machine {
    let mut copy = machine.clone();
    copy.archived_at = Some(ts(5));
    copy
}

#[test]
fn create_of_an_s7_machine_keeps_the_number() {
    let machine = create(PlcPlatform::S7_1500, Some(number("59999"))).unwrap();
    assert_eq!(machine.db_number(), Some(number("59999")));
}

#[test]
fn create_of_an_iec_machine_without_a_number_has_none() {
    let machine = create(PlcPlatform::CodesysIec, None).unwrap();
    assert_eq!(machine.db_number(), None);
}

#[test]
fn create_of_an_s7_machine_without_a_number_fails_with_data_block_missing() {
    let result = create(PlcPlatform::S7_300, None);
    assert!(matches!(
        result,
        Err(PlantError::DataBlockMissing {
            platform: PlcPlatform::S7_300,
            ..
        })
    ));
}

#[test]
fn create_of_an_iec_machine_with_a_number_fails_with_data_block_not_on_platform() {
    let result = create(PlcPlatform::TwinCat3, Some(number("10")));
    assert!(matches!(
        result,
        Err(PlantError::DataBlockNotOnPlatform {
            platform: PlcPlatform::TwinCat3,
            ..
        })
    ));
}

#[test]
fn create_of_an_s7_300_machine_above_the_platform_limit_fails_with_out_of_range() {
    let result = create(PlcPlatform::S7_300, Some(number("16001")));
    assert!(matches!(
        result,
        Err(PlantError::DataBlockOutOfRange {
            number: 16001,
            max: 16000,
            ..
        })
    ));
}

#[test]
fn set_data_block_replaces_the_number_of_a_live_s7_machine() {
    let mut machine = create(PlcPlatform::S7_1200, Some(number("1"))).unwrap();
    machine.set_data_block(Some(number("7"))).unwrap();
    assert_eq!(machine.db_number(), Some(number("7")));
}

#[test]
fn set_data_block_without_a_number_on_a_live_s7_machine_fails_and_keeps_the_number() {
    let mut machine = create(PlcPlatform::S7_1200, Some(number("1"))).unwrap();
    let result = machine.set_data_block(None);
    assert!(matches!(result, Err(PlantError::DataBlockMissing { .. })));
    assert_eq!(machine.db_number(), Some(number("1")));
}

#[test]
fn set_data_block_with_a_number_on_an_iec_machine_fails_with_data_block_not_on_platform() {
    let mut machine = create(PlcPlatform::CodesysIec, None).unwrap();
    let result = machine.set_data_block(Some(number("10")));
    assert!(matches!(
        result,
        Err(PlantError::DataBlockNotOnPlatform { .. })
    ));
    assert_eq!(machine.db_number(), None);
}

#[test]
fn set_data_block_above_the_platform_limit_fails_and_keeps_the_number() {
    let mut machine = create(PlcPlatform::S7_300, Some(number("1"))).unwrap();
    let result = machine.set_data_block(Some(number("16001")));
    assert!(matches!(
        result,
        Err(PlantError::DataBlockOutOfRange { max: 16000, .. })
    ));
    assert_eq!(machine.db_number(), Some(number("1")));
}

#[test]
fn set_data_block_of_an_archived_machine_fails_with_machine_archived() {
    let mut machine = archived(&create(PlcPlatform::S7_1500, Some(number("1"))).unwrap());
    let result = machine.set_data_block(Some(number("2")));
    assert!(matches!(result, Err(PlantError::MachineArchived { .. })));
    assert_eq!(machine.db_number(), Some(number("1")));
}
