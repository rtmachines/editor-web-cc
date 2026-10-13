use super::support::*;
use super::*;
use crate::error_codes::{ErrorCode, REGISTRY};

use VersionState::{Approved, Discarded, LiveDraft, Retired};

/// The recipe of the tests. Version `n` has the row version 7.
fn recipe_with(states: &[(u32, VersionState)]) -> Recipe {
    recipe(states.iter().map(|&(n, s)| header(n, s, 7)).collect())
}

fn state_of(recipe: &Recipe, number: u32) -> VersionState {
    recipe.version(number).unwrap().state()
}

#[test]
fn retire_moves_an_approved_version_to_retired() {
    let mut recipe = recipe_with(&[(1, Approved), (2, LiveDraft)]);
    let outcome = recipe.retire(1, 7).unwrap();
    assert_eq!(outcome, Optimistic::Applied(vid(1)));
    assert_eq!(state_of(&recipe, 1), Retired);
    assert_eq!(state_of(&recipe, 2), LiveDraft);
}

#[test]
fn retire_of_a_version_that_is_not_approved_gives_version_not_approved() {
    for (state, name) in [
        (LiveDraft, "draft"),
        (Retired, "retired"),
        (Discarded, "discarded"),
    ] {
        let mut recipe = recipe_with(&[(1, Retired), (2, state)]);
        let before = recipe.clone();
        let error = recipe.retire(2, 7).unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_009, "{name}");
        assert_eq!(
            error.to_string(),
            "Version 2 of recipe R-1 is not approved.",
            "{name}"
        );
        assert_eq!(recipe, before, "{name}");
    }
}

#[test]
fn retire_of_an_unknown_number_gives_version_not_approved_with_that_number() {
    for number in [0, 9] {
        let mut recipe = recipe_with(&[(1, Approved)]);
        let before = recipe.clone();
        let error = recipe.retire(number, 7).unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_009);
        assert_eq!(
            error.to_string(),
            format!("Version {number} of recipe R-1 is not approved.")
        );
        assert_eq!(recipe, before);
    }
}

#[test]
fn retire_with_an_old_row_version_gives_conflict_and_changes_nothing() {
    let mut recipe = recipe_with(&[(1, Approved)]);
    let before = recipe.clone();
    let outcome = recipe.retire(1, 6).unwrap();
    assert_eq!(
        outcome,
        Optimistic::Conflict {
            expected: 6,
            actual: 7
        }
    );
    assert_eq!(recipe, before);
}

#[test]
fn retire_checks_the_row_version_before_the_transition() {
    let mut recipe = recipe_with(&[(1, Retired)]);
    let before = recipe.clone();
    let outcome = recipe.retire(1, 3).unwrap();
    assert_eq!(
        outcome,
        Optimistic::Conflict {
            expected: 3,
            actual: 7
        }
    );
    assert_eq!(recipe, before);
}

#[test]
fn retire_of_a_deleted_recipe_gives_recipe_deleted_before_every_other_check() {
    for number in [1, 9] {
        let mut recipe = deleted_recipe(vec![header(1, Retired, 7)]);
        let before = recipe.clone();
        let error = recipe.retire(number, 3).unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_012);
        assert_eq!(recipe, before);
    }
}

#[test]
fn discard_moves_a_live_draft_to_discarded() {
    let mut recipe = recipe_with(&[(1, Approved), (2, LiveDraft)]);
    let outcome = recipe.discard(2, 7).unwrap();
    assert_eq!(outcome, Optimistic::Applied(vid(2)));
    assert_eq!(state_of(&recipe, 2), Discarded);
    assert_eq!(state_of(&recipe, 1), Approved);
}

#[test]
fn discard_of_a_version_that_is_not_a_live_draft_gives_version_not_draft() {
    for (state, name) in [
        (Approved, "approved"),
        (Retired, "retired"),
        (Discarded, "discarded"),
    ] {
        let mut recipe = recipe_with(&[(1, state)]);
        let before = recipe.clone();
        let error = recipe.discard(1, 7).unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_007, "{name}");
        assert_eq!(
            error.to_string(),
            "Version 1 of recipe R-1 is not a draft.",
            "{name}"
        );
        assert_eq!(recipe, before, "{name}");
    }
}

#[test]
fn discard_of_an_unknown_number_gives_version_not_draft_with_that_number() {
    let mut recipe = recipe_with(&[(1, LiveDraft)]);
    let before = recipe.clone();
    let error = recipe.discard(4, 7).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_007);
    assert_eq!(error.to_string(), "Version 4 of recipe R-1 is not a draft.");
    assert_eq!(recipe, before);
}

#[test]
fn discard_with_an_old_row_version_gives_conflict_also_for_a_forbidden_state() {
    for state in [LiveDraft, Approved, Discarded] {
        let mut recipe = recipe_with(&[(1, state)]);
        let before = recipe.clone();
        let outcome = recipe.discard(1, 2).unwrap();
        assert_eq!(
            outcome,
            Optimistic::Conflict {
                expected: 2,
                actual: 7
            },
            "{state:?}"
        );
        assert_eq!(recipe, before, "{state:?}");
    }
}

#[test]
fn discard_of_a_deleted_recipe_gives_recipe_deleted() {
    let mut recipe = deleted_recipe(vec![header(1, Discarded, 7)]);
    let before = recipe.clone();
    let error = recipe.discard(1, 7).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert_eq!(recipe, before);
}

#[test]
fn discarded_draft_cannot_be_discarded_again() {
    let mut recipe = recipe_with(&[(1, LiveDraft)]);
    recipe.discard(1, 7).unwrap();
    let error = recipe.discard(1, 7).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_007);
}

#[test]
fn transition_errors_have_registry_rows() {
    for code in [ErrorCode::RCP_007, ErrorCode::RCP_009] {
        assert!(REGISTRY.contains(&code), "{code}");
    }
}
