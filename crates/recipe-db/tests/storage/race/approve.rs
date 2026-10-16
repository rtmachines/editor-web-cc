//! Approval races (task 10.4, `reliability.md` section 6, row "Concurrency").
//!
//! Each test holds the first command at a gate inside its transaction, after its locks. The
//! second command starts, and the test proves with `pg_stat_activity` that it waits for the first
//! command. Then the gate opens and both commands finish.

use std::future::Future;

use recipe_app::auth::CurrentUser;
use recipe_app::error::AppError;
use recipe_app::recipe::{
    AddParameter, AddParameterInput, ApproveVersion, ApproveVersionInput, DeleteRecipe,
    DeleteRecipeInput, DeleteRecipeOutput, SetParameterValue, SetParameterValueInput,
};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{TypeExpr, UnitPath};
use recipe_domain::recipe::{
    ApprovalOutcome, LimitInput, LimitList, Ordinal, ParameterKey, ParameterTag, ValueInput,
};

use crate::atomic::world::{Param, World, actions_named, long_text, second_user, status, text};
use crate::support::app::App;
use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::gate::{self, assert_no_deadlock, error_code};
use crate::support::{Role, assert_trigger};

/// The condition of the gate on the statement that approves a version.
const APPROVING: &str = "NEW.status = 'approved'";
/// The number of parameters of a version.
const PARAMETER_ROWS: &str = "SELECT count(*) FROM parameter WHERE recipe_version_id = $1";

/// A world with the parameter `SPEED` and a second administrator who can approve it.
async fn approvable(app: &mut App) -> (World, Param, CurrentUser) {
    let (world, param) = World::with_parameter(app).await;
    let approver = second_user(app).await;
    (world, param, approver)
}

fn approve_input(world: &World, expected_row_version: i64) -> ApproveVersionInput {
    ApproveVersionInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version,
        allow_outdated: false,
    }
}

/// Start `ApproveVersion` as a future that owns its inputs.
fn approve(
    app: &App,
    user: &CurrentUser,
    input: &ApproveVersionInput,
) -> impl Future<Output = Result<ApprovalOutcome, AppError>> + Send + 'static {
    let (ctx, user, meta, input) = (app.context(), user.clone(), app.meta.clone(), input.clone());
    async move { ApproveVersion::new(ctx).execute(&user, &meta, input).await }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_approvals_with_one_row_version_write_once_and_the_loser_gets_already_approved() {
    let mut app = App::create_with(&gate::pool()).await;
    let (world, _, approver) = approvable(&mut app).await;
    let input = approve_input(&world, world.row_version(&mut app).await);
    let before = app.head().await;
    app.install_gate("recipe_version", "UPDATE", APPROVING)
        .await;

    let (winner, loser) = (
        approve(&app, &approver, &input),
        approve(&app, &approver, &input),
    );

    let (first, second) = gate::run(&mut app, winner, loser).await;

    assert_no_deadlock(&first);
    assert_no_deadlock(&second);
    assert!(
        matches!(first, Ok(ApprovalOutcome::Approved { .. })),
        "{first:?}"
    );
    assert!(
        matches!(second, Ok(ApprovalOutcome::AlreadyApproved)),
        "{second:?}"
    );
    assert_eq!(actions_named(&mut app, "VersionApproved").await, 1);
    assert_eq!(status(&mut app, world.version).await, "approved");
    app.assert_chain_moved(&before, &["VersionApproved"]).await;
}

fn set_value_input(world: &World, key: ParameterKey, expected: i64) -> SetParameterValueInput {
    SetParameterValueInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: expected,
        key,
        value: ValueInput::Json("7".to_owned()),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_approval_with_an_older_row_version_gets_e_web_001_after_an_edit_commits() {
    let mut app = App::create_with(&gate::pool()).await;
    let (world, param, approver) = approvable(&mut app).await;
    let expected = world.row_version(&mut app).await;
    let input = approve_input(&world, expected);
    let (ctx, user, meta) = (app.context(), app.user.clone(), app.meta.clone());
    let edit_input = set_value_input(&world, param.key, expected);
    let edit = async move {
        SetParameterValue::new(ctx)
            .execute(&user, &meta, edit_input)
            .await
    };
    app.install_gate("parameter", "UPDATE", "true").await;

    let approval = approve(&app, &approver, &input);

    let (edit, approval) = gate::run(&mut app, edit, approval).await;

    assert_no_deadlock(&edit);
    assert_no_deadlock(&approval);
    edit.unwrap();
    assert_eq!(error_code(&approval), ErrorCode::WEB_001);
    assert_eq!(status(&mut app, world.version).await, "draft");
    assert_eq!(actions_named(&mut app, "VersionApproved").await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_raw_parameter_insert_waits_for_the_approval_and_then_gets_e_rcp_031() {
    let mut app = App::create_with(&gate::pool()).await;
    let (world, _, approver) = approvable(&mut app).await;
    let input = approve_input(&world, world.row_version(&mut app).await);
    let dint = fixtures::primitive(app.migrator(), "DINT").await;
    let mut session = app.session(Role::App).await;
    let (version, unit) = (world.version, world.unit);
    let raw_insert = async move {
        sqlx::query(
            "INSERT INTO parameter \
                 (recipe_version_id, plc_platform, unit_id, tag, name_en, description_en, \
                  plc_type_id, value) \
             VALUES ($1, $2, $3, 'RAW', 'name', 'description', $4, '0'::jsonb)",
        )
        .bind(version)
        .bind(TWIN_CAT)
        .bind(unit)
        .bind(dint)
        .execute(&mut session)
        .await
    };
    app.install_gate("recipe_version", "UPDATE", APPROVING)
        .await;

    let approval = approve(&app, &approver, &input);

    let (approval, insert) = gate::run(&mut app, approval, raw_insert).await;

    assert!(
        matches!(approval, Ok(ApprovalOutcome::Approved { .. })),
        "{approval:?}"
    );
    assert_trigger(insert, "E-RCP-031");
    assert_eq!(app.count(PARAMETER_ROWS, world.version).await, 1);
    assert_eq!(status(&mut app, world.version).await, "approved");
}

fn add_input(world: &World, expected: i64) -> AddParameterInput {
    AddParameterInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: expected,
        unit_path: UnitPath::parse("U1").unwrap(),
        tag: ParameterTag::parse("LABEL").unwrap(),
        name_en: text("Label"),
        description_en: long_text("Label of the zone"),
        type_expr: TypeExpr::parse("STRING(20)").unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(1).unwrap(),
        value: ValueInput::Json("\"abc\"".to_owned()),
        limits: LimitList::parse(Vec::<LimitInput>::new()).unwrap(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_parameter_waits_for_the_approval_and_then_gets_e_rcp_002_from_the_domain() {
    let mut app = App::create_with(&gate::pool()).await;
    let (world, _, approver) = approvable(&mut app).await;
    let expected = world.row_version(&mut app).await;
    let input = approve_input(&world, expected);
    let (ctx, user, meta) = (app.context(), app.user.clone(), app.meta.clone());
    let add_input = add_input(&world, expected);
    let add = async move {
        AddParameter::new(ctx)
            .execute(&user, &meta, add_input)
            .await
    };
    app.install_gate("recipe_version", "UPDATE", APPROVING)
        .await;

    let approval = approve(&app, &approver, &input);

    let (approval, add) = gate::run(&mut app, approval, add).await;

    assert_no_deadlock(&approval);
    assert_no_deadlock(&add);
    assert!(
        matches!(approval, Ok(ApprovalOutcome::Approved { .. })),
        "{approval:?}"
    );
    assert_eq!(error_code(&add), ErrorCode::RCP_002);
    assert_eq!(app.count(PARAMETER_ROWS, world.version).await, 1);
}

fn delete_input(world: &World) -> DeleteRecipeInput {
    DeleteRecipeInput {
        reference: world.reference.clone(),
    }
}

/// Start `DeleteRecipe` as a future that owns its inputs.
fn delete(
    app: &App,
    world: &World,
) -> impl Future<Output = Result<DeleteRecipeOutput, AppError>> + Send + 'static {
    let (ctx, user, meta, input) = (
        app.context(),
        app.user.clone(),
        app.meta.clone(),
        delete_input(world),
    );
    async move { DeleteRecipe::new(ctx).execute(&user, &meta, input).await }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_recipe_after_the_approval_started_fails_with_e_rcp_010() {
    // Order of the gate: the approval holds the recipe lock first, so `DeleteRecipe` waits and
    // then finds the approved version.
    let mut app = App::create_with(&gate::pool()).await;
    let (world, _, approver) = approvable(&mut app).await;
    let input = approve_input(&world, world.row_version(&mut app).await);
    app.install_gate("recipe_version", "UPDATE", APPROVING)
        .await;

    let (approval, deletion) = (approve(&app, &approver, &input), delete(&app, &world));

    let (approval, deletion) = gate::run(&mut app, approval, deletion).await;

    assert_no_deadlock(&approval);
    assert_no_deadlock(&deletion);
    assert!(
        matches!(approval, Ok(ApprovalOutcome::Approved { .. })),
        "{approval:?}"
    );
    assert_eq!(error_code(&deletion), ErrorCode::RCP_010);
    assert_eq!(status(&mut app, world.version).await, "approved");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_approval_after_delete_recipe_started_fails() {
    // Order of the gate: `DeleteRecipe` holds the recipe lock first, so the approval waits and
    // then finds the discarded draft.
    let mut app = App::create_with(&gate::pool()).await;
    let (world, _, approver) = approvable(&mut app).await;
    let input = approve_input(&world, world.row_version(&mut app).await);
    app.install_gate("recipe", "UPDATE", "NEW.deleted_at IS NOT NULL")
        .await;

    let (deletion, approval) = (delete(&app, &world), approve(&app, &approver, &input));

    let (deletion, approval) = gate::run(&mut app, deletion, approval).await;

    assert_no_deadlock(&deletion);
    assert_no_deadlock(&approval);
    deletion.unwrap();
    assert_eq!(error_code(&approval), ErrorCode::RCP_012);
    assert_eq!(actions_named(&mut app, "VersionApproved").await, 0);
}
