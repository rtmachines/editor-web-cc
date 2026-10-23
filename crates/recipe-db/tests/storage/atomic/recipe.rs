//! Atomic tests of the recipe and version commands (task 10.2).
//!
//! Each command has a success test and a midway failure test. The failure comes from a test
//! trigger that the superuser of the harness creates. The failed command leaves no row and does
//! not move the head of the audit chain.

use std::sync::Arc;

use recipe_app::export::{
    ExportApprovedInput, ExportApprovedVersion, ExportByNumberInput, ExportVersionByNumber,
};
use recipe_app::memory::BodyWriter;
use recipe_app::recipe::{
    ApproveVersion, ApproveVersionInput, CreateRecipe, CreateRecipeInput, CreateVersion,
    CreateVersionInput, DeleteRecipe, DeleteRecipeInput, DiscardDraft, DiscardDraftInput,
    RenameRecipe, RenameRecipeInput, RestoreRecipe, RestoreRecipeInput, RetireVersion,
    RetireVersionInput,
};
use recipe_domain::recipe::ReferenceNumber;
use uuid::Uuid;

use super::world::{DRAFT_ROWS, World, long_text, row_version, second_user, status, text};
use crate::support::app::App;
use crate::support::fault::{FAULT, assert_audit_fault, assert_statement_fault};
use crate::support::fixtures::{self, TWIN_CAT};

const RECIPE_BY_REFERENCE: &str = "SELECT count(*) FROM recipe WHERE reference_number = $1";

/// Count the rows of a query with a text bind.
async fn count_text(app: &mut App, query: &str, value: &str) -> i64 {
    sqlx::query_scalar(query)
        .bind(value)
        .fetch_one(app.admin())
        .await
        .unwrap()
}

/// Insert the version 1 of the world with a parameter and approve it with a fixture row.
async fn approved_base(app: &mut App) -> World {
    let world = World::create(app).await;
    world.add_speed(app, world.version, "SPEED").await;
    fixtures::approve(app.migrator(), world.version, world.creator).await;
    world
}

/// Insert the draft 2 with a parameter on top of the approved version 1.
async fn second_draft(app: &mut App, world: &World) -> Uuid {
    let version =
        fixtures::draft_version(app.migrator(), world.recipe, TWIN_CAT, 2, world.creator).await;
    world.add_speed(app, version, "SPEED").await;
    version
}

// CreateRecipe

fn create_recipe_input(app_code: recipe_domain::plant::MachineCode) -> CreateRecipeInput {
    CreateRecipeInput {
        machine_code: app_code,
        reference: ReferenceNumber::parse("NEW-1").unwrap(),
        name: text("New recipe"),
        change_note: long_text("First version"),
    }
}

#[tokio::test]
async fn create_recipe_commits_the_recipe_the_version_and_two_entries() {
    let mut app = App::create().await;
    let seed = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let before = app.head().await;

    let output = CreateRecipe::new(app.context())
        .execute(&app.user, &app.meta, create_recipe_input(seed.code))
        .await
        .unwrap();

    assert_eq!(count_text(&mut app, RECIPE_BY_REFERENCE, "NEW-1").await, 1);
    let recipe = *output.recipe.as_uuid();
    assert_eq!(
        app.count(
            "SELECT count(*) FROM recipe_version WHERE recipe_id = $1 AND number = 1",
            recipe
        )
        .await,
        1
    );
    app.assert_chain_moved(&before, &["RecipeCreated", "VersionCreated"])
        .await;
}

#[tokio::test]
async fn create_recipe_fails_on_the_version_insert_and_leaves_no_recipe() {
    let mut app = App::create().await;
    let seed = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let before = app.head().await;
    app.install_probe("recipe_version", "BEFORE", "INSERT", FAULT)
        .await;

    let error = CreateRecipe::new(app.context())
        .execute(&app.user, &app.meta, create_recipe_input(seed.code))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(count_text(&mut app, RECIPE_BY_REFERENCE, "NEW-1").await, 0);
    app.assert_chain_unchanged(&before).await;
}

// RenameRecipe

fn rename_input(world: &World) -> RenameRecipeInput {
    RenameRecipeInput {
        reference: world.reference.clone(),
        name: text("Renamed"),
    }
}

const RECIPE_NAME: &str = "SELECT count(*) FROM recipe WHERE id = $1 AND name = 'Renamed'";

#[tokio::test]
async fn rename_recipe_commits_the_name_and_one_entry() {
    let mut app = App::create().await;
    let world = World::create(&mut app).await;
    let before = app.head().await;

    RenameRecipe::new(app.context())
        .execute(&app.user, &app.meta, rename_input(&world))
        .await
        .unwrap();

    assert_eq!(app.count(RECIPE_NAME, world.recipe).await, 1);
    app.assert_chain_moved(&before, &["RecipeRenamed"]).await;
}

#[tokio::test]
async fn rename_recipe_fails_in_the_audit_insert_and_keeps_the_old_name() {
    let mut app = App::create().await;
    let world = World::create(&mut app).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = RenameRecipe::new(app.context())
        .execute(&app.user, &app.meta, rename_input(&world))
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(app.count(RECIPE_NAME, world.recipe).await, 0);
    app.assert_chain_unchanged(&before).await;
}

// DeleteRecipe

fn delete_input(world: &World) -> DeleteRecipeInput {
    DeleteRecipeInput {
        reference: world.reference.clone(),
    }
}

const DELETED_RECIPE: &str = "SELECT count(*) FROM recipe WHERE id = $1 AND deleted_at IS NOT NULL";

#[tokio::test]
async fn delete_recipe_discards_the_draft_deletes_the_recipe_and_writes_two_entries() {
    let mut app = App::create().await;
    let world = World::create(&mut app).await;
    let before = app.head().await;

    DeleteRecipe::new(app.context())
        .execute(&app.user, &app.meta, delete_input(&world))
        .await
        .unwrap();

    assert_eq!(app.count(DELETED_RECIPE, world.recipe).await, 1);
    assert_eq!(app.count(DRAFT_ROWS, world.recipe).await, 0);
    app.assert_chain_moved(&before, &["DraftDiscarded", "RecipeDeleted"])
        .await;
}

#[tokio::test]
async fn delete_recipe_fails_on_the_recipe_update_and_keeps_the_discarded_draft_live() {
    let mut app = App::create().await;
    let world = World::create(&mut app).await;
    let before = app.head().await;
    app.install_probe(
        "recipe",
        "BEFORE",
        "UPDATE",
        "IF NEW.deleted_at IS NOT NULL THEN RAISE EXCEPTION 'injected fault'; END IF;",
    )
    .await;

    let error = DeleteRecipe::new(app.context())
        .execute(&app.user, &app.meta, delete_input(&world))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count(DELETED_RECIPE, world.recipe).await, 0);
    assert_eq!(app.count(DRAFT_ROWS, world.recipe).await, 1);
    app.assert_chain_unchanged(&before).await;
}

// RestoreRecipe

/// Delete the recipe of the world with the use case, so the restore has a deleted recipe.
async fn deleted_world(app: &mut App) -> World {
    let world = World::create(app).await;
    DeleteRecipe::new(app.context())
        .execute(&app.user, &app.meta, delete_input(&world))
        .await
        .unwrap();
    world
}

fn restore_input(world: &World) -> RestoreRecipeInput {
    RestoreRecipeInput {
        reference: world.reference.clone(),
    }
}

#[tokio::test]
async fn restore_recipe_clears_the_delete_mark_and_writes_one_entry() {
    let mut app = App::create().await;
    let world = deleted_world(&mut app).await;
    let before = app.head().await;

    RestoreRecipe::new(app.context())
        .execute(&app.user, &app.meta, restore_input(&world))
        .await
        .unwrap();

    assert_eq!(app.count(DELETED_RECIPE, world.recipe).await, 0);
    app.assert_chain_moved(&before, &["RecipeRestored"]).await;
}

#[tokio::test]
async fn restore_recipe_fails_in_the_audit_insert_and_keeps_the_recipe_deleted() {
    let mut app = App::create().await;
    let world = deleted_world(&mut app).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = RestoreRecipe::new(app.context())
        .execute(&app.user, &app.meta, restore_input(&world))
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(app.count(DELETED_RECIPE, world.recipe).await, 1);
    app.assert_chain_unchanged(&before).await;
}

// CreateVersion

fn create_version_input(world: &World) -> CreateVersionInput {
    CreateVersionInput {
        reference: world.reference.clone(),
        based_on: Some(1),
        change_note: long_text("Second version"),
    }
}

const COPIED_PARAMETERS: &str = "SELECT count(*) FROM parameter p \
     JOIN recipe_version v ON v.id = p.recipe_version_id \
     WHERE v.recipe_id = $1 AND v.number = 2";

#[tokio::test]
async fn create_version_copies_parameters_limits_and_translations_with_the_same_keys() {
    let mut app = App::create().await;
    let world = approved_base(&mut app).await;
    let before = app.head().await;

    CreateVersion::new(app.context())
        .execute(&app.user, &app.meta, create_version_input(&world))
        .await
        .unwrap();

    assert_eq!(app.count(COPIED_PARAMETERS, world.recipe).await, 1);
    let same_keys = app
        .count(
            "SELECT count(*) FROM parameter a JOIN parameter b \
               ON a.parameter_key = b.parameter_key AND a.recipe_version_id <> b.recipe_version_id \
             JOIN recipe_version v ON v.id = b.recipe_version_id \
             WHERE v.recipe_id = $1 AND v.number = 2",
            world.recipe,
        )
        .await;
    assert_eq!(same_keys, 1);
    let children = app
        .count(
            "SELECT (SELECT count(*) FROM parameter_limit l JOIN recipe_version v \
                       ON v.id = l.recipe_version_id WHERE v.recipe_id = $1 AND v.number = 2) \
                  + (SELECT count(*) FROM parameter_translation t JOIN recipe_version v \
                       ON v.id = t.recipe_version_id WHERE v.recipe_id = $1 AND v.number = 2)",
            world.recipe,
        )
        .await;
    assert_eq!(children, 2);
    app.assert_chain_moved(&before, &["VersionCreated"]).await;
}

#[tokio::test]
async fn create_version_fails_on_the_limit_copy_and_leaves_no_version_and_no_parameter() {
    let mut app = App::create().await;
    let world = approved_base(&mut app).await;
    let before = app.head().await;
    app.install_probe("parameter_limit", "BEFORE", "INSERT", FAULT)
        .await;

    let error = CreateVersion::new(app.context())
        .execute(&app.user, &app.meta, create_version_input(&world))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(
        app.count(
            "SELECT count(*) FROM recipe_version WHERE recipe_id = $1",
            world.recipe
        )
        .await,
        1
    );
    assert_eq!(app.count(COPIED_PARAMETERS, world.recipe).await, 0);
    app.assert_chain_unchanged(&before).await;
}

// DiscardDraft

async fn discard_input(app: &mut App, world: &World) -> DiscardDraftInput {
    DiscardDraftInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: world.row_version(app).await,
    }
}

#[tokio::test]
async fn discard_draft_marks_the_draft_deleted_and_writes_one_entry() {
    let mut app = App::create().await;
    let world = World::create(&mut app).await;
    let input = discard_input(&mut app, &world).await;
    let before = app.head().await;

    DiscardDraft::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();

    assert_eq!(app.count(DRAFT_ROWS, world.recipe).await, 0);
    app.assert_chain_moved(&before, &["DraftDiscarded"]).await;
}

#[tokio::test]
async fn discard_draft_fails_in_the_audit_insert_and_keeps_the_draft_live() {
    let mut app = App::create().await;
    let world = World::create(&mut app).await;
    let input = discard_input(&mut app, &world).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = DiscardDraft::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(app.count(DRAFT_ROWS, world.recipe).await, 1);
    app.assert_chain_unchanged(&before).await;
}

// ApproveVersion

async fn approve_input(
    app: &mut App,
    world: &World,
    version: Uuid,
    number: u32,
) -> ApproveVersionInput {
    ApproveVersionInput {
        reference: world.reference.clone(),
        number,
        expected_row_version: row_version(app, version).await,
        allow_outdated: false,
    }
}

#[tokio::test]
async fn approve_version_retires_the_old_approved_version_and_approves_the_draft() {
    let mut app = App::create().await;
    let world = approved_base(&mut app).await;
    let draft = second_draft(&mut app, &world).await;
    let approver = second_user(&mut app).await;
    let input = approve_input(&mut app, &world, draft, 2).await;
    let before = app.head().await;

    let output = ApproveVersion::new(app.context())
        .execute(&approver, &app.meta, input)
        .await
        .unwrap();

    assert_eq!(output.retired, Some(1));
    assert_eq!(status(&mut app, draft).await, "approved");
    assert_eq!(status(&mut app, world.version).await, "retired");
    let hashed = app
        .count(
            "SELECT count(*) FROM recipe_version WHERE id = $1 AND content_sha256 IS NOT NULL",
            draft,
        )
        .await;
    assert_eq!(hashed, 1);
    app.assert_chain_moved(&before, &["VersionRetired", "VersionApproved"])
        .await;
}

#[tokio::test]
async fn approve_version_fails_on_the_second_update_and_keeps_the_old_version_approved() {
    let mut app = App::create().await;
    let world = approved_base(&mut app).await;
    let draft = second_draft(&mut app, &world).await;
    let approver = second_user(&mut app).await;
    let input = approve_input(&mut app, &world, draft, 2).await;
    let before = app.head().await;
    app.install_probe(
        "recipe_version",
        "BEFORE",
        "UPDATE",
        "IF NEW.status = 'approved' THEN RAISE EXCEPTION 'injected fault'; END IF;",
    )
    .await;

    let error = ApproveVersion::new(app.context())
        .execute(&approver, &app.meta, input)
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(status(&mut app, draft).await, "draft");
    assert_eq!(status(&mut app, world.version).await, "approved");
    app.assert_chain_unchanged(&before).await;
}

// RetireVersion

pub(super) async fn retire_input(app: &mut App, world: &World) -> RetireVersionInput {
    RetireVersionInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: row_version(app, world.version).await,
    }
}

#[tokio::test]
async fn retire_version_marks_the_version_retired_and_writes_one_entry() {
    let mut app = App::create().await;
    let world = approved_base(&mut app).await;
    let input = retire_input(&mut app, &world).await;
    let before = app.head().await;

    RetireVersion::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();

    assert_eq!(status(&mut app, world.version).await, "retired");
    app.assert_chain_moved(&before, &["VersionRetired"]).await;
}

#[tokio::test]
async fn retire_version_fails_in_the_audit_insert_and_keeps_the_version_approved() {
    let mut app = App::create().await;
    let world = approved_base(&mut app).await;
    let input = retire_input(&mut app, &world).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = RetireVersion::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(status(&mut app, world.version).await, "approved");
    app.assert_chain_unchanged(&before).await;
}

// ExportVersion

/// Approve the draft of a world with the use case, so the stored hash matches the rebuilt body.
pub(super) async fn approved_by_use_case(app: &mut App) -> World {
    let world = World::create(app).await;
    world.add_speed(app, world.version, "SPEED").await;
    let approver = second_user(app).await;
    let input = approve_input(app, &world, world.version, 1).await;
    ApproveVersion::new(app.context())
        .execute(&approver, &app.meta, input)
        .await
        .unwrap();
    world
}

pub(super) fn export_input(world: &World) -> ExportApprovedInput {
    ExportApprovedInput {
        reference: world.reference.clone(),
    }
}

#[tokio::test]
async fn export_version_writes_one_entry_with_the_content_hash() {
    let mut app = App::create().await;
    let world = approved_by_use_case(&mut app).await;
    let before = app.head().await;

    let exported = ExportApprovedVersion::new(app.context(), Arc::new(BodyWriter))
        .execute(&app.user, &app.meta, export_input(&world))
        .await
        .unwrap();

    app.assert_chain_moved(&before, &["VersionExported"]).await;
    let stored: String = sqlx::query_scalar(
        "SELECT after_state->>'content_sha256' FROM audit_log WHERE action = 'VersionExported'",
    )
    .fetch_one(app.admin())
    .await
    .unwrap();
    assert_eq!(stored, exported.content_sha256.to_string());
}

#[tokio::test]
async fn export_version_fails_in_the_audit_insert_and_leaves_no_entry() {
    let mut app = App::create().await;
    let world = approved_by_use_case(&mut app).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = ExportApprovedVersion::new(app.context(), Arc::new(BodyWriter))
        .execute(&app.user, &app.meta, export_input(&world))
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(
        super::world::actions_named(&mut app, "VersionExported").await,
        0
    );
    app.assert_chain_unchanged(&before).await;
}

// ExportVersion by number of a retired version

/// Approve version 1 with the use case, then retire it with the use case.
async fn retired_by_use_case(app: &mut App) -> World {
    let world = approved_by_use_case(app).await;
    let input = retire_input(app, &world).await;
    RetireVersion::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();
    world
}

pub(super) fn export_by_number_input(world: &World) -> ExportByNumberInput {
    ExportByNumberInput {
        reference: world.reference.clone(),
        number: 1,
    }
}

#[tokio::test]
async fn export_by_number_of_a_retired_version_writes_one_entry_with_the_stored_hash() {
    let mut app = App::create().await;
    let world = retired_by_use_case(&mut app).await;
    assert_eq!(status(&mut app, world.version).await, "retired");
    let before = app.head().await;

    let exported = ExportVersionByNumber::new(app.context(), Arc::new(BodyWriter))
        .execute(&app.user, &app.meta, export_by_number_input(&world))
        .await
        .unwrap();

    app.assert_chain_moved(&before, &["VersionExported"]).await;
    let stored: String =
        sqlx::query_scalar("SELECT content_sha256 FROM recipe_version WHERE id = $1")
            .bind(world.version)
            .fetch_one(app.admin())
            .await
            .unwrap();
    assert_eq!(exported.content_sha256.to_string(), stored);
    let entry: String = sqlx::query_scalar(
        "SELECT after_state->>'content_sha256' FROM audit_log WHERE action = 'VersionExported'",
    )
    .fetch_one(app.admin())
    .await
    .unwrap();
    assert_eq!(entry, stored);
}

#[tokio::test]
async fn export_by_number_of_a_retired_version_fails_in_the_audit_insert_and_leaves_no_entry() {
    let mut app = App::create().await;
    let world = retired_by_use_case(&mut app).await;
    let before = app.head().await;
    app.install_probe("audit_log", "BEFORE", "INSERT", FAULT)
        .await;

    let error = ExportVersionByNumber::new(app.context(), Arc::new(BodyWriter))
        .execute(&app.user, &app.meta, export_by_number_input(&world))
        .await
        .unwrap_err();

    assert_audit_fault(&error);
    assert_eq!(
        super::world::actions_named(&mut app, "VersionExported").await,
        0
    );
    app.assert_chain_unchanged(&before).await;
}
