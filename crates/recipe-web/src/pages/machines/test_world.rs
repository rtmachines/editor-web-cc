//! The extra data of the tests of the machine pages (design D12, D16). This module is test code.
//!
//! The module solves one problem: the tests of the machine list and the unit tree need machines that
//! `seed_plant` does not hold. The world, the sessions, and the senders live once in
//! `test_support::plant` (design D20 U13). This file keeps only what the machine pages alone need.

use recipe_domain::plant::{
    Machine, MachineCode, MachineId, PlcPlatform, UnitId, UnitPath, UnitUsage,
};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::test_support::Fixture;

/// The first field of the identifier of a machine that `seed_machines` makes.
const MACHINE_TAG: u32 = 0x7357_0010;
/// The first field of the identifier of a unit that `seed_machines` makes.
const UNIT_TAG: u32 = 0x7357_0011;
/// The number of the first machine that `seed_machines` makes.
const FIRST_NUMBER: u32 = 101;
/// The instant of the archive of the unit that `seed_machine_with_archived_unit` makes.
const ARCHIVED_AT_SECONDS: i64 = 600_000;

/// Make a machine identifier or a unit identifier from the tag and the number.
fn id_of(tag: u32, number: u32) -> Uuid {
    let bytes = number.to_be_bytes();
    let tail = [0, 0, 0, 0, bytes[0], bytes[1], bytes[2], bytes[3]];
    Uuid::from_fields(tag, 0, 0, &tail)
}

/// Build the live machine `M<number>` with the main unit `U1`.
fn machine(number: u32, name: &str) -> Machine {
    Machine::create(
        MachineId::from(id_of(MACHINE_TAG, number)),
        MachineCode::parse(&format!("M{number}")).unwrap(),
        Text::parse(name).unwrap(),
        PlcPlatform::TwinCat3,
        (
            UnitId::from(id_of(UNIT_TAG, number)),
            Text::parse("Infeed").unwrap(),
        ),
    )
}

/// Seed `count` live machines with the codes `M101`, `M102`, and so on, all with one name.
pub(crate) fn seed_machines(fixture: &Fixture, count: u32, name: &str) {
    for offset in 0..count {
        let number = FIRST_NUMBER.checked_add(offset).unwrap();
        fixture.store.seed_machine(&machine(number, name));
    }
}

/// Seed the live machine `M20`: the live units `U1`, `U1.1`, `U2` and the archived unit `U3`.
pub(super) fn seed_machine_with_archived_unit(fixture: &Fixture) {
    let mut machine = machine(20, "Spare line");
    let unit = |number: u32| UnitId::from(id_of(UNIT_TAG, number));
    let name = |text: &str| Text::parse(text).unwrap();
    let main = UnitPath::parse("U1").unwrap();
    machine
        .add_unit(unit(2_001), None, "2", name("Outfeed"))
        .unwrap();
    machine
        .add_unit(unit(2_002), None, "3", name("Spare"))
        .unwrap();
    machine
        .add_unit(unit(2_003), Some(&main), "1", name("Feeder"))
        .unwrap();
    let at = Timestamp::from(OffsetDateTime::from_unix_timestamp(ARCHIVED_AT_SECONDS).unwrap());
    let spare = UnitPath::parse("U3").unwrap();
    machine
        .archive_unit(&spare, at, &UnitUsage::default())
        .unwrap();
    fixture.store.seed_machine(&machine);
}

/// Return the row of the table that holds `needle`.
pub(super) fn row_with<'a>(body: &'a str, needle: &str) -> &'a str {
    let found = body.split("<tr>").skip(1).find(|row| row.contains(needle));
    assert!(found.is_some(), "no table row holds {needle}: {body}");
    found.unwrap()
}
