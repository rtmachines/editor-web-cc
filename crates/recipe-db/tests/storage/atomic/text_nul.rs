//! Pinning tests of the U+0000 rule on PostgreSQL (ADR-0137, design D6).
//!
//! PostgreSQL refuses U+0000 in JSONB with SQLSTATE `22P05`. The domain refuses it first and
//! returns `E-RCP-005`. A refused command leaves the row version, the editor rows, and the
//! parameter rows as they were.

use recipe_app::recipe::{
    AddParameter, AddParameterInput, SetParameterValue, SetParameterValueInput,
};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::TypeExpr;
use recipe_domain::recipe::ValueInput;
use uuid::Uuid;

use super::draft::{PARAMETER_ROWS, add_input, assert_untouched, rv, set_value_input};
use super::world::{Param, World};
use crate::support::app::App;
use crate::support::fixtures::{self, TWIN_CAT};

/// A JSON string whose text holds U+0000 between two letters.
const NUL_TEXT: &str = r#""a\u0000b""#;

const STRING_10_ROWS: &str = "SELECT count(*) FROM plc_type WHERE machine_id = $1 \
                              AND name = 'STRING' AND string_length = 10";
const VALUE_TEXT: &str = "SELECT value::text FROM parameter WHERE id = $1";
const STORED_TEXT: &str = "\"abc\"";

/// Insert a `STRING(10)` parameter that holds the text `abc`.
async fn string_parameter(app: &mut App, world: &World) -> Param {
    let string = fixtures::machine_string(app.migrator(), world.machine, TWIN_CAT, 10).await;
    let id = fixtures::parameter(
        app.migrator(),
        world.version,
        TWIN_CAT,
        world.unit,
        string,
        "LABEL",
    )
    .await;
    sqlx::query("UPDATE parameter SET value = $2::jsonb WHERE id = $1")
        .bind(id)
        .bind(STORED_TEXT)
        .execute(app.migrator())
        .await
        .unwrap();
    let key: Uuid = sqlx::query_scalar("SELECT parameter_key FROM parameter WHERE id = $1")
        .bind(id)
        .fetch_one(app.migrator())
        .await
        .unwrap();
    Param {
        id,
        key: key.into(),
    }
}

async fn stored_text(app: &mut App, parameter: Uuid) -> String {
    sqlx::query_scalar(VALUE_TEXT)
        .bind(parameter)
        .fetch_one(app.admin())
        .await
        .unwrap()
}

#[tokio::test]
async fn set_value_with_a_nul_in_a_string_returns_value_shape_mismatch_and_changes_nothing() {
    let mut app = App::create().await;
    let world = World::create(&mut app).await;
    let param = string_parameter(&mut app, &world).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;
    let input = SetParameterValueInput {
        value: ValueInput::Json(NUL_TEXT.to_owned()),
        ..set_value_input(&world, &param, expected)
    };

    let error = SetParameterValue::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_005, "{error:?}");
    assert_eq!(stored_text(&mut app, param.id).await, STORED_TEXT);
    assert_eq!(app.count(PARAMETER_ROWS, world.version).await, 1);
    assert_untouched(&mut app, &world, expected).await;
    app.assert_chain_unchanged(&before).await;
}

#[tokio::test]
async fn add_parameter_with_a_nul_in_a_string_returns_value_shape_mismatch_and_leaves_no_type_row()
{
    let mut app = App::create().await;
    let (world, _) = World::with_parameter(&mut app).await;
    let expected = rv(&mut app, &world).await;
    let before = app.head().await;
    let input = AddParameterInput {
        type_expr: TypeExpr::parse("STRING(10)").unwrap(),
        value: ValueInput::Json(NUL_TEXT.to_owned()),
        ..add_input(&world, expected)
    };

    let error = AddParameter::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_005, "{error:?}");
    assert_eq!(app.count(PARAMETER_ROWS, world.version).await, 1);
    assert_eq!(app.count(STRING_10_ROWS, world.machine).await, 0);
    assert_untouched(&mut app, &world, expected).await;
    app.assert_chain_unchanged(&before).await;
}
