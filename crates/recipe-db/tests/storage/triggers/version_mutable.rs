//! Tests of the sections 4.1 to 4.3: `enforce_version_mutable`, `enforce_type_platform`, and
//! `enforce_unit_machine`.
//!
//! Each code has a positive test that shows the allowed write and a negative test that asserts
//! SQLSTATE `23514` and the code at the start of the message.

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

/// A draft with one parameter, one limit, and one translation.
async fn filled_plant() -> (PgConnection, Draft, Uuid) {
    let (mut conn, draft) = plant().await;
    let parameter = draft.parameter(&mut conn, "SPEED").await;
    fixtures::limit(&mut conn, parameter, draft.version, "").await;
    fixtures::translation(&mut conn, parameter, draft.version, "pl").await;
    (conn, draft, parameter)
}

async fn count(conn: &mut PgConnection, table: &str, parameter: Uuid) -> i64 {
    let query = match table {
        "parameter" => "SELECT count(*) FROM parameter WHERE id = $1",
        "parameter_limit" => "SELECT count(*) FROM parameter_limit WHERE parameter_id = $1",
        _ => "SELECT count(*) FROM parameter_translation WHERE parameter_id = $1",
    };
    sqlx::query_scalar(query)
        .bind(parameter)
        .fetch_one(conn)
        .await
        .unwrap()
}

// ---- E-RCP-031: write to a version that is not a live draft ----

#[tokio::test]
async fn e_rcp_031_a_live_draft_accepts_the_writes_of_the_three_tables() {
    let (mut conn, _draft, parameter) = filled_plant().await;

    let updated = sqlx::query("UPDATE parameter SET ordinal = 5 WHERE id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("UPDATE parameter_limit SET max_value = 20 WHERE parameter_id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("UPDATE parameter_translation SET name = 'inna' WHERE parameter_id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("DELETE FROM parameter_translation WHERE parameter_id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("DELETE FROM parameter_limit WHERE parameter_id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await
        .unwrap();

    assert_eq!(updated.rows_affected(), 1);
    assert_eq!(count(&mut conn, "parameter", parameter).await, 1);
    assert_eq!(count(&mut conn, "parameter_limit", parameter).await, 0);
    assert_eq!(
        count(&mut conn, "parameter_translation", parameter).await,
        0
    );
}

#[tokio::test]
async fn e_rcp_031_an_approved_version_rejects_every_write_to_parameter() {
    let (mut conn, draft, parameter) = filled_plant().await;
    fixtures::approve(&mut conn, draft.version, draft.user).await;

    let insert = draft_insert(&mut conn, &draft, "OTHER").await;
    let update = sqlx::query("UPDATE parameter SET ordinal = 5 WHERE id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await;
    let delete = sqlx::query("DELETE FROM parameter WHERE id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await;

    assert_trigger(insert, "E-RCP-031");
    assert_trigger(update, "E-RCP-031");
    assert_trigger(delete, "E-RCP-031");
    assert_eq!(count(&mut conn, "parameter", parameter).await, 1);
}

async fn draft_insert(
    conn: &mut PgConnection,
    draft: &Draft,
    tag: &str,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    sqlx::query(
        "INSERT INTO parameter \
             (recipe_version_id, plc_platform, unit_id, tag, name_en, description_en, \
              plc_type_id, value) \
         VALUES ($1, $2, $3, $4, 'name', 'description', $5, '0'::jsonb)",
    )
    .bind(draft.version)
    .bind(TWIN_CAT)
    .bind(draft.unit)
    .bind(tag)
    .bind(draft.dint)
    .execute(conn)
    .await
}

#[tokio::test]
async fn e_rcp_031_an_approved_version_rejects_every_write_to_parameter_limit() {
    let (mut conn, draft, parameter) = filled_plant().await;
    fixtures::approve(&mut conn, draft.version, draft.user).await;

    let insert = sqlx::query(
        "INSERT INTO parameter_limit (parameter_id, recipe_version_id, leaf_path, max_value) \
         VALUES ($1, $2, '.x', 1)",
    )
    .bind(parameter)
    .bind(draft.version)
    .execute(&mut conn)
    .await;
    let update = sqlx::query("UPDATE parameter_limit SET max_value = 20 WHERE parameter_id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await;
    let delete = sqlx::query("DELETE FROM parameter_limit WHERE parameter_id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await;

    assert_trigger(insert, "E-RCP-031");
    assert_trigger(update, "E-RCP-031");
    assert_trigger(delete, "E-RCP-031");
    assert_eq!(count(&mut conn, "parameter_limit", parameter).await, 1);
}

#[tokio::test]
async fn e_rcp_031_an_approved_version_rejects_every_write_to_parameter_translation() {
    let (mut conn, draft, parameter) = filled_plant().await;
    fixtures::approve(&mut conn, draft.version, draft.user).await;

    let insert = sqlx::query(
        "INSERT INTO parameter_translation (parameter_id, recipe_version_id, lang, name) \
         VALUES ($1, $2, 'de', 'name')",
    )
    .bind(parameter)
    .bind(draft.version)
    .execute(&mut conn)
    .await;
    let update =
        sqlx::query("UPDATE parameter_translation SET name = 'inna' WHERE parameter_id = $1")
            .bind(parameter)
            .execute(&mut conn)
            .await;
    let delete = sqlx::query("DELETE FROM parameter_translation WHERE parameter_id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await;

    assert_trigger(insert, "E-RCP-031");
    assert_trigger(update, "E-RCP-031");
    assert_trigger(delete, "E-RCP-031");
    assert_eq!(
        count(&mut conn, "parameter_translation", parameter).await,
        1
    );
}

#[tokio::test]
async fn e_rcp_031_a_discarded_draft_rejects_a_new_parameter() {
    let (mut conn, draft) = plant().await;
    sqlx::query("UPDATE recipe_version SET deleted_at = now() WHERE id = $1")
        .bind(draft.version)
        .execute(&mut conn)
        .await
        .unwrap();

    let insert = draft_insert(&mut conn, &draft, "LATE").await;

    assert_trigger(insert, "E-RCP-031");
    let live_drafts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM recipe_version \
         WHERE recipe_id = $1 AND status = 'draft' AND deleted_at IS NULL",
    )
    .bind(draft.recipe)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(live_drafts, 0);
}

// ---- E-RCP-030: row moved to another version ----

#[tokio::test]
async fn e_rcp_030_an_update_inside_one_version_passes_on_the_three_tables() {
    let (mut conn, draft, parameter) = filled_plant().await;

    let result = sqlx::query("UPDATE parameter SET recipe_version_id = $2 WHERE id = $1")
        .bind(parameter)
        .bind(draft.version)
        .execute(&mut conn)
        .await
        .unwrap();

    assert_eq!(result.rows_affected(), 1);
}

#[tokio::test]
async fn e_rcp_030_a_parameter_cannot_move_to_another_version() {
    let (mut conn, draft, parameter) = filled_plant().await;
    let other = draft.other_draft(&mut conn).await;

    let moved = sqlx::query("UPDATE parameter SET recipe_version_id = $2 WHERE id = $1")
        .bind(parameter)
        .bind(other)
        .execute(&mut conn)
        .await;

    assert_trigger(moved, "E-RCP-030");
}

#[tokio::test]
async fn e_rcp_030_a_limit_and_a_translation_cannot_move_to_another_version() {
    let (mut conn, draft, parameter) = filled_plant().await;
    let other = draft.other_draft(&mut conn).await;

    let limit =
        sqlx::query("UPDATE parameter_limit SET recipe_version_id = $2 WHERE parameter_id = $1")
            .bind(parameter)
            .bind(other)
            .execute(&mut conn)
            .await;
    let translation = sqlx::query(
        "UPDATE parameter_translation SET recipe_version_id = $2 WHERE parameter_id = $1",
    )
    .bind(parameter)
    .bind(other)
    .execute(&mut conn)
    .await;

    assert_trigger(limit, "E-RCP-030");
    assert_trigger(translation, "E-RCP-030");
}

// ---- Cascade branch of 4.1 ----

#[tokio::test]
async fn deleting_a_parameter_in_a_live_draft_cascades_to_its_limits_and_translations() {
    let (mut conn, _draft, parameter) = filled_plant().await;

    let deleted = sqlx::query("DELETE FROM parameter WHERE id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await
        .unwrap();

    assert_eq!(deleted.rows_affected(), 1);
    assert_eq!(count(&mut conn, "parameter", parameter).await, 0);
    assert_eq!(count(&mut conn, "parameter_limit", parameter).await, 0);
    assert_eq!(
        count(&mut conn, "parameter_translation", parameter).await,
        0
    );
}

#[tokio::test]
async fn deleting_a_parameter_of_an_approved_version_keeps_its_children() {
    let (mut conn, draft, parameter) = filled_plant().await;
    fixtures::approve(&mut conn, draft.version, draft.user).await;

    let deleted = sqlx::query("DELETE FROM parameter WHERE id = $1")
        .bind(parameter)
        .execute(&mut conn)
        .await;

    assert_trigger(deleted, "E-RCP-031");
    assert_eq!(count(&mut conn, "parameter_limit", parameter).await, 1);
    assert_eq!(
        count(&mut conn, "parameter_translation", parameter).await,
        1
    );
}

// ---- E-PLT-031 and E-PLT-032: type and platform (4.2) ----

#[tokio::test]
async fn e_plt_031_a_type_on_the_platform_of_the_version_passes() {
    let (mut conn, draft) = plant().await;

    let parameter = draft.parameter(&mut conn, "OK").await;

    assert_eq!(count(&mut conn, "parameter", parameter).await, 1);
}

#[tokio::test]
async fn e_plt_031_a_type_that_does_not_exist_on_the_platform_is_rejected() {
    let (mut conn, draft) = plant().await;
    let only_s7 = fixtures::primitive_on(&mut conn, "ONLYS7", &["S7_1500"]).await;

    let insert = sqlx::query(
        "INSERT INTO parameter \
             (recipe_version_id, plc_platform, unit_id, tag, name_en, description_en, \
              plc_type_id, value) \
         VALUES ($1, $2, $3, 'BADTYPE', 'name', 'description', $4, '0'::jsonb)",
    )
    .bind(draft.version)
    .bind(TWIN_CAT)
    .bind(draft.unit)
    .bind(only_s7)
    .execute(&mut conn)
    .await;
    let update = sqlx::query("UPDATE parameter SET plc_type_id = $2 WHERE id = $1")
        .bind(draft.parameter(&mut conn, "GOOD").await)
        .bind(only_s7)
        .execute(&mut conn)
        .await;

    assert_trigger(insert, "E-PLT-031");
    assert_trigger(update, "E-PLT-031");
}

#[tokio::test]
async fn e_plt_032_a_type_of_the_machine_of_the_recipe_passes() {
    let (mut conn, draft) = plant().await;
    let own = fixtures::machine_string(&mut conn, draft.machine, TWIN_CAT, 20).await;

    let parameter =
        fixtures::parameter(&mut conn, draft.version, TWIN_CAT, draft.unit, own, "OWN").await;

    assert_eq!(count(&mut conn, "parameter", parameter).await, 1);
}

#[tokio::test]
async fn e_plt_032_a_type_of_another_machine_is_rejected() {
    let (mut conn, draft) = plant().await;
    let other_machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    let foreign = fixtures::machine_string(&mut conn, other_machine, TWIN_CAT, 20).await;

    let insert = sqlx::query(
        "INSERT INTO parameter \
             (recipe_version_id, plc_platform, unit_id, tag, name_en, description_en, \
              plc_type_id, value) \
         VALUES ($1, $2, $3, 'FOREIGN', 'name', 'description', $4, '0'::jsonb)",
    )
    .bind(draft.version)
    .bind(TWIN_CAT)
    .bind(draft.unit)
    .bind(foreign)
    .execute(&mut conn)
    .await;

    assert_trigger(insert, "E-PLT-032");
}

// ---- E-RCP-032: unit and machine (4.3) ----

#[tokio::test]
async fn e_rcp_032_a_unit_of_the_machine_of_the_recipe_passes() {
    let (mut conn, draft) = plant().await;
    let second_unit = fixtures::unit(&mut conn, draft.machine, "U2").await;

    let parameter = fixtures::parameter(
        &mut conn,
        draft.version,
        TWIN_CAT,
        second_unit,
        draft.dint,
        "SECOND",
    )
    .await;

    assert_eq!(count(&mut conn, "parameter", parameter).await, 1);
}

#[tokio::test]
async fn e_rcp_032_a_unit_of_another_machine_is_rejected_on_insert_and_update() {
    let (mut conn, draft) = plant().await;
    let other_machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    let foreign_unit = fixtures::unit(&mut conn, other_machine, "U1").await;
    let own = draft.parameter(&mut conn, "OWN").await;

    let insert = sqlx::query(
        "INSERT INTO parameter \
             (recipe_version_id, plc_platform, unit_id, tag, name_en, description_en, \
              plc_type_id, value) \
         VALUES ($1, $2, $3, 'FOREIGN', 'name', 'description', $4, '0'::jsonb)",
    )
    .bind(draft.version)
    .bind(TWIN_CAT)
    .bind(foreign_unit)
    .bind(draft.dint)
    .execute(&mut conn)
    .await;
    let update = sqlx::query("UPDATE parameter SET unit_id = $2 WHERE id = $1")
        .bind(own)
        .bind(foreign_unit)
        .execute(&mut conn)
        .await;

    assert_trigger(insert, "E-RCP-032");
    assert_trigger(update, "E-RCP-032");
}
