use super::*;
use uuid::Uuid;

#[test]
fn machine_id_from_uuid_keeps_the_uuid() {
    let raw = Uuid::from_u128(1);
    assert_eq!(MachineId::from(raw).as_uuid(), &raw);
}

#[test]
fn machine_id_display_is_the_hyphenated_uuid() {
    let raw = Uuid::from_u128(2);
    assert_eq!(MachineId::from(raw).to_string(), raw.to_string());
}

#[test]
fn unit_id_from_uuid_keeps_the_uuid() {
    let raw = Uuid::from_u128(3);
    assert_eq!(UnitId::from(raw).as_uuid(), &raw);
}

#[test]
fn unit_id_display_is_the_hyphenated_uuid() {
    let raw = Uuid::from_u128(4);
    assert_eq!(UnitId::from(raw).to_string(), raw.to_string());
}

#[test]
fn plc_type_id_from_uuid_keeps_the_uuid() {
    let raw = Uuid::from_u128(5);
    assert_eq!(PlcTypeId::from(raw).as_uuid(), &raw);
}

#[test]
fn plc_type_id_display_is_the_hyphenated_uuid() {
    let raw = Uuid::from_u128(6);
    assert_eq!(PlcTypeId::from(raw).to_string(), raw.to_string());
}

#[test]
fn plc_connection_id_from_uuid_keeps_the_uuid() {
    let raw = Uuid::from_u128(7);
    assert_eq!(PlcConnectionId::from(raw).as_uuid(), &raw);
}

#[test]
fn plc_connection_id_display_is_the_hyphenated_uuid() {
    let raw = Uuid::from_u128(8);
    assert_eq!(PlcConnectionId::from(raw).to_string(), raw.to_string());
}
