//! Atomic tests of the seven Plant Model commands and of `EnsureDerivedType` (task 10.1).
//!
//! `EnsureDerivedType` is not a use case of its own. `DefineUdt` calls it for each member type, so
//! a `STRING(16)` member and an `ARRAY` member create new derived rows in the same transaction.

use recipe_app::plant::{
    AddUnit, AddUnitInput, ArchiveMachine, ArchiveMachineInput, ArchiveUnit, ArchiveUnitInput,
    CreateMachine, CreateMachineInput, DefineUdt, DefineUdtInput, DefineUdtMode, RenameMachine,
    RenameMachineInput, RenameUnit, RenameUnitInput,
};
use recipe_domain::plant::{MachineCode, MemberName, PlcPlatform, TypeExpr, TypeName, UnitPath};
use recipe_domain::text::Text;

use crate::support::app::{App, Seeded};
use crate::support::fault::{FAULT, assert_audit_fault, assert_statement_fault};
use crate::support::fixtures::TWIN_CAT;

fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

/// Seed a `TwinCat3` machine with the main unit `U1`.
async fn machine(app: &mut App) -> Seeded {
    app.seed_machine(TWIN_CAT, &["U1"]).await
}

// CreateMachine

fn create_input(code: &str) -> CreateMachineInput {
    CreateMachineInput {
        code: MachineCode::parse(code).unwrap(),
        name: text("Press"),
        platform: PlcPlatform::TwinCat3,
        main_unit_name: text("Feeder"),
    }
}

#[tokio::test]
async fn create_machine_commits_the_machine_the_unit_and_two_entries() {
    let mut app = App::create().await;
    let before = app.head().await;

    let output = CreateMachine::new(app.context())
        .execute(&app.user, &app.meta, create_input("M801"))
        .await
        .unwrap();

    let machine = *output.machine.as_uuid();
    assert_eq!(
        app.count("SELECT count(*) FROM machine WHERE id = $1", machine)
            .await,
        1
    );
    assert_eq!(
        app.count("SELECT count(*) FROM unit WHERE machine_id = $1", machine)
            .await,
        1
    );
    app.assert_chain_moved(&before, &["MachineCreated", "UnitAdded"])
        .await;
}

#[tokio::test]
async fn create_machine_fails_midway_and_leaves_no_row_and_the_same_chain_head() {
    let mut app = App::create().await;
    let before = app.head().await;
    app.install_probe("unit", "BEFORE", "INSERT", FAULT).await;

    let error = CreateMachine::new(app.context())
        .execute(&app.user, &app.meta, create_input("M802"))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count_machines_with_code("M802").await, 0);
    app.assert_chain_unchanged(&before).await;
}

// RenameMachine

fn rename_machine_input(seed: &Seeded) -> RenameMachineInput {
    RenameMachineInput {
        code: seed.code,
        name: text("Renamed"),
    }
}

#[tokio::test]
async fn rename_machine_commits_the_name_and_one_entry() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;
    let before = app.head().await;

    RenameMachine::new(app.context())
        .execute(&app.user, &app.meta, rename_machine_input(&seed))
        .await
        .unwrap();

    assert_eq!(app.machine_name(seed.id).await, "Renamed");
    app.assert_chain_moved(&before, &["MachineRenamed"]).await;
}

#[tokio::test]
async fn rename_machine_fails_in_the_audit_insert_and_keeps_the_old_name() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = RenameMachine::new(app.context())
        .execute(&app.user, &app.meta, rename_machine_input(&seed))
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(app.machine_name(seed.id).await, "machine");
    app.assert_chain_unchanged(&before).await;
}

// ArchiveMachine

#[tokio::test]
async fn archive_machine_commits_the_archive_mark_and_one_entry() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;
    let before = app.head().await;

    ArchiveMachine::new(app.context())
        .execute(
            &app.user,
            &app.meta,
            ArchiveMachineInput { code: seed.code },
        )
        .await
        .unwrap();

    assert_eq!(
        app.count(
            "SELECT count(*) FROM machine WHERE id = $1 AND archived_at IS NOT NULL",
            seed.id
        )
        .await,
        1
    );
    app.assert_chain_moved(&before, &["MachineArchived"]).await;
}

#[tokio::test]
async fn archive_machine_fails_in_the_audit_insert_and_keeps_the_machine_live() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = ArchiveMachine::new(app.context())
        .execute(
            &app.user,
            &app.meta,
            ArchiveMachineInput { code: seed.code },
        )
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(
        app.count(
            "SELECT count(*) FROM machine WHERE id = $1 AND archived_at IS NULL",
            seed.id
        )
        .await,
        1
    );
    app.assert_chain_unchanged(&before).await;
}

// AddUnit

fn add_unit_input(seed: &Seeded) -> AddUnitInput {
    AddUnitInput {
        code: seed.code,
        parent_path: None,
        segment: "2".to_owned(),
        name: text("Second"),
    }
}

#[tokio::test]
async fn add_unit_commits_the_unit_and_one_entry() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;
    let before = app.head().await;

    AddUnit::new(app.context())
        .execute(&app.user, &app.meta, add_unit_input(&seed))
        .await
        .unwrap();

    assert_eq!(
        app.count(
            "SELECT count(*) FROM unit WHERE machine_id = $1 AND path = 'U2'",
            seed.id
        )
        .await,
        1
    );
    app.assert_chain_moved(&before, &["UnitAdded"]).await;
}

#[tokio::test]
async fn add_unit_fails_in_the_audit_insert_and_leaves_no_unit() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = AddUnit::new(app.context())
        .execute(&app.user, &app.meta, add_unit_input(&seed))
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(
        app.count("SELECT count(*) FROM unit WHERE machine_id = $1", seed.id)
            .await,
        1
    );
    app.assert_chain_unchanged(&before).await;
}

// RenameUnit

fn rename_unit_input(seed: &Seeded) -> RenameUnitInput {
    RenameUnitInput {
        code: seed.code,
        path: UnitPath::parse("U1").unwrap(),
        name: text("Renamed unit"),
    }
}

#[tokio::test]
async fn rename_unit_commits_the_name_and_one_entry() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;
    let before = app.head().await;

    RenameUnit::new(app.context())
        .execute(&app.user, &app.meta, rename_unit_input(&seed))
        .await
        .unwrap();

    assert_eq!(app.unit_name(seed.id, "U1").await, "Renamed unit");
    app.assert_chain_moved(&before, &["UnitRenamed"]).await;
}

#[tokio::test]
async fn rename_unit_fails_in_the_audit_insert_and_keeps_the_old_name() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = RenameUnit::new(app.context())
        .execute(&app.user, &app.meta, rename_unit_input(&seed))
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(app.unit_name(seed.id, "U1").await, "unit");
    app.assert_chain_unchanged(&before).await;
}

// ArchiveUnit

async fn machine_with_a_unit_tree(app: &mut App) -> Seeded {
    app.seed_machine(TWIN_CAT, &["U1", "U2", "U2.1"]).await
}

fn archive_unit_input(seed: &Seeded) -> ArchiveUnitInput {
    ArchiveUnitInput {
        code: seed.code,
        path: UnitPath::parse("U2").unwrap(),
    }
}

const LIVE_UNITS: &str = "SELECT count(*) FROM unit WHERE machine_id = $1 AND archived_at IS NULL";

#[tokio::test]
async fn archive_unit_commits_both_units_of_the_tree_and_two_entries() {
    let mut app = App::create().await;
    let seed = machine_with_a_unit_tree(&mut app).await;
    let before = app.head().await;

    let output = ArchiveUnit::new(app.context())
        .execute(&app.user, &app.meta, archive_unit_input(&seed))
        .await
        .unwrap();

    assert_eq!(output.units.len(), 2);
    assert_eq!(app.count(LIVE_UNITS, seed.id).await, 1);
    app.assert_chain_moved(&before, &["UnitArchived", "UnitArchived"])
        .await;
}

#[tokio::test]
async fn archive_unit_fails_on_the_second_unit_and_archives_no_unit() {
    let mut app = App::create().await;
    let seed = machine_with_a_unit_tree(&mut app).await;
    let before = app.head().await;
    app.install_probe(
        "unit",
        "BEFORE",
        "UPDATE",
        "IF NEW.path = 'U2.1' THEN RAISE EXCEPTION 'injected fault'; END IF;",
    )
    .await;

    let error = ArchiveUnit::new(app.context())
        .execute(&app.user, &app.meta, archive_unit_input(&seed))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count(LIVE_UNITS, seed.id).await, 3);
    app.assert_chain_unchanged(&before).await;
}

// DefineUdt and EnsureDerivedType

fn define_input(seed: &Seeded) -> DefineUdtInput {
    DefineUdtInput {
        code: seed.code,
        name: TypeName::parse("ST_Zone").unwrap(),
        members: vec![
            (
                MemberName::parse("label").unwrap(),
                TypeExpr::parse("STRING(16)").unwrap(),
            ),
            (
                MemberName::parse("temps").unwrap(),
                TypeExpr::parse("ARRAY[1..2] OF REAL").unwrap(),
            ),
        ],
        mode: DefineUdtMode::New,
    }
}

const TYPE_ROWS: &str = "SELECT count(*) FROM plc_type WHERE machine_id = $1";

#[tokio::test]
async fn define_udt_commits_the_struct_its_members_and_the_new_derived_rows() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;
    let before = app.head().await;

    DefineUdt::new(app.context())
        .execute(&app.user, &app.meta, define_input(&seed))
        .await
        .unwrap();

    assert_eq!(app.count(TYPE_ROWS, seed.id).await, 3);
    assert_eq!(
        app.count(
            "SELECT count(*) FROM plc_type_member m JOIN plc_type t ON t.id = m.struct_type_id \
             WHERE t.machine_id = $1",
            seed.id
        )
        .await,
        2
    );
    app.assert_chain_moved(
        &before,
        &["DerivedTypeCreated", "DerivedTypeCreated", "UdtDefined"],
    )
    .await;
}

#[tokio::test]
async fn define_udt_fails_on_the_member_insert_and_leaves_no_type_row() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;
    let before = app.head().await;
    app.install_probe("plc_type_member", "BEFORE", "INSERT", FAULT)
        .await;

    let error = DefineUdt::new(app.context())
        .execute(&app.user, &app.meta, define_input(&seed))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count(TYPE_ROWS, seed.id).await, 0);
    app.assert_chain_unchanged(&before).await;
}

#[tokio::test]
async fn ensure_derived_type_rows_vanish_when_the_struct_insert_fails() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;
    let before = app.head().await;
    app.install_probe(
        "plc_type",
        "BEFORE",
        "INSERT",
        "IF NEW.kind = 'struct' THEN RAISE EXCEPTION 'injected fault'; END IF;",
    )
    .await;

    let error = DefineUdt::new(app.context())
        .execute(&app.user, &app.meta, define_input(&seed))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count(TYPE_ROWS, seed.id).await, 0);
    app.assert_chain_unchanged(&before).await;
}

#[tokio::test]
async fn ensure_derived_type_creates_a_string_row_and_an_array_row_on_success() {
    let mut app = App::create().await;
    let seed = machine(&mut app).await;

    DefineUdt::new(app.context())
        .execute(&app.user, &app.meta, define_input(&seed))
        .await
        .unwrap();

    assert_eq!(
        app.count(
            "SELECT count(*) FROM plc_type WHERE machine_id = $1 AND kind = 'primitive' \
             AND name = 'STRING' AND string_length = 16",
            seed.id
        )
        .await,
        1
    );
    assert_eq!(
        app.count(
            "SELECT count(*) FROM plc_type WHERE machine_id = $1 AND kind = 'array'",
            seed.id
        )
        .await,
        1
    );
}
