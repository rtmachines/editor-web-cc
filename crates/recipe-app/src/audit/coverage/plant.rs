//! Audit coverage of the plant use cases: machines, units, and user-defined types.

use recipe_domain::audit::{AggregateType, AuditAction};
use recipe_domain::identity::Role;

use super::assert_trail;
use crate::memory::fixtures::scenarios;
use crate::memory::fixtures::{Fixture, actor, meta};
use crate::plant::{
    AddUnit, ArchiveMachine, ArchiveUnit, CreateMachine, DefineUdt, RenameMachine, RenameUnit,
};

#[tokio::test]
async fn create_machine_writes_the_machine_then_its_main_unit() {
    let fixture = Fixture::new();
    let user = actor(Role::Administrator);
    let request = meta();
    CreateMachine::new(fixture.context())
        .execute(&user, &request, scenarios::create_machine_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "CreateMachine",
        &[
            (
                AuditAction::MachineCreated,
                "machine",
                AggregateType::Machine,
            ),
            (AuditAction::UnitAdded, "unit", AggregateType::Machine),
        ],
    );
}

#[tokio::test]
async fn rename_machine_writes_one_machine_entry() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    RenameMachine::new(fixture.context())
        .execute(&user, &request, scenarios::rename_machine_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "RenameMachine",
        &[(
            AuditAction::MachineRenamed,
            "machine",
            AggregateType::Machine,
        )],
    );
}

#[tokio::test]
async fn archive_machine_writes_one_machine_entry() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    ArchiveMachine::new(fixture.context())
        .execute(&user, &request, scenarios::archive_machine_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "ArchiveMachine",
        &[(
            AuditAction::MachineArchived,
            "machine",
            AggregateType::Machine,
        )],
    );
}

#[tokio::test]
async fn add_unit_writes_one_unit_entry() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    AddUnit::new(fixture.context())
        .execute(&user, &request, scenarios::add_unit_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "AddUnit",
        &[(AuditAction::UnitAdded, "unit", AggregateType::Machine)],
    );
}

#[tokio::test]
async fn rename_unit_writes_one_unit_entry() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    RenameUnit::new(fixture.context())
        .execute(&user, &request, scenarios::rename_unit_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "RenameUnit",
        &[(AuditAction::UnitRenamed, "unit", AggregateType::Machine)],
    );
}

#[tokio::test]
async fn archive_unit_writes_one_entry_per_archived_unit() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    ArchiveUnit::new(fixture.context())
        .execute(&user, &request, scenarios::archive_unit_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "ArchiveUnit",
        &[
            (AuditAction::UnitArchived, "unit", AggregateType::Machine),
            (AuditAction::UnitArchived, "unit", AggregateType::Machine),
        ],
    );
}

#[tokio::test]
async fn define_udt_writes_the_derived_type_before_the_struct() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    DefineUdt::new(fixture.context())
        .execute(&user, &request, scenarios::define_udt_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "DefineUdt",
        &[
            (
                AuditAction::DerivedTypeCreated,
                "plc_type",
                AggregateType::Machine,
            ),
            (AuditAction::UdtDefined, "plc_type", AggregateType::Machine),
        ],
    );
}
