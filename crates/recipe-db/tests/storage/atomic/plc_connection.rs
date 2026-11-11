//! Atomic tests of `SetPlcConnection` and `ExportPlcConnection` (task 4.10, design D6 and D9).
//!
//! A test trigger on `audit_log` fails the audit insert of the command. A failed save leaves no
//! row and no change. A failed export leaves no entry and returns no password.

use std::sync::Arc;

use recipe_app::export::{ExportPlcConnection, ExportPlcConnectionInput};
use recipe_app::memory::MemoryCredentialCipher;
use recipe_app::plant::{SetPlcConnection, SetPlcConnectionInput};
use recipe_domain::plant::{
    CommMode, ConnectionFields, PlcAddress, PlcHost, PlcLogin, PlcPassword, TcpPort,
};
use recipe_domain::recipe::ReferenceNumber;
use uuid::Uuid;

use crate::support::app::{App, Seeded};
use crate::support::fault::{FAULT, assert_audit_fault};
use crate::support::fixtures::{self, TWIN_CAT};

/// The plain password of the records of these tests.
const PASSWORD: &str = "s3cret-pw-9";
/// The `row_version` and the host of the record of a machine.
const RECORD_STATE: &str = "SELECT row_version::text || ' ' || host FROM plc_connection \
                            WHERE machine_id = $1";
/// Counts the records of a machine.
const RECORD_ROWS: &str = "SELECT count(*) FROM plc_connection WHERE machine_id = $1";
/// Counts the audit entries of an action for one machine.
const ENTRIES_OF_MACHINE: &str = "SELECT count(*) FROM audit_log WHERE action = $1 \
                                  AND after_state->>'machine_id' = $2";

/// A `TwinCat3` machine with the unit `U1`, a recipe, and the cipher of the tests.
struct Setup {
    seed: Seeded,
    reference: ReferenceNumber,
    cipher: Arc<MemoryCredentialCipher>,
}

async fn setup(app: &mut App) -> Setup {
    let seed = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let creator = *app.user.user().as_uuid();
    let recipe = fixtures::recipe(app.migrator(), seed.id, TWIN_CAT, creator).await;
    let reference: String = sqlx::query_scalar("SELECT reference_number FROM recipe WHERE id = $1")
        .bind(recipe)
        .fetch_one(app.migrator())
        .await
        .unwrap();
    Setup {
        seed,
        reference: ReferenceNumber::parse(&reference).unwrap(),
        cipher: Arc::new(MemoryCredentialCipher::default()),
    }
}

/// An `Ads` save with a password. Only `host` differs between two saves.
fn save_input(setup: &Setup, expected: i64, host: &str) -> SetPlcConnectionInput {
    SetPlcConnectionInput {
        code: setup.seed.code,
        expected_row_version: expected,
        fields: ConnectionFields {
            host: Some(PlcHost::parse(host).unwrap()),
            port: Some(TcpPort::parse("851").unwrap()),
            address: Some(PlcAddress::parse("5.12.34.56.1.1").unwrap()),
            mode: Some(CommMode::parse("Ads").unwrap()),
            login: Some(PlcLogin::parse("transfer").unwrap()),
            password: Some(PlcPassword::parse(PASSWORD).unwrap()),
            ..ConnectionFields::default()
        },
    }
}

fn set_use_case(app: &App, setup: &Setup) -> SetPlcConnection {
    SetPlcConnection::new(app.context(), setup.cipher.clone())
}

fn export_use_case(app: &App, setup: &Setup) -> ExportPlcConnection {
    ExportPlcConnection::new(app.context(), setup.cipher.clone())
}

fn export_input(setup: &Setup) -> ExportPlcConnectionInput {
    ExportPlcConnectionInput {
        code: setup.seed.code,
        recipe: setup.reference.clone(),
    }
}

/// Save the first record of the machine.
async fn first_save(app: &mut App, setup: &Setup) {
    set_use_case(app, setup)
        .execute(
            &app.user,
            &app.meta,
            save_input(setup, 0, "plc-first.plant.local"),
        )
        .await
        .unwrap();
}

async fn record_state(app: &mut App, machine: Uuid) -> String {
    sqlx::query_scalar(RECORD_STATE)
        .bind(machine)
        .fetch_one(app.admin())
        .await
        .unwrap()
}

async fn entries(app: &mut App, action: &str, machine: Uuid) -> i64 {
    sqlx::query_scalar(ENTRIES_OF_MACHINE)
        .bind(action)
        .bind(machine.to_string())
        .fetch_one(app.admin())
        .await
        .unwrap()
}

// SetPlcConnection

#[tokio::test]
async fn set_plc_connection_commits_the_first_record_and_one_entry() {
    let mut app = App::create().await;
    let setup = setup(&mut app).await;
    let before = app.head().await;

    let output = set_use_case(&app, &setup)
        .execute(
            &app.user,
            &app.meta,
            save_input(&setup, 0, "plc-first.plant.local"),
        )
        .await
        .unwrap();

    assert_eq!(output.form.row_version, 1);
    assert_eq!(
        record_state(&mut app, setup.seed.id).await,
        "1 plc-first.plant.local"
    );
    app.assert_chain_moved(&before, &["PlcConnectionSet"]).await;
}

#[tokio::test]
async fn a_failed_audit_insert_of_the_first_save_leaves_no_record() {
    let mut app = App::create().await;
    let setup = setup(&mut app).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = set_use_case(&app, &setup)
        .execute(
            &app.user,
            &app.meta,
            save_input(&setup, 0, "plc-first.plant.local"),
        )
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(app.count(RECORD_ROWS, setup.seed.id).await, 0);
    app.assert_chain_unchanged(&before).await;
}

#[tokio::test]
async fn a_failed_audit_insert_of_a_later_save_keeps_the_record_and_its_row_version() {
    let mut app = App::create().await;
    let setup = setup(&mut app).await;
    first_save(&mut app, &setup).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = set_use_case(&app, &setup)
        .execute(
            &app.user,
            &app.meta,
            save_input(&setup, 1, "plc-second.plant.local"),
        )
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(
        record_state(&mut app, setup.seed.id).await,
        "1 plc-first.plant.local"
    );
    app.assert_chain_unchanged(&before).await;
}

// ExportPlcConnection

#[tokio::test]
async fn export_plc_connection_commits_one_entry_without_the_password() {
    let mut app = App::create().await;
    let setup = setup(&mut app).await;
    first_save(&mut app, &setup).await;
    let before = app.head().await;

    let exported = export_use_case(&app, &setup)
        .execute(&app.user, &app.meta, export_input(&setup))
        .await
        .unwrap();

    assert!(exported.password.is_some());
    app.assert_chain_moved(&before, &["PlcConnectionExported"])
        .await;
    assert_eq!(
        entries(&mut app, "PlcConnectionExported", setup.seed.id).await,
        1
    );
    let state: String = sqlx::query_scalar(
        "SELECT after_state::text FROM audit_log WHERE action = 'PlcConnectionExported'",
    )
    .fetch_one(app.admin())
    .await
    .unwrap();
    assert!(!state.contains(PASSWORD), "{state}");
}

#[tokio::test]
async fn a_failed_audit_insert_of_the_export_leaves_no_entry_and_returns_no_password() {
    let mut app = App::create().await;
    let setup = setup(&mut app).await;
    first_save(&mut app, &setup).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let result = export_use_case(&app, &setup)
        .execute(&app.user, &app.meta, export_input(&setup))
        .await;

    let error = result.unwrap_err();
    assert_audit_fault(&error);
    assert_eq!(
        entries(&mut app, "PlcConnectionExported", setup.seed.id).await,
        0
    );
    app.assert_chain_unchanged(&before).await;
    assert_eq!(
        record_state(&mut app, setup.seed.id).await,
        "1 plc-first.plant.local"
    );
}
