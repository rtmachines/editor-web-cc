//! Tests of the sections 4.5 and 4.8: `machine_platform_frozen` and `freeze_identifiers`.
//!
//! The tests run as the table owner `recipe_migrator`. A change without a reference passes.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures::{self, Draft, TWIN_CAT};
use crate::support::{Role, TestDatabase, assert_trigger};

async fn plant() -> (PgConnection, Draft) {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let draft = Draft::create(&mut conn).await;
    (conn, draft)
}

/// A plant whose only version is approved. The version holds one parameter in the unit `U1`.
async fn approved_plant() -> (PgConnection, Draft) {
    let (mut conn, draft) = plant().await;
    draft.parameter(&mut conn, "SPEED").await;
    fixtures::approve(&mut conn, draft.version, draft.user).await;
    (conn, draft)
}

async fn set_machine_platform(
    conn: &mut PgConnection,
    machine: Uuid,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    sqlx::query("UPDATE machine SET plc_platform = 'S7_300' WHERE id = $1")
        .bind(machine)
        .execute(conn)
        .await
}

async fn set_machine_code(
    conn: &mut PgConnection,
    machine: Uuid,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    sqlx::query("UPDATE machine SET code = 'M9999999' WHERE id = $1")
        .bind(machine)
        .execute(conn)
        .await
}

// ---- E-PLT-030: platform of a machine with a recipe (4.5) ----

#[tokio::test]
async fn e_plt_030_a_machine_without_a_recipe_can_change_its_platform() {
    let (mut conn, _draft) = plant().await;
    let bare = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = set_machine_platform(&mut conn, bare).await.unwrap();

    assert_eq!(result.rows_affected(), 1);
}

#[tokio::test]
async fn e_plt_030_a_machine_with_a_recipe_cannot_change_its_platform() {
    let (mut conn, draft) = plant().await;

    let result = set_machine_platform(&mut conn, draft.machine).await;

    assert_trigger(result, "E-PLT-030");
}

#[tokio::test]
async fn e_plt_030_an_update_that_keeps_the_platform_passes_with_a_recipe() {
    let (mut conn, draft) = plant().await;

    let result = sqlx::query("UPDATE machine SET plc_platform = 'TwinCat3' WHERE id = $1")
        .bind(draft.machine)
        .execute(&mut conn)
        .await
        .unwrap();

    assert_eq!(result.rows_affected(), 1);
}

// ---- E-PLT-037: code of a machine that a non-draft version references (4.8) ----

#[tokio::test]
async fn e_plt_037_a_machine_with_only_a_draft_can_change_its_code() {
    let (mut conn, draft) = plant().await;

    let result = set_machine_code(&mut conn, draft.machine).await.unwrap();

    assert_eq!(result.rows_affected(), 1);
}

#[tokio::test]
async fn e_plt_037_a_machine_with_an_approved_version_cannot_change_its_code() {
    let (mut conn, draft) = approved_plant().await;

    let result = set_machine_code(&mut conn, draft.machine).await;

    assert_trigger(result, "E-PLT-037");
}

#[tokio::test]
async fn e_plt_037_a_machine_with_a_retired_version_cannot_change_its_code() {
    let (mut conn, draft) = approved_plant().await;
    fixtures::retire(&mut conn, draft.version).await;

    let result = set_machine_code(&mut conn, draft.machine).await;

    assert_trigger(result, "E-PLT-037");
}

// ---- E-PLT-038: path or machine of a unit that a non-draft version references (4.8) ----

async fn set_unit_path(
    conn: &mut PgConnection,
    unit: Uuid,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    sqlx::query("UPDATE unit SET path = 'U9' WHERE id = $1")
        .bind(unit)
        .execute(conn)
        .await
}

#[tokio::test]
async fn e_plt_038_an_unreferenced_unit_can_change_its_path_and_its_machine() {
    let (mut conn, draft) = approved_plant().await;
    let free = fixtures::unit(&mut conn, draft.machine, "U2").await;
    let other_machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let path = set_unit_path(&mut conn, free).await.unwrap();
    let moved = sqlx::query("UPDATE unit SET machine_id = $2 WHERE id = $1")
        .bind(free)
        .bind(other_machine)
        .execute(&mut conn)
        .await
        .unwrap();

    assert_eq!((path.rows_affected(), moved.rows_affected()), (1, 1));
}

#[tokio::test]
async fn e_plt_038_a_unit_that_only_a_draft_references_can_change_its_path() {
    let (mut conn, draft) = plant().await;
    draft.parameter(&mut conn, "SPEED").await;

    let result = set_unit_path(&mut conn, draft.unit).await.unwrap();

    assert_eq!(result.rows_affected(), 1);
}

#[tokio::test]
async fn e_plt_038_a_unit_that_an_approved_version_references_cannot_change_its_path() {
    let (mut conn, draft) = approved_plant().await;

    let result = set_unit_path(&mut conn, draft.unit).await;

    assert_trigger(result, "E-PLT-038");
}

#[tokio::test]
async fn e_plt_038_a_unit_that_an_approved_version_references_cannot_change_its_machine() {
    let (mut conn, draft) = approved_plant().await;
    let other_machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = sqlx::query("UPDATE unit SET machine_id = $2 WHERE id = $1")
        .bind(draft.unit)
        .bind(other_machine)
        .execute(&mut conn)
        .await;

    assert_trigger(result, "E-PLT-038");
}

// ---- E-RCP-034: reference number or machine of a recipe with a non-draft version (4.8) ----

async fn set_reference(
    conn: &mut PgConnection,
    recipe: Uuid,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    sqlx::query("UPDATE recipe SET reference_number = 'CHANGED' WHERE id = $1")
        .bind(recipe)
        .execute(conn)
        .await
}

#[tokio::test]
async fn e_rcp_034_a_recipe_with_only_drafts_can_change_its_reference_and_its_machine() {
    let (mut conn, draft) = plant().await;
    let other_machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let reference = set_reference(&mut conn, draft.recipe).await.unwrap();
    let moved = sqlx::query("UPDATE recipe SET machine_id = $2 WHERE id = $1")
        .bind(draft.recipe)
        .bind(other_machine)
        .execute(&mut conn)
        .await
        .unwrap();

    assert_eq!((reference.rows_affected(), moved.rows_affected()), (1, 1));
}

#[tokio::test]
async fn e_rcp_034_a_recipe_with_an_approved_version_cannot_change_its_reference() {
    let (mut conn, draft) = approved_plant().await;

    let result = set_reference(&mut conn, draft.recipe).await;

    assert_trigger(result, "E-RCP-034");
}

#[tokio::test]
async fn e_rcp_034_a_recipe_with_an_approved_version_cannot_change_its_machine() {
    let (mut conn, draft) = approved_plant().await;
    let other_machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = sqlx::query("UPDATE recipe SET machine_id = $2 WHERE id = $1")
        .bind(draft.recipe)
        .bind(other_machine)
        .execute(&mut conn)
        .await;

    assert_trigger(result, "E-RCP-034");
}
