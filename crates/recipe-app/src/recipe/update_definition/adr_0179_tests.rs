//! Tests of `UpdateParameterDefinition` for the rule "nesting limit of a type on S7" (ADR-0179).

use recipe_domain::plant::{PlantError, PlcPlatform, S7Address, TypeLimit};
use serde_json::Value;

use super::UpdateParameterDefinitionInput;
use super::requests::{run, speed_request, with_retype, with_type};
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, over_limit_top_parameter, parameter_on, seed_struct_chain, struct_chain_value, text,
    uid, vid,
};
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot, speed_key, stored_parameters};

/// A draft of a machine on `platform` with the `INT` parameter `SPEED` and no S7 address.
fn draft_with_speed(platform: PlcPlatform) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let speed = parameter_on(platform, 1, "SPEED", uid(101), 0);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    fixture
}

/// A request that changes `SPEED` to the type text with the value.
///
/// Only `S7_300` needs an S7 address (A31), so the request has one for that platform.
fn retype_request(
    platform: PlcPlatform,
    type_text: &str,
    value: &Value,
) -> UpdateParameterDefinitionInput {
    let request = UpdateParameterDefinitionInput {
        s7_address: (platform == PlcPlatform::S7_300)
            .then(|| S7Address::parse("DB1.DBB0").unwrap()),
        ..with_type(speed_request(), type_text)
    };
    with_retype(request, &value.to_string(), Vec::new())
}

/// Assert that the error is `E-PLT-018` with the S7 nesting limit for `type_text`.
fn assert_nesting_refused(error: &AppError, type_text: &str) {
    assert!(
        matches!(
            error,
            AppError::Plant(PlantError::TypeTooLarge { type_name, limit: TypeLimit::S7Nesting })
                if type_name == type_text
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn adr_0179_a_changed_type_with_nine_levels_on_s7_gives_plt_018_and_changes_nothing() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        let fixture = draft_with_speed(platform);
        seed_struct_chain(&fixture, platform, ("ST_Top", 9));
        let before = DraftSnapshot::capture(&fixture);
        let error = run(
            &fixture,
            retype_request(platform, "ST_Top@1", &struct_chain_value(9)),
        )
        .await
        .unwrap_err();
        assert_nesting_refused(&error, "ST_Top@1");
        before.assert_unchanged(&fixture);
    }
}

#[tokio::test]
async fn adr_0179_a_changed_type_with_eight_levels_on_s7_succeeds() {
    let fixture = draft_with_speed(PlcPlatform::S7_1500);
    seed_struct_chain(&fixture, PlcPlatform::S7_1500, ("ST_Top", 8));
    run(
        &fixture,
        retype_request(PlcPlatform::S7_1500, "ST_Top@1", &struct_chain_value(8)),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn adr_0179_a_changed_type_with_nine_levels_on_twincat_succeeds() {
    let fixture = draft_with_speed(PlcPlatform::TwinCat3);
    seed_struct_chain(&fixture, PlcPlatform::TwinCat3, ("ST_Top", 9));
    run(
        &fixture,
        retype_request(PlcPlatform::TwinCat3, "ST_Top@1", &struct_chain_value(9)),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn adr_0179_an_unchanged_stored_type_over_the_limit_does_not_stop_a_rename() {
    let fixture = draft_with_speed(PlcPlatform::S7_1500);
    let top = seed_struct_chain(&fixture, PlcPlatform::S7_1500, ("ST_Top", 9));
    let stored = over_limit_top_parameter(top);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &stored);
    let request = UpdateParameterDefinitionInput {
        key: stored.key,
        tag: stored.tag.clone(),
        type_expr: recipe_domain::plant::TypeExpr::parse("ST_Top@1").unwrap(),
        ordinal: stored.ordinal,
        name_en: text("Renamed top"),
        ..speed_request()
    };
    run(&fixture, request).await.unwrap();
    let kept = stored_parameters(&fixture)
        .into_iter()
        .find(|row| row.key == stored.key)
        .unwrap();
    assert_eq!(kept.plc_type, top);
    assert_ne!(kept.key, speed_key());
}
