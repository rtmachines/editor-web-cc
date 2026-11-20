//! Tests of `SetParameterValue` and `ReplaceLimits` for the rule "Long prefixes of IEC literals"
//! (ADR-0205 (a) to (d), (g), (h)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{PlcPlatform, Primitive};
use recipe_domain::recipe::{
    LimitInput, LimitList, ParameterKey, StoredLimit, StoredParameter, StoredValue, ValueInput,
};
use serde_json::{Value, json};

use super::{SetParameterValue, SetParameterValueInput};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, actor, meta, parameter_on, reference, uid, vid};
use crate::memory::seed_type_id;
use crate::recipe::replace_limits::{ReplaceLimits, ReplaceLimitsInput};
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot, stored_parameters};
use crate::recipe::{ListParameters, ListParametersInput};

/// The identifier of the global row that [`draft_with`] seeds for a primitive without a seed row.
const EXTRA_TYPE: u128 = 990;

/// The key of the parameter that [`draft_with`] stores.
fn key() -> ParameterKey {
    crate::memory::fixtures::pkey(5)
}

/// Store a draft `R-1` of the machine `M1` on `platform` with the parameter `TARGET` of type
/// `primitive` and the stored value `initial`.
///
/// `LTIME` and `TIME` use the seed row. Every other primitive gets a global row.
fn draft_with(platform: PlcPlatform, primitive: Primitive, initial: Value) -> Fixture {
    let fixture = Fixture::new();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let plc_type = seed_type_id(primitive, None)
        .filter(|_| matches!(primitive, Primitive::Time | Primitive::Ltime))
        .unwrap_or_else(|| fixture.seed_primitive_type(EXTRA_TYPE, platform, primitive));
    let parameter = StoredParameter {
        plc_type,
        value: StoredValue::from(initial),
        limits: Vec::new(),
        ..parameter_on(platform, 5, "TARGET", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &parameter);
    fixture
}

fn value_request(text: &str) -> SetParameterValueInput {
    SetParameterValueInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: key(),
        value: ValueInput::Json(json!(text).to_string()),
    }
}

async fn set(fixture: &Fixture, text: &str) -> Result<(), AppError> {
    SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), value_request(text))
        .await
        .map(|_| ())
}

async fn replace(fixture: &Fixture, min: Option<&str>, max: Option<&str>) -> Result<(), AppError> {
    let row = LimitInput {
        leaf_path: recipe_domain::plant::LeafPath::parse("").unwrap(),
        min: min.map(str::to_owned),
        max: max.map(str::to_owned),
        max_length: None,
    };
    let input = ReplaceLimitsInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: key(),
        limits: LimitList::parse(vec![row]).unwrap(),
    };
    ReplaceLimits::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input)
        .await
        .map(|_| ())
}

fn target(fixture: &Fixture) -> StoredParameter {
    stored_parameters(fixture)
        .into_iter()
        .find(|row| row.key == key())
        .unwrap()
}

fn stored_value(fixture: &Fixture) -> Value {
    target(fixture).value.as_json().clone()
}

fn stored_limits(fixture: &Fixture) -> Vec<StoredLimit> {
    target(fixture).limits
}

/// The text that `ListParameters` shows for the value of the parameter.
async fn shown_text(fixture: &Fixture) -> String {
    let list = ListParameters::new(fixture.context())
        .execute(
            &actor(Role::Operator),
            &meta(),
            ListParametersInput {
                reference: reference("R-1"),
                number: 1,
                unit: None,
                page: 0,
                lang: None,
            },
        )
        .await
        .unwrap();
    list.value_texts.get(&key()).cloned().unwrap()
}

#[tokio::test]
async fn adr_0205_an_ltime_value_with_the_long_prefix_stores_the_count_of_the_short_prefix() {
    let long = draft_with(PlcPlatform::TwinCat3, Primitive::Ltime, json!(0));
    let short = draft_with(PlcPlatform::TwinCat3, Primitive::Ltime, json!(0));

    set(&long, "LTIME#1h").await.unwrap();
    set(&short, "LT#1h").await.unwrap();

    assert_eq!(stored_value(&long), stored_value(&short));
    assert_ne!(stored_value(&long), json!(0));
    assert_eq!(shown_text(&long).await, "LT#1h");
}

#[tokio::test]
async fn adr_0205_a_time_of_day_value_with_the_long_prefix_is_stored_on_s7() {
    let long = draft_with(PlcPlatform::S7_1500, Primitive::TimeOfDay, json!(0));
    let short = draft_with(PlcPlatform::S7_1500, Primitive::TimeOfDay, json!(0));

    set(&long, "TIME_OF_DAY#06:00:00").await.unwrap();
    set(&short, "TOD#06:00:00").await.unwrap();

    assert_eq!(stored_value(&long), stored_value(&short));
    assert_ne!(stored_value(&long), json!(0));
    assert_eq!(shown_text(&long).await, "TOD#06:00:00");
}

#[tokio::test]
async fn adr_0205_a_long_prefix_in_lower_case_gives_rcp_005() {
    let fixture = draft_with(PlcPlatform::TwinCat3, Primitive::Ltime, json!(0));
    let before = DraftSnapshot::capture(&fixture);

    let error = set(&fixture, "ltime#1us").await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_005);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn adr_0205_s5time_has_no_long_prefix() {
    let fixture = draft_with(PlcPlatform::S7_300, Primitive::S5Time, json!(0));
    let before = DraftSnapshot::capture(&fixture);

    let error = set(&fixture, "S5TIME#1s").await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_005);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn adr_0205_a_limit_with_the_long_prefix_stores_the_count_of_the_short_prefix() {
    let long = draft_with(PlcPlatform::TwinCat3, Primitive::Time, json!(5000));
    let short = draft_with(PlcPlatform::TwinCat3, Primitive::Time, json!(5000));

    replace(&long, Some("TIME#1s"), None).await.unwrap();
    replace(&short, Some("T#1s"), None).await.unwrap();

    let limits = stored_limits(&long);
    assert_eq!(limits.len(), 1);
    assert_eq!(limits, stored_limits(&short));
    assert_eq!(
        limits.first().unwrap().min,
        Some("1000".parse().unwrap()),
        "one second is 1000 ms"
    );
}

#[tokio::test]
async fn adr_0205_a_limit_with_a_lower_case_long_prefix_gives_rcp_006() {
    let fixture = draft_with(PlcPlatform::TwinCat3, Primitive::Time, json!(0));
    let before = DraftSnapshot::capture(&fixture);

    let error = replace(&fixture, Some("time#1s"), None).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_006);
    before.assert_unchanged(&fixture);
}
