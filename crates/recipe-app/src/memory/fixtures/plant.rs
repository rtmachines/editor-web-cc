//! The machine fixture with three live units.

use recipe_domain::plant::{Machine, MachineCode, PlcPlatform, UnitId, UnitPath};

use super::{mid, text, uid};

/// Return the identifier of unit `index` of machine `n`. Unit 1 is the main unit `U1`.
fn unit_id(n: u128, index: u128) -> UnitId {
    uid(n.checked_mul(100).unwrap().checked_add(index).unwrap())
}

/// A machine of `platform` with the live units `U1`, `U1.2`, and `U1.2.1`.
///
/// The main unit has the identifier `n * 100 + 1`, and the other units follow it.
/// The machine does not use the store. Use `Fixture::seed_machine_on` to store it.
pub(crate) fn machine_on(platform: PlcPlatform, n: u128, code: &str) -> Machine {
    let mut machine = Machine::create(
        mid(n),
        MachineCode::parse(code).unwrap(),
        text("Press"),
        platform,
        (unit_id(n, 1), text("Main")),
    );
    let root = UnitPath::parse("U1").unwrap();
    let second = machine
        .add_unit(unit_id(n, 2), Some(&root), "2", text("Two"))
        .unwrap()
        .path()
        .clone();
    machine
        .add_unit(unit_id(n, 3), Some(&second), "1", text("Sub"))
        .unwrap();
    machine
}
