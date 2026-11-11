//! Races of `SetPlcConnection` on two connections (task 4.9, ADR-0154 (i), design D6).
//!
//! Each test holds the first command at a gate inside its transaction, after its locks. The
//! second command starts and waits for the machine lock of the first command. The test proves the
//! wait with `pg_stat_activity` before it opens the gate. No command may end with `E-DB-001`.

use std::future::Future;
use std::sync::Arc;

use recipe_app::auth::{CurrentUser, RequestMeta};
use recipe_app::context::AppContext;
use recipe_app::error::AppError;
use recipe_app::memory::MemoryCredentialCipher;
use recipe_app::plant::{
    ArchiveMachine, ArchiveMachineInput, SetPlcConnection, SetPlcConnectionInput,
    SetPlcConnectionOutput,
};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{CommMode, ConnectionFields, MachineCode, PlcAddress, PlcHost, TcpPort};
use uuid::Uuid;

use crate::support::app::App;
use crate::support::fixtures::TWIN_CAT;
use crate::support::gate::{self, assert_no_deadlock, error_code};

/// Counts the records of a machine.
const RECORD_ROWS: &str = "SELECT count(*) FROM plc_connection WHERE machine_id = $1";
/// The `row_version` of the record of a machine.
const RECORD_VERSION: &str = "SELECT row_version FROM plc_connection WHERE machine_id = $1";
/// The host of the record of a machine.
const RECORD_HOST: &str = "SELECT host FROM plc_connection WHERE machine_id = $1";
/// Counts the audit entries of the save action.
const SAVE_ENTRIES: &str = "SELECT count(*) FROM audit_log WHERE action = 'PlcConnectionSet'";
/// Counts the machines that carry an archive mark.
const ARCHIVED_ROWS: &str =
    "SELECT count(*) FROM machine WHERE id = $1 AND archived_at IS NOT NULL";
/// The condition of the gate on a statement that sets an archive mark.
const ARCHIVING: &str = "NEW.archived_at IS NOT NULL";

/// The context, the user, and the request data that a spawned command owns.
fn who(app: &App) -> (AppContext, CurrentUser, RequestMeta) {
    (app.context(), app.user.clone(), app.meta.clone())
}

/// A save of an `Ads` record of a `TwinCat3` machine. Only `host` differs between two saves.
fn save_input(code: MachineCode, expected: i64, host: &str) -> SetPlcConnectionInput {
    SetPlcConnectionInput {
        code,
        expected_row_version: expected,
        fields: ConnectionFields {
            host: Some(PlcHost::parse(host).unwrap()),
            port: Some(TcpPort::parse("851").unwrap()),
            address: Some(PlcAddress::parse("5.12.34.56.1.1").unwrap()),
            mode: Some(CommMode::parse("Ads").unwrap()),
            ..ConnectionFields::default()
        },
    }
}

/// Start a save as a future that owns its data.
fn save(
    app: &App,
    input: SetPlcConnectionInput,
) -> impl Future<Output = Result<SetPlcConnectionOutput, AppError>> + Send + 'static {
    let (ctx, user, meta) = who(app);
    async move {
        SetPlcConnection::new(ctx, Arc::new(MemoryCredentialCipher::default()))
            .execute(&user, &meta, input)
            .await
    }
}

/// Start `ArchiveMachine` as a future that owns its data.
fn archive(
    app: &App,
    code: MachineCode,
) -> impl Future<Output = Result<(), AppError>> + Send + 'static {
    let (ctx, user, meta) = who(app);
    async move {
        ArchiveMachine::new(ctx)
            .execute(&user, &meta, ArchiveMachineInput { code })
            .await
            .map(|_| ())
    }
}

/// Insert the first record of a machine as `recipe_migrator`. The record starts at `row_version` 1.
async fn seed_record(app: &mut App, machine: Uuid) {
    sqlx::query(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, port, address, comm_mode) \
         VALUES ($1, 'TwinCat3', 'plc-seed.plant.local', 851, '5.12.34.56.1.1', 'Ads')",
    )
    .bind(machine)
    .execute(app.migrator())
    .await
    .unwrap();
}

/// Counts the rows that a query without a parameter returns as one number.
async fn count_all(app: &mut App, query: &str) -> i64 {
    sqlx::query_scalar(query)
        .fetch_one(app.admin())
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_first_saves_give_one_record_and_one_e_web_013() {
    let mut app = App::create_with(&gate::pool()).await;
    let machine = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let winner = save(&app, save_input(machine.code, 0, "plc-winner.plant.local"));
    let loser = save(&app, save_input(machine.code, 0, "plc-loser.plant.local"));
    app.install_gate("plc_connection", "INSERT", "true").await;

    let (winner, loser) = gate::run(&mut app, winner, loser).await;

    assert_no_deadlock(&winner);
    assert_no_deadlock(&loser);
    assert_eq!(winner.unwrap().row_version, 1);
    assert_eq!(error_code(&loser), ErrorCode::WEB_013);
    assert_eq!(app.count(RECORD_ROWS, machine.id).await, 1);
    assert_eq!(app.count(RECORD_VERSION, machine.id).await, 1);
    assert_eq!(count_all(&mut app, SAVE_ENTRIES).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_saves_with_one_row_version_give_one_e_web_013() {
    let mut app = App::create_with(&gate::pool()).await;
    let machine = app.seed_machine(TWIN_CAT, &["U1"]).await;
    seed_record(&mut app, machine.id).await;
    let winner = save(&app, save_input(machine.code, 1, "plc-winner.plant.local"));
    let loser = save(&app, save_input(machine.code, 1, "plc-loser.plant.local"));
    app.install_gate("plc_connection", "UPDATE", "true").await;

    let (winner, loser) = gate::run(&mut app, winner, loser).await;

    assert_no_deadlock(&winner);
    assert_no_deadlock(&loser);
    assert_eq!(winner.unwrap().row_version, 2);
    assert_eq!(error_code(&loser), ErrorCode::WEB_013);
    assert_eq!(app.count(RECORD_VERSION, machine.id).await, 2);
    assert_eq!(count_all(&mut app, SAVE_ENTRIES).await, 1);
    let host: String = sqlx::query_scalar(RECORD_HOST)
        .bind(machine.id)
        .fetch_one(app.admin())
        .await
        .unwrap();
    assert_eq!(host, "plc-winner.plant.local");
}

/// The archive commits first. The save waits for the machine lock and then finds the archive mark.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_against_archive_machine_gives_e_plt_010_when_the_archive_commits_first() {
    let mut app = App::create_with(&gate::pool()).await;
    let machine = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let archive = archive(&app, machine.code);
    let save = save(&app, save_input(machine.code, 0, "plc-late.plant.local"));
    app.install_gate("machine", "UPDATE", ARCHIVING).await;

    let (archive, save) = gate::run(&mut app, archive, save).await;

    assert_no_deadlock(&archive);
    assert_no_deadlock(&save);
    archive.unwrap();
    assert_eq!(error_code(&save), ErrorCode::PLT_010);
    assert_eq!(app.count(ARCHIVED_ROWS, machine.id).await, 1);
    assert_eq!(app.count(RECORD_ROWS, machine.id).await, 0);
    assert_eq!(count_all(&mut app, SAVE_ENTRIES).await, 0);
}

/// The save commits first. The archive waits for the machine lock and then archives the machine.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_against_archive_machine_commits_before_the_archive_when_the_save_goes_first() {
    let mut app = App::create_with(&gate::pool()).await;
    let machine = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let save = save(&app, save_input(machine.code, 0, "plc-early.plant.local"));
    let archive = archive(&app, machine.code);
    app.install_gate("plc_connection", "INSERT", "true").await;

    let (save, archive) = gate::run(&mut app, save, archive).await;

    assert_no_deadlock(&save);
    assert_no_deadlock(&archive);
    assert_eq!(save.unwrap().row_version, 1);
    archive.unwrap();
    assert_eq!(app.count(RECORD_ROWS, machine.id).await, 1);
    assert_eq!(app.count(ARCHIVED_ROWS, machine.id).await, 1);
    assert_eq!(count_all(&mut app, SAVE_ENTRIES).await, 1);
}
