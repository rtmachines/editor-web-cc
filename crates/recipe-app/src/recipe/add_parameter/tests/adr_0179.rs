//! Tests of `AddParameter` for the rule "nesting limit of a type on S7" (ADR-0179).

use recipe_domain::plant::{PlantError, PlcPlatform, S7Address, TypeLimit, UnitPath};
use serde_json::{Value, json};

use super::{AddParameterInput, assert_nothing_written, empty_draft, input, run};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, seed_struct_chain, struct_chain_value};
use crate::memory::{DERIVED_TYPE_LOCK_TABLE, JournalEntry};
use crate::recipe::test_support::stored_parameters;

const S7_PLATFORMS: [PlcPlatform; 3] = [
    PlcPlatform::S7_300,
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
];

/// A request for the tag `FEED` with the type text and the value.
///
/// Only `S7_300` needs an S7 address (A31), so the request has one for that platform.
fn request(platform: PlcPlatform, type_text: &str, value: &Value) -> AddParameterInput {
    AddParameterInput {
        s7_address: (platform == PlcPlatform::S7_300)
            .then(|| S7Address::parse("DB1.DBB0").unwrap()),
        ..input(type_text, &value.to_string(), Vec::new())
    }
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
async fn adr_0179_a_struct_with_nine_levels_on_s7_gives_plt_018_and_adds_nothing() {
    for platform in S7_PLATFORMS {
        let fixture = empty_draft(platform);
        seed_struct_chain(&fixture, platform, ("ST_Top", 9));
        let error = run(
            &fixture,
            request(platform, "ST_Top@1", &struct_chain_value(9)),
        )
        .await
        .unwrap_err();
        assert_nesting_refused(&error, "ST_Top@1");
        assert!(stored_parameters(&fixture).is_empty(), "{platform:?}");
        assert_nothing_written(&fixture);
    }
}

#[tokio::test]
async fn adr_0179_an_array_of_a_struct_counts_three_levels_and_gives_plt_018() {
    let fixture = empty_draft(PlcPlatform::S7_1500);
    seed_struct_chain(&fixture, PlcPlatform::S7_1500, ("ST_Mid", 7));
    let value = json!([struct_chain_value(7), struct_chain_value(7)]);
    let error = run(
        &fixture,
        request(PlcPlatform::S7_1500, "ARRAY[1..2] OF ST_Mid@1", &value),
    )
    .await
    .unwrap_err();
    assert_nesting_refused(&error, "ARRAY[1..2] OF ST_Mid@1");
    assert!(stored_parameters(&fixture).is_empty());
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn adr_0179_an_array_of_a_struct_with_eight_levels_in_the_count_succeeds() {
    let fixture = empty_draft(PlcPlatform::S7_1500);
    seed_struct_chain(&fixture, PlcPlatform::S7_1500, ("ST_Mid", 6));
    let value = json!([struct_chain_value(6), struct_chain_value(6)]);
    run(
        &fixture,
        request(PlcPlatform::S7_1500, "ARRAY[1..2] OF ST_Mid@1", &value),
    )
    .await
    .unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}

#[tokio::test]
async fn adr_0179_a_struct_with_eight_levels_on_s7_succeeds() {
    let fixture = empty_draft(PlcPlatform::S7_1500);
    seed_struct_chain(&fixture, PlcPlatform::S7_1500, ("ST_Top", 8));
    run(
        &fixture,
        request(PlcPlatform::S7_1500, "ST_Top@1", &struct_chain_value(8)),
    )
    .await
    .unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}

#[tokio::test]
async fn adr_0179_a_struct_with_nine_levels_on_twincat_succeeds() {
    let fixture = empty_draft(PlcPlatform::TwinCat3);
    seed_struct_chain(&fixture, PlcPlatform::TwinCat3, ("ST_Top", 9));
    run(
        &fixture,
        request(PlcPlatform::TwinCat3, "ST_Top@1", &struct_chain_value(9)),
    )
    .await
    .unwrap();
    assert_eq!(stored_parameters(&fixture).len(), 1);
}

/// The tables of the locks that the command took, in order.
fn locked_tables(fixture: &Fixture) -> Vec<&'static str> {
    fixture
        .store
        .journal()
        .into_iter()
        .filter_map(|line| match line {
            JournalEntry::Lock(lock) => Some(lock.table),
            JournalEntry::Begin(_) => None,
        })
        .collect()
}

#[tokio::test]
async fn adr_0179_a_type_over_the_limit_wins_over_a_missing_unit_and_takes_no_unit_or_type_lock() {
    for type_text in ["ST_Top@1", "ARRAY[1..2] OF ST_Top@1"] {
        let fixture = empty_draft(PlcPlatform::S7_1500);
        seed_struct_chain(&fixture, PlcPlatform::S7_1500, ("ST_Top", 9));
        let mut request = request(PlcPlatform::S7_1500, type_text, &struct_chain_value(9));
        request.unit_path = UnitPath::parse("U9").unwrap();
        let error = run(&fixture, request).await.unwrap_err();
        assert_nesting_refused(&error, type_text);
        let tables = locked_tables(&fixture);
        assert!(!tables.contains(&"unit"), "{type_text}: {tables:?}");
        assert!(
            !tables.contains(&DERIVED_TYPE_LOCK_TABLE),
            "{type_text}: {tables:?}"
        );
        assert_nothing_written(&fixture);
    }
}
