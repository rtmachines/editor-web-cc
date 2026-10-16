//! Tests that the seeded draft of `R-100` meets the invariants of the domain (task 8.3).
//!
//! A command of the recipe pages checks the S7 layout of the whole draft. So a seeded address
//! with a wrong width or an overlap would fail every command of waves 4 and 5. The test sends one
//! `SetParameterValue` for each seeded parameter through the app of the fixture.

use recipe_app::auth::RequestMeta;
use recipe_app::identity::authenticate;
use recipe_app::recipe::{SetParameterValue, SetParameterValueInput};
use recipe_domain::audit::RequestId;
use recipe_domain::identity::{Role, SessionToken};
use recipe_domain::recipe::{ParameterKey, ReferenceNumber, ValueInput};
use uuid::Uuid;

use super::{GRID_KEY, HOLD_KEY, LABEL_KEY, LINE_A_KEY, SPEED_KEY, SPEED_TABLE_KEY, ZONES_KEY};
use crate::test_support::Fixture;

/// The new value of each seeded parameter, as JSON text. Each value fits its type and its limit.
const NEW_VALUES: [(Uuid, &str, &str); 7] = [
    (SPEED_KEY, "SPEED", "1300"),
    (LABEL_KEY, "LABEL", "\"LINE B\""),
    (HOLD_KEY, "HOLD", "\"T#2m\""),
    (SPEED_TABLE_KEY, "SPEED_TABLE", "[1,2,3,4,5,6,7,8]"),
    (GRID_KEY, "GRID", "[[1.5,1.5],[2.5,3.5]]"),
    (
        LINE_A_KEY,
        "LINE_A",
        r#"{"head":{"a":2,"b":3.5},"count":4}"#,
    ),
    (
        ZONES_KEY,
        "ZONES",
        r#"[{"speed":1,"temp_max":0},{"speed":0,"temp_max":0},{"speed":0,"temp_max":0},{"speed":0,"temp_max":0}]"#,
    ),
];

#[tokio::test]
async fn seed_recipes_each_seeded_parameter_accepts_a_value_command_on_the_draft() {
    let fixture = Fixture::new();
    fixture.seed_recipes();
    let _admin = fixture.seed_session(1, Role::Administrator, false);
    let token = SessionToken::from_bytes([1; 16]);
    let auth = authenticate(fixture.state.identity(), &token)
        .await
        .unwrap()
        .unwrap();
    let meta = RequestMeta::new(RequestId::from(Uuid::nil()), None, None, None);
    let command = SetParameterValue::new(fixture.state.identity().app().clone());
    let mut row_version = 1;
    for (key, tag, json) in NEW_VALUES {
        let input = SetParameterValueInput {
            reference: ReferenceNumber::parse("R-100").unwrap(),
            number: 3,
            expected_row_version: row_version,
            key: ParameterKey::from(key),
            value: ValueInput::Json(json.to_owned()),
        };
        let result = command.execute(auth.user(), &meta, input).await;
        assert!(result.is_ok(), "{tag}: {result:?}");
        row_version = result.unwrap().row_version;
    }
}
