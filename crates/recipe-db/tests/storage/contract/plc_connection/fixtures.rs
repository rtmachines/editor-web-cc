//! Fixtures of the `PlcConnections` cases. They go through the ports and `UnitOfWork::commit`.

use recipe_app::ports::{RowImage, SealedSecret};
use recipe_domain::plant::{
    CommMode, DataBlockNumber, Machine, MachineId, PlcAddress, PlcConnection, PlcHost, PlcLogin,
    PlcPlatform, Rack, Slot, TcpPort,
};

use crate::contract::harness::{Harness, begin, commit};
use crate::contract::seed::{code, machine_id, name, unit_id};

/// Insert a live machine of a platform in its own committed transaction.
pub async fn save_machine(
    harness: &dyn Harness,
    n: u128,
    code_text: &str,
    platform: PlcPlatform,
) -> MachineId {
    let built = Machine::create(
        machine_id(n),
        code(code_text),
        name("Press"),
        platform,
        (unit_id(n.checked_mul(10).unwrap()), name("Main unit")),
    );
    let mut tx = begin(harness).await;
    tx.plant().insert_machine(&built).await.unwrap();
    commit(harness, tx, built.id()).await;
    built.id()
}

/// An S7 connection from the text of its fields.
pub fn s7_connection(host: &str, rack: &str, slot: &str, db_number: &str) -> PlcConnection {
    PlcConnection::S7 {
        host: PlcHost::parse(host).unwrap(),
        rack: Rack::parse(rack).unwrap(),
        slot: Slot::parse(slot).unwrap(),
        db_number: DataBlockNumber::parse(db_number).unwrap(),
    }
}

/// An IEC connection from the text of its fields.
pub fn iec_connection(
    host: &str,
    port: &str,
    address: &str,
    mode: &str,
    login: Option<&str>,
) -> PlcConnection {
    PlcConnection::Iec {
        host: PlcHost::parse(host).unwrap(),
        port: TcpPort::parse(port).unwrap(),
        address: PlcAddress::parse(address).unwrap(),
        mode: CommMode::parse(mode).unwrap(),
        login: login.map(|text| PlcLogin::parse(text).unwrap()),
    }
}

/// A sealed password of fixed bytes. The adapters keep the bytes and never read their layout.
pub fn sealed(bytes: &[u8]) -> SealedSecret {
    SealedSecret::from_bytes(bytes.to_vec())
}

/// The `Debug` text of a connection. `PlcConnection` has no `PartialEq`.
pub fn connection_text(connection: &PlcConnection) -> String {
    format!("{connection:?}")
}

/// Insert a record in its own committed transaction and return its image.
pub async fn insert_connection(
    harness: &dyn Harness,
    machine: MachineId,
    connection: &PlcConnection,
    password: Option<&SealedSecret>,
) -> RowImage {
    let mut tx = begin(harness).await;
    let image = tx
        .plc_connections()
        .insert_connection(machine, connection, password)
        .await
        .unwrap();
    commit(harness, tx, machine).await;
    image
}
