//! Lock order races (task 10.5, `reliability.md` section 6, row "Concurrency", ADR-0015).
//!
//! Each test holds the first command at a gate inside its transaction, after its locks. The
//! second command starts and waits for a lock of the first command. The test proves the wait with
//! `pg_stat_activity` before it opens the gate. No command may end with `E-DB-001`.

use recipe_app::auth::{CurrentUser, RequestMeta};
use recipe_app::context::AppContext;
use recipe_app::plant::{
    ArchiveMachine, ArchiveMachineInput, ArchiveUnit, ArchiveUnitInput, RenameMachine,
    RenameMachineInput,
};
use recipe_app::recipe::{
    AddParameter, AddParameterInput, CreateVersion, CreateVersionInput, SetParameterValue,
    SetParameterValueInput,
};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{MachineCode, TypeExpr, UnitPath};
use recipe_domain::recipe::{LimitInput, LimitList, Ordinal, ParameterKey, ParameterTag};
use uuid::Uuid;

use crate::atomic::world::{World, actions_named, long_text, text};
use crate::support::app::App;
use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::gate::{self, assert_no_deadlock, error_code};
use crate::triggers::hash_chain::check;

/// The condition of the gate on a statement that sets an archive mark.
const ARCHIVING: &str = "NEW.archived_at IS NOT NULL";
/// Counts the `STRING(20)` types of a machine.
const STRING_20_ROWS: &str = "SELECT count(*) FROM plc_type WHERE machine_id = $1 \
                              AND name = 'STRING' AND string_length = 20";
/// Counts the parameters of a version.
const PARAMETER_ROWS: &str = "SELECT count(*) FROM parameter WHERE recipe_version_id = $1";

/// The context, the user, and the request data that a spawned command owns.
fn who(app: &App) -> (AppContext, CurrentUser, RequestMeta) {
    (app.context(), app.user.clone(), app.meta.clone())
}

/// The live code of a machine.
async fn code_of(app: &mut App, machine: Uuid) -> MachineCode {
    let code: String = sqlx::query_scalar("SELECT code FROM machine WHERE id = $1")
        .bind(machine)
        .fetch_one(app.admin())
        .await
        .unwrap();
    MachineCode::parse(&code).unwrap()
}

fn add_input(world: &World, unit: &str, expected: i64) -> AddParameterInput {
    AddParameterInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: expected,
        unit_path: UnitPath::parse(unit).unwrap(),
        tag: ParameterTag::parse("LABEL").unwrap(),
        name_en: text("Label"),
        description_en: long_text("Label of the zone"),
        type_expr: TypeExpr::parse("STRING(20)").unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(1).unwrap(),
        value_json: "\"abc\"".to_owned(),
        limits: LimitList::parse(Vec::<LimitInput>::new()).unwrap(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn archive_unit_against_add_parameter_with_a_new_type_gives_no_deadlock_and_e_plt_009() {
    let mut app = App::create_with(&gate::pool()).await;
    let world = World::create(&mut app).await;
    fixtures::unit(app.migrator(), world.machine, "U2").await;
    let code = code_of(&mut app, world.machine).await;
    let expected = world.row_version(&mut app).await;
    let (ctx, user, meta) = who(&app);
    let archive_input = ArchiveUnitInput {
        code,
        path: UnitPath::parse("U2").unwrap(),
    };
    let archive = async move {
        ArchiveUnit::new(ctx)
            .execute(&user, &meta, archive_input)
            .await
    };
    let (ctx, user, meta) = who(&app);
    let add_input = add_input(&world, "U2", expected);
    let add = async move {
        AddParameter::new(ctx)
            .execute(&user, &meta, add_input)
            .await
    };
    app.install_gate("unit", "UPDATE", ARCHIVING).await;

    let (archive, add) = gate::run(&mut app, archive, add).await;

    assert_no_deadlock(&archive);
    assert_no_deadlock(&add);
    archive.unwrap();
    assert_eq!(error_code(&add), ErrorCode::PLT_009);
    assert_eq!(app.count(STRING_20_ROWS, world.machine).await, 0);
    assert_eq!(app.count(PARAMETER_ROWS, world.version).await, 0);
}

/// The other order: `AddParameter` holds its class-2 lock and its `plc_type` row at the gate.
///
/// The insert of the `STRING(20)` row takes `FOR KEY SHARE` on the machine through the foreign
/// key. `ArchiveUnit` asks for `FOR NO KEY UPDATE` on the same machine and waits. The command
/// that goes first wins, and the unit then belongs to a live draft.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_parameter_with_a_new_type_against_archive_unit_gives_no_deadlock_and_e_plt_012() {
    let mut app = App::create_with(&gate::pool()).await;
    let world = World::create(&mut app).await;
    fixtures::unit(app.migrator(), world.machine, "U2").await;
    let code = code_of(&mut app, world.machine).await;
    let expected = world.row_version(&mut app).await;
    let (ctx, user, meta) = who(&app);
    let add_input = add_input(&world, "U2", expected);
    let add = async move {
        AddParameter::new(ctx)
            .execute(&user, &meta, add_input)
            .await
    };
    let (ctx, user, meta) = who(&app);
    let archive_input = ArchiveUnitInput {
        code,
        path: UnitPath::parse("U2").unwrap(),
    };
    let archive = async move {
        ArchiveUnit::new(ctx)
            .execute(&user, &meta, archive_input)
            .await
    };
    app.install_gate("parameter", "INSERT", "true").await;

    let (add, archive) = gate::run(&mut app, add, archive).await;

    assert_no_deadlock(&add);
    assert_no_deadlock(&archive);
    add.unwrap();
    assert_eq!(error_code(&archive), ErrorCode::PLT_012);
    assert_eq!(app.count(STRING_20_ROWS, world.machine).await, 1);
    assert_eq!(app.count(PARAMETER_ROWS, world.version).await, 1);
}

fn create_version_input(world: &World) -> CreateVersionInput {
    CreateVersionInput {
        reference: world.reference.clone(),
        based_on: Some(1),
        change_note: long_text("Second version"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn archive_machine_against_create_version_gives_no_deadlock_and_e_plt_010() {
    let mut app = App::create_with(&gate::pool()).await;
    let world = World::create(&mut app).await;
    world.add_speed(&mut app, world.version, "SPEED").await;
    fixtures::approve(app.migrator(), world.version, world.creator).await;
    let code = code_of(&mut app, world.machine).await;
    let (ctx, user, meta) = who(&app);
    let archive = async move {
        ArchiveMachine::new(ctx)
            .execute(&user, &meta, ArchiveMachineInput { code })
            .await
    };
    let (ctx, user, meta) = who(&app);
    let create_input = create_version_input(&world);
    let create = async move {
        CreateVersion::new(ctx)
            .execute(&user, &meta, create_input)
            .await
    };
    app.install_gate("machine", "UPDATE", ARCHIVING).await;

    let (archive, create) = gate::run(&mut app, archive, create).await;

    assert_no_deadlock(&archive);
    assert_no_deadlock(&create);
    archive.unwrap();
    assert_eq!(error_code(&create), ErrorCode::PLT_010);
    assert_eq!(
        app.count(
            "SELECT count(*) FROM recipe_version WHERE recipe_id = $1",
            world.recipe
        )
        .await,
        1
    );
}

fn set_value_input(
    world: &World,
    key: ParameterKey,
    value: &str,
    expected: i64,
) -> SetParameterValueInput {
    SetParameterValueInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: expected,
        key,
        value_json: value.to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_edits_with_one_row_version_let_one_succeed_and_the_other_get_e_web_001() {
    let mut app = App::create_with(&gate::pool()).await;
    let (world, param) = World::with_parameter(&mut app).await;
    let expected = world.row_version(&mut app).await;
    let (ctx, user, meta) = who(&app);
    let winner_input = set_value_input(&world, param.key, "7", expected);
    let winner = async move {
        SetParameterValue::new(ctx)
            .execute(&user, &meta, winner_input)
            .await
    };
    let (ctx, user, meta) = who(&app);
    let loser_input = set_value_input(&world, param.key, "8", expected);
    let loser = async move {
        SetParameterValue::new(ctx)
            .execute(&user, &meta, loser_input)
            .await
    };
    app.install_gate("parameter", "UPDATE", "true").await;

    let (winner, loser) = gate::run(&mut app, winner, loser).await;

    assert_no_deadlock(&winner);
    assert_no_deadlock(&loser);
    winner.unwrap();
    assert_eq!(error_code(&loser), ErrorCode::WEB_001);
    let seven = app
        .count(
            "SELECT count(*) FROM parameter WHERE id = $1 AND value = '7'::jsonb",
            param.id,
        )
        .await;
    assert_eq!(seven, 1);
    assert_eq!(actions_named(&mut app, "ParameterValueSet").await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_concurrent_machine_renames_get_consecutive_chain_sequence_values() {
    let mut app = App::create_with(&gate::pool()).await;
    let first_machine = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let second_machine = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let before = app.head().await;
    let (ctx, user, meta) = who(&app);
    let first_input = RenameMachineInput {
        code: first_machine.code,
        name: text("First"),
    };
    let first = async move {
        RenameMachine::new(ctx)
            .execute(&user, &meta, first_input)
            .await
    };
    let (ctx, user, meta) = who(&app);
    let second_input = RenameMachineInput {
        code: second_machine.code,
        name: text("Second"),
    };
    let second = async move {
        RenameMachine::new(ctx)
            .execute(&user, &meta, second_input)
            .await
    };
    // The trigger of the hash chain fires before the gate. The first insert holds the head row.
    app.install_gate("audit_log", "INSERT", "true").await;

    let (first, second) = gate::run(&mut app, first, second).await;

    assert_no_deadlock(&first);
    assert_no_deadlock(&second);
    first.unwrap();
    second.unwrap();
    app.assert_chain_moved(&before, &["MachineRenamed", "MachineRenamed"])
        .await;
    let after = app.head().await;
    let sequences: Vec<i64> = sqlx::query_scalar(
        "SELECT chain_seq FROM audit_log WHERE chain_seq > $1 ORDER BY chain_seq",
    )
    .bind(before.last_seq)
    .fetch_all(app.admin())
    .await
    .unwrap();
    let next = before.last_seq.checked_add(1).unwrap();
    assert_eq!(sequences, vec![next, next.checked_add(1).unwrap()]);
    let (first_break, rows, last_seq, last_hash) = check(app.admin()).await;
    assert_eq!(first_break, None);
    assert_eq!(rows, after.rows);
    assert_eq!(last_seq, Some(after.last_seq));
    assert_eq!(last_hash, after.last_hash);
}
