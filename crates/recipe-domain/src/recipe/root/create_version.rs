use super::support::*;
use super::*;
use crate::error_codes::{ErrorCode, REGISTRY};

use VersionState::{Approved, Discarded, LiveDraft, Retired};

fn largest() -> u32 {
    u32::try_from(i32::MAX).unwrap()
}

#[test]
fn live_draft_blocks_a_new_version_with_its_number() {
    let mut recipe = recipe(vec![header(1, Approved, 2), header(2, LiveDraft, 1)]);
    let before = recipe.clone();
    let error = recipe.create_version(vid(50)).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_011);
    assert_eq!(
        error.to_string(),
        "Recipe R-1 already has draft version 2. Edit or discard that draft."
    );
    assert_eq!(recipe, before);
}

#[test]
fn next_number_counts_approved_retired_and_discarded_versions() {
    let mut recipe = recipe(vec![
        header(1, Approved, 2),
        header(2, Retired, 3),
        header(3, Discarded, 2),
    ]);
    let created = recipe.create_version(vid(50)).unwrap();
    assert_eq!(created.number(), 4);
    assert_eq!(created.id(), vid(50));
    assert_eq!(created.state(), LiveDraft);
    assert_eq!(created.row_version(), 1);
    assert_eq!(
        states(&recipe),
        vec![
            (1, Approved, 2),
            (2, Retired, 3),
            (3, Discarded, 2),
            (4, LiveDraft, 1)
        ]
    );
}

/// A recipe whose headers have exactly the given numbers.
///
/// `TryFrom<StoredRecipe>` rejects a gap or a number near the SQL range, so the tests of the
/// defensive overflow rule build the value directly.
fn recipe_with_numbers(numbers: &[u32]) -> Recipe {
    let versions = numbers
        .iter()
        .map(|number| VersionHeader {
            id: vid(u128::from(*number)),
            number: *number,
            state: Retired,
            row_version: 3,
        })
        .collect();
    Recipe {
        id: rid(1),
        machine: machine_id(),
        reference: reference(),
        name: name("Recipe"),
        deleted_at: None,
        versions,
    }
}

#[test]
fn next_number_follows_the_largest_number_not_the_count() {
    let mut recipe = recipe_with_numbers(&[2, 5]);
    let created = recipe.create_version(vid(50)).unwrap();
    assert_eq!(created.number(), 6);
}

#[test]
fn recipe_without_a_header_gets_number_one() {
    let mut recipe = recipe(Vec::new());
    assert_eq!(recipe.create_version(vid(50)).unwrap().number(), 1);
}

#[test]
fn largest_sql_integer_number_is_still_allowed() {
    let below = largest().checked_sub(1).unwrap();
    let mut recipe = recipe_with_numbers(&[below]);
    assert_eq!(recipe.create_version(vid(50)).unwrap().number(), largest());
}

#[test]
fn number_above_the_sql_integer_range_gives_size_budget_exceeded_and_no_panic() {
    let mut recipe = recipe_with_numbers(&[largest()]);
    let before = recipe.clone();
    let error = recipe.create_version(vid(50)).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_021);
    assert_eq!(
        error.to_string(),
        "The version exceeds the limit of 2147483647 version numbers. Current count: 2147483647."
    );
    assert_eq!(recipe, before);
}

#[test]
fn deleted_recipe_gives_recipe_deleted_before_the_draft_check() {
    let mut recipe = deleted_unchecked(vec![header(1, LiveDraft, 1)]);
    let before = recipe.clone();
    let error = recipe.create_version(vid(50)).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert_eq!(recipe, before);
}

#[test]
fn budget_words_name_the_counted_thing() {
    assert_eq!(Budget::Parameters.to_string(), "parameters");
    assert_eq!(Budget::Leaves.to_string(), "leaves");
    assert_eq!(Budget::LimitRows.to_string(), "limit rows");
    assert_eq!(Budget::VersionNumbers.to_string(), "version numbers");
}

#[test]
fn size_budget_exceeded_shows_budget_limit_and_actual() {
    let error = RecipeError::SizeBudgetExceeded {
        budget: Budget::Parameters,
        limit: 4_000,
        actual: 4_001,
    };
    assert_eq!(error.code(), ErrorCode::RCP_021);
    assert_eq!(
        error.to_string(),
        "The version exceeds the limit of 4000 parameters. Current count: 4001."
    );
}

#[test]
fn create_version_errors_have_registry_rows() {
    for code in [ErrorCode::RCP_011, ErrorCode::RCP_021] {
        assert!(REGISTRY.contains(&code), "{code}");
    }
}
