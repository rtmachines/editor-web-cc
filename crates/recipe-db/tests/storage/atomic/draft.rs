//! Atomic tests of the eight draft commands (task 10.3).
//!
//! Each draft command takes the lock, compares `row_version`, touches the draft, and writes
//! one `draft_editor` row. A success test checks that touch. A midway failure test makes a later
//! statement fail, so the touch and the editor row vanish with the rest.

use std::collections::BTreeMap;

use recipe_app::recipe::{
    AddParameter, AddParameterInput, RebindTypeRevision, RebindTypeRevisionInput, RemoveParameter,
    RemoveParameterInput, RemoveTranslation, RemoveTranslationInput, ReplaceLimits,
    ReplaceLimitsInput, RetypeInput, SetParameterValue, SetParameterValueInput,
    UpdateParameterDefinition, UpdateParameterDefinitionInput, UpsertTranslation,
    UpsertTranslationInput,
};
use recipe_domain::plant::{LeafPath, PlcTypeId, TypeExpr, UnitPath};
use recipe_domain::recipe::{
    LangCode, LimitInput, LimitList, MemberValue, Ordinal, ParameterKey, ParameterTag, Translation,
    ValueInput,
};
use serde_json::json;
use uuid::Uuid;

use super::world::{Param, World, long_text, text};
use crate::support::app::App;
use crate::support::fault::{FAULT, assert_statement_fault};
use crate::support::fixtures::{self, TWIN_CAT};

const EDITORS: &str = "SELECT count(*) FROM draft_editor WHERE recipe_version_id = $1";

/// The row version before a command, for the checks of the touch.
async fn rv(app: &mut App, world: &World) -> i64 {
    world.row_version(app).await
}

/// Assert that the command touched the draft once and wrote one editor row.
async fn assert_touched(app: &mut App, world: &World, before: i64) {
    assert_eq!(rv(app, world).await, before.checked_add(1).unwrap());
    assert_eq!(app.count(EDITORS, world.version).await, 1);
}

/// Assert that the touch and the editor row vanished.
async fn assert_untouched(app: &mut App, world: &World, before: i64) {
    assert_eq!(rv(app, world).await, before);
    assert_eq!(app.count(EDITORS, world.version).await, 0);
}

fn limit(min: &str, max: &str) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some(min.to_owned()),
        max: Some(max.to_owned()),
        max_length: None,
    }
}

fn limits(rows: Vec<LimitInput>) -> LimitList {
    LimitList::parse(rows).unwrap()
}

fn lang(code: &str) -> LangCode {
    LangCode::parse(code).unwrap()
}

async fn world_with_speed(app: &mut App) -> (World, Param) {
    World::with_parameter(app).await
}

// AddParameter

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
        limits: limits(vec![]),
    }
}

const STRING_20_ROWS: &str = "SELECT count(*) FROM plc_type WHERE machine_id = $1 \
                              AND name = 'STRING' AND string_length = 20";
const PARAMETER_ROWS: &str = "SELECT count(*) FROM parameter WHERE recipe_version_id = $1";

#[tokio::test]
async fn add_parameter_commits_the_parameter_the_derived_type_and_the_touch() {
    let mut app = App::create().await;
    let (world, _) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;

    AddParameter::new(app.context())
        .execute(&app.user, &app.meta, add_input(&world, expected))
        .await
        .unwrap();

    assert_eq!(app.count(PARAMETER_ROWS, world.version).await, 2);
    assert_eq!(app.count(STRING_20_ROWS, world.machine).await, 1);
    assert_touched(&mut app, &world, expected).await;
    app.assert_chain_moved(&before, &["DerivedTypeCreated", "ParameterAdded"])
        .await;
}

#[tokio::test]
async fn add_parameter_fails_on_the_parameter_insert_and_leaves_no_type_row_and_no_touch() {
    let mut app = App::create().await;
    let (world, _) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;
    app.install_probe("parameter", "BEFORE", "INSERT", FAULT)
        .await;

    let error = AddParameter::new(app.context())
        .execute(&app.user, &app.meta, add_input(&world, expected))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count(PARAMETER_ROWS, world.version).await, 1);
    assert_eq!(app.count(STRING_20_ROWS, world.machine).await, 0);
    assert_untouched(&mut app, &world, expected).await;
    app.assert_chain_unchanged(&before).await;
}

// UpdateParameterDefinition

fn update_input(world: &World, param: &Param, expected: i64) -> UpdateParameterDefinitionInput {
    UpdateParameterDefinitionInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: expected,
        key: param.key,
        unit_path: UnitPath::parse("U1").unwrap(),
        tag: ParameterTag::parse("SPEED").unwrap(),
        name_en: text("Speed"),
        description_en: long_text("Speed of the belt"),
        type_expr: TypeExpr::parse("INT").unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(0).unwrap(),
        retype: Some(RetypeInput {
            value: ValueInput::Json("5".to_owned()),
            limits: limits(vec![limit("1", "9")]),
        }),
    }
}

const TYPE_IS_INT: &str = "SELECT count(*) FROM parameter p JOIN plc_type t ON t.id = p.plc_type_id \
                           WHERE p.id = $1 AND t.name = 'INT' AND p.value = '5'::jsonb";
const TYPE_IS_DINT: &str = "SELECT count(*) FROM parameter p JOIN plc_type t ON t.id = p.plc_type_id \
                            WHERE p.id = $1 AND t.name = 'DINT' AND p.value = '0'::jsonb";
const LIMIT_MAX_9: &str =
    "SELECT count(*) FROM parameter_limit WHERE parameter_id = $1 AND max_value = 9";
const LIMIT_MAX_10: &str =
    "SELECT count(*) FROM parameter_limit WHERE parameter_id = $1 AND max_value = 10";
const TRANSLATIONS: &str = "SELECT count(*) FROM parameter_translation WHERE parameter_id = $1";

#[tokio::test]
async fn update_definition_commits_the_new_type_value_limits_and_the_touch() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;

    UpdateParameterDefinition::new(app.context())
        .execute(&app.user, &app.meta, update_input(&world, &param, expected))
        .await
        .unwrap();

    assert_eq!(app.count(TYPE_IS_INT, param.id).await, 1);
    assert_eq!(app.count(LIMIT_MAX_9, param.id).await, 1);
    assert_eq!(app.count(TRANSLATIONS, param.id).await, 1);
    assert_touched(&mut app, &world, expected).await;
    app.assert_chain_moved(&before, &["ParameterDefinitionUpdated"])
        .await;
}

#[tokio::test]
async fn update_definition_fails_on_the_limit_insert_and_keeps_type_value_and_limits() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;
    app.install_probe("parameter_limit", "BEFORE", "INSERT", FAULT)
        .await;

    let error = UpdateParameterDefinition::new(app.context())
        .execute(&app.user, &app.meta, update_input(&world, &param, expected))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count(TYPE_IS_DINT, param.id).await, 1);
    assert_eq!(app.count(LIMIT_MAX_10, param.id).await, 1);
    assert_untouched(&mut app, &world, expected).await;
    app.assert_chain_unchanged(&before).await;
}

// SetParameterValue

fn set_value_input(world: &World, param: &Param, expected: i64) -> SetParameterValueInput {
    SetParameterValueInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: expected,
        key: param.key,
        value: ValueInput::Json("7".to_owned()),
    }
}

const VALUE_7: &str = "SELECT count(*) FROM parameter WHERE id = $1 AND value = '7'::jsonb";

#[tokio::test]
async fn set_value_commits_the_value_and_keeps_limits_and_translations() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;

    SetParameterValue::new(app.context())
        .execute(
            &app.user,
            &app.meta,
            set_value_input(&world, &param, expected),
        )
        .await
        .unwrap();

    assert_eq!(app.count(VALUE_7, param.id).await, 1);
    assert_eq!(app.count(LIMIT_MAX_10, param.id).await, 1);
    assert_eq!(app.count(TRANSLATIONS, param.id).await, 1);
    assert_touched(&mut app, &world, expected).await;
    app.assert_chain_moved(&before, &["ParameterValueSet"])
        .await;
}

#[tokio::test]
async fn set_value_fails_on_the_parameter_update_and_keeps_the_value_and_the_touch_state() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;
    app.install_probe("parameter", "BEFORE", "UPDATE", FAULT)
        .await;

    let error = SetParameterValue::new(app.context())
        .execute(
            &app.user,
            &app.meta,
            set_value_input(&world, &param, expected),
        )
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count(VALUE_7, param.id).await, 0);
    assert_untouched(&mut app, &world, expected).await;
    app.assert_chain_unchanged(&before).await;
}

// ReplaceLimits

fn replace_limits_input(world: &World, param: &Param, expected: i64) -> ReplaceLimitsInput {
    ReplaceLimitsInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: expected,
        key: param.key,
        limits: limits(vec![limit("-5", "20")]),
    }
}

const LIMIT_MAX_20: &str =
    "SELECT count(*) FROM parameter_limit WHERE parameter_id = $1 AND max_value = 20";

#[tokio::test]
async fn replace_limits_commits_the_new_limit_and_the_touch() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;

    ReplaceLimits::new(app.context())
        .execute(
            &app.user,
            &app.meta,
            replace_limits_input(&world, &param, expected),
        )
        .await
        .unwrap();

    assert_eq!(app.count(LIMIT_MAX_20, param.id).await, 1);
    assert_eq!(app.count(LIMIT_MAX_10, param.id).await, 0);
    assert_touched(&mut app, &world, expected).await;
    app.assert_chain_moved(&before, &["LimitsReplaced"]).await;
}

#[tokio::test]
async fn replace_limits_fails_on_the_limit_insert_and_keeps_the_old_limit() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;
    app.install_probe("parameter_limit", "BEFORE", "INSERT", FAULT)
        .await;

    let error = ReplaceLimits::new(app.context())
        .execute(
            &app.user,
            &app.meta,
            replace_limits_input(&world, &param, expected),
        )
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count(LIMIT_MAX_10, param.id).await, 1);
    assert_eq!(app.count(LIMIT_MAX_20, param.id).await, 0);
    assert_untouched(&mut app, &world, expected).await;
    app.assert_chain_unchanged(&before).await;
}

// UpsertTranslation

fn upsert_input(world: &World, param: &Param, expected: i64) -> UpsertTranslationInput {
    UpsertTranslationInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: expected,
        key: param.key,
        translation: Translation {
            lang: lang("de"),
            name: text("Geschwindigkeit"),
            description: Some(long_text("Bandgeschwindigkeit")),
        },
    }
}

#[tokio::test]
async fn upsert_translation_commits_the_new_language_and_the_touch() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;

    UpsertTranslation::new(app.context())
        .execute(&app.user, &app.meta, upsert_input(&world, &param, expected))
        .await
        .unwrap();

    assert_eq!(app.count(TRANSLATIONS, param.id).await, 2);
    assert_touched(&mut app, &world, expected).await;
    app.assert_chain_moved(&before, &["TranslationUpserted"])
        .await;
}

#[tokio::test]
async fn upsert_translation_fails_on_the_translation_insert_and_adds_no_language() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;
    app.install_probe("parameter_translation", "BEFORE", "INSERT", FAULT)
        .await;

    let error = UpsertTranslation::new(app.context())
        .execute(&app.user, &app.meta, upsert_input(&world, &param, expected))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count(TRANSLATIONS, param.id).await, 1);
    assert_untouched(&mut app, &world, expected).await;
    app.assert_chain_unchanged(&before).await;
}

// RemoveTranslation

fn remove_translation_input(world: &World, param: &Param, expected: i64) -> RemoveTranslationInput {
    RemoveTranslationInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: expected,
        key: param.key,
        lang: lang("pl"),
    }
}

#[tokio::test]
async fn remove_translation_deletes_the_language_and_commits_the_touch() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;

    RemoveTranslation::new(app.context())
        .execute(
            &app.user,
            &app.meta,
            remove_translation_input(&world, &param, expected),
        )
        .await
        .unwrap();

    assert_eq!(app.count(TRANSLATIONS, param.id).await, 0);
    assert_touched(&mut app, &world, expected).await;
    app.assert_chain_moved(&before, &["TranslationRemoved"])
        .await;
}

#[tokio::test]
async fn remove_translation_fails_on_the_delete_and_keeps_the_language() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;
    app.install_probe("parameter_translation", "BEFORE", "DELETE", FAULT)
        .await;

    let error = RemoveTranslation::new(app.context())
        .execute(
            &app.user,
            &app.meta,
            remove_translation_input(&world, &param, expected),
        )
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count(TRANSLATIONS, param.id).await, 1);
    assert_untouched(&mut app, &world, expected).await;
    app.assert_chain_unchanged(&before).await;
}

// RemoveParameter

fn remove_parameter_input(world: &World, param: &Param, expected: i64) -> RemoveParameterInput {
    RemoveParameterInput {
        reference: world.reference.clone(),
        number: 1,
        expected_row_version: expected,
        key: param.key,
    }
}

const PARAMETER_BY_ID: &str = "SELECT count(*) FROM parameter WHERE id = $1";
const LIMITS_OF: &str = "SELECT count(*) FROM parameter_limit WHERE parameter_id = $1";

#[tokio::test]
async fn remove_parameter_deletes_the_children_and_keeps_both_arrays_in_the_before_state() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;

    RemoveParameter::new(app.context())
        .execute(
            &app.user,
            &app.meta,
            remove_parameter_input(&world, &param, expected),
        )
        .await
        .unwrap();

    assert_eq!(app.count(PARAMETER_BY_ID, param.id).await, 0);
    assert_eq!(app.count(LIMITS_OF, param.id).await, 0);
    assert_eq!(app.count(TRANSLATIONS, param.id).await, 0);
    let (limit_rows, translation_rows): (i32, i32) = sqlx::query_as(
        "SELECT jsonb_array_length(before_state->'limits'), \
                jsonb_array_length(before_state->'translations') \
         FROM audit_log WHERE action = 'ParameterRemoved'",
    )
    .fetch_one(app.admin())
    .await
    .unwrap();
    assert_eq!((limit_rows, translation_rows), (1, 1));
    assert_touched(&mut app, &world, expected).await;
    app.assert_chain_moved(&before, &["ParameterRemoved"]).await;
}

#[tokio::test]
async fn remove_parameter_fails_on_the_translation_cascade_and_keeps_the_parameter_with_children() {
    let mut app = App::create().await;
    let (world, param) = world_with_speed(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;
    app.install_probe("parameter_translation", "BEFORE", "DELETE", FAULT)
        .await;

    let error = RemoveParameter::new(app.context())
        .execute(
            &app.user,
            &app.meta,
            remove_parameter_input(&world, &param, expected),
        )
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(app.count(PARAMETER_BY_ID, param.id).await, 1);
    assert_eq!(app.count(LIMITS_OF, param.id).await, 1);
    assert_eq!(app.count(TRANSLATIONS, param.id).await, 1);
    assert_untouched(&mut app, &world, expected).await;
    app.assert_chain_unchanged(&before).await;
}

// RebindTypeRevision

/// A draft with a parameter of the struct `ST_Zone` revision 1 and the revision 2 with `m2`.
struct Rebind {
    world: World,
    key: ParameterKey,
    parameter: Uuid,
    from: Uuid,
    to: Uuid,
}

async fn rebind_world(app: &mut App) -> Rebind {
    let world = World::create(app).await;
    let int = fixtures::primitive(app.migrator(), "INT").await;
    let from =
        fixtures::complete_struct(app.migrator(), world.machine, TWIN_CAT, "ST_Zone", int).await;
    let to = revision_two(app, &world, from, int).await;
    let parameter: Uuid = sqlx::query_scalar(
        "INSERT INTO parameter (recipe_version_id, plc_platform, unit_id, tag, name_en, \
             description_en, plc_type_id, value) \
         VALUES ($1, $2, $3, 'ZONE', 'Zone', 'Zone state', $4, '{\"m1\": 1}'::jsonb) RETURNING id",
    )
    .bind(world.version)
    .bind(TWIN_CAT)
    .bind(world.unit)
    .bind(from)
    .fetch_one(app.migrator())
    .await
    .unwrap();
    let key: Uuid = sqlx::query_scalar("SELECT parameter_key FROM parameter WHERE id = $1")
        .bind(parameter)
        .fetch_one(app.migrator())
        .await
        .unwrap();
    Rebind {
        world,
        key: ParameterKey::from(key),
        parameter,
        from,
        to,
    }
}

/// Insert `ST_Zone` revision 2 with the members `m1` and `m2` in one transaction.
async fn revision_two(app: &mut App, world: &World, from: Uuid, int: Uuid) -> Uuid {
    let mut tx = sqlx::Connection::begin(app.migrator()).await.unwrap();
    let to: Uuid = sqlx::query_scalar(
        "INSERT INTO plc_type (kind, name, platforms, machine_id, revision, supersedes_type_id) \
         VALUES ('struct', 'ST_Zone', ARRAY['TwinCat3'], $1, 2, $2) RETURNING id",
    )
    .bind(world.machine)
    .bind(from)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    fixtures::member(&mut tx, to, 1, int).await;
    fixtures::member(&mut tx, to, 2, int).await;
    tx.commit().await.unwrap();
    to
}

fn rebind_input(rebind: &Rebind, expected: i64) -> RebindTypeRevisionInput {
    RebindTypeRevisionInput {
        reference: rebind.world.reference.clone(),
        number: 1,
        expected_row_version: expected,
        from: PlcTypeId::from(rebind.from),
        to: PlcTypeId::from(rebind.to),
        new_member_values: BTreeMap::from([(
            (rebind.key, LeafPath::parse(".m2").unwrap()),
            MemberValue::Json(json!(7)),
        )]),
    }
}

const MOVED: &str = "SELECT count(*) FROM parameter WHERE id = $1 AND plc_type_id = $2 \
                     AND value = '{\"m1\": 1, \"m2\": 7}'::jsonb";

async fn count_moved(app: &mut App, rebind: &Rebind, plc_type: Uuid) -> i64 {
    sqlx::query_scalar(MOVED)
        .bind(rebind.parameter)
        .bind(plc_type)
        .fetch_one(app.admin())
        .await
        .unwrap()
}

#[tokio::test]
async fn rebind_moves_the_parameter_to_revision_two_with_the_new_member_value() {
    let mut app = App::create().await;
    let rebind = rebind_world(&mut app).await;
    let expected = rv(&mut app, &rebind.world).await;
    let before = app.head().await;

    RebindTypeRevision::new(app.context())
        .execute(&app.user, &app.meta, rebind_input(&rebind, expected))
        .await
        .unwrap();

    assert_eq!(count_moved(&mut app, &rebind, rebind.to).await, 1);
    assert_touched(&mut app, &rebind.world, expected).await;
    app.assert_chain_moved(&before, &["TypeRevisionRebound"])
        .await;
}

#[tokio::test]
async fn rebind_fails_on_the_parameter_update_and_keeps_revision_one_and_the_value() {
    let mut app = App::create().await;
    let rebind = rebind_world(&mut app).await;
    let expected = rv(&mut app, &rebind.world).await;
    let before = app.head().await;
    app.install_probe("parameter", "BEFORE", "UPDATE", FAULT)
        .await;

    let error = RebindTypeRevision::new(app.context())
        .execute(&app.user, &app.meta, rebind_input(&rebind, expected))
        .await
        .unwrap_err();

    assert_statement_fault(&error);
    assert_eq!(count_moved(&mut app, &rebind, rebind.to).await, 0);
    let unchanged: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM parameter WHERE id = $1 AND plc_type_id = $2 \
         AND value = '{\"m1\": 1}'::jsonb",
    )
    .bind(rebind.parameter)
    .bind(rebind.from)
    .fetch_one(app.admin())
    .await
    .unwrap();
    assert_eq!(unchanged, 1);
    assert_untouched(&mut app, &rebind.world, expected).await;
    app.assert_chain_unchanged(&before).await;
}
