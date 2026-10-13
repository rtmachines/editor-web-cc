use super::support::*;
use super::*;
use crate::error_codes::{ErrorCode, REGISTRY};

use RecipeDeleter::{Administrator, ProcessEngineer};
use VersionState::{Approved, Discarded, LiveDraft, Retired};

const BOTH: [RecipeDeleter; 2] = [Administrator, ProcessEngineer];

/// Delete and expect the error. The recipe must stay unchanged.
fn assert_delete_fails(
    versions: Vec<StoredVersionHeader>,
    deleter: RecipeDeleter,
    code: ErrorCode,
    message: &str,
) {
    let mut recipe = recipe(versions);
    let before = recipe.clone();
    let error = recipe.delete(deleter, ts(50)).unwrap_err();
    assert_eq!(error.code(), code, "{deleter:?}");
    assert_eq!(error.to_string(), message, "{deleter:?}");
    assert_eq!(recipe, before, "{deleter:?} changed the recipe");
}

#[test]
fn recipe_with_only_a_live_draft_is_deleted_by_both_roles() {
    for deleter in BOTH {
        let mut recipe = recipe(vec![header(1, LiveDraft, 4)]);
        let discarded = recipe.delete(deleter, ts(50)).unwrap();
        assert_eq!(discarded, Some(vid(1)), "{deleter:?}");
        assert_eq!(recipe.deleted_at(), Some(ts(50)));
        assert!(recipe.is_deleted());
        assert_eq!(states(&recipe), vec![(1, Discarded, 4)]);
    }
}

#[test]
fn recipe_with_only_discarded_drafts_is_deleted_without_a_discard() {
    for deleter in BOTH {
        let mut recipe = recipe(vec![header(1, Discarded, 2), header(2, Discarded, 2)]);
        assert_eq!(recipe.delete(deleter, ts(50)).unwrap(), None, "{deleter:?}");
        assert_eq!(recipe.deleted_at(), Some(ts(50)));
        assert_eq!(states(&recipe), vec![(1, Discarded, 2), (2, Discarded, 2)]);
    }
}

#[test]
fn administrator_deletes_a_recipe_with_a_retired_version_and_discards_the_draft() {
    let mut recipe = recipe(vec![header(1, Retired, 3), header(2, LiveDraft, 1)]);
    let discarded = recipe.delete(Administrator, ts(50)).unwrap();
    assert_eq!(discarded, Some(vid(2)));
    assert!(recipe.is_deleted());
    assert_eq!(states(&recipe), vec![(1, Retired, 3), (2, Discarded, 1)]);
}

#[test]
fn administrator_deletes_a_recipe_with_only_a_retired_version() {
    let mut recipe = recipe(vec![header(1, Retired, 3)]);
    assert_eq!(recipe.delete(Administrator, ts(50)).unwrap(), None);
    assert!(recipe.is_deleted());
}

#[test]
fn process_engineer_cannot_delete_a_recipe_with_a_retired_version() {
    assert_delete_fails(
        vec![header(1, Retired, 3), header(2, LiveDraft, 1)],
        ProcessEngineer,
        ErrorCode::RCP_026,
        "Recipe R-1 has a retired version. Only an administrator can delete it.",
    );
}

#[test]
fn nobody_can_delete_a_recipe_with_an_approved_version() {
    for deleter in BOTH {
        assert_delete_fails(
            vec![header(1, Approved, 3), header(2, LiveDraft, 1)],
            deleter,
            ErrorCode::RCP_010,
            "Recipe R-1 has an approved version. An Administrator can retire the version.",
        );
    }
}

#[test]
fn approved_version_wins_over_a_retired_version_for_both_roles() {
    for deleter in BOTH {
        assert_delete_fails(
            vec![header(1, Retired, 3), header(2, Approved, 3)],
            deleter,
            ErrorCode::RCP_010,
            "Recipe R-1 has an approved version. An Administrator can retire the version.",
        );
    }
}

#[test]
fn deleting_a_deleted_recipe_gives_recipe_deleted_and_changes_nothing() {
    for deleter in BOTH {
        let mut recipe = deleted_recipe(vec![header(1, Retired, 3)]);
        let before = recipe.clone();
        let error = recipe.delete(deleter, ts(60)).unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_012);
        assert_eq!(recipe, before);
    }
}

#[test]
fn deleted_recipe_check_comes_before_the_approved_check() {
    let mut recipe = deleted_unchecked(vec![header(1, Approved, 3)]);
    let error = recipe.delete(Administrator, ts(60)).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
}

#[test]
fn restore_clears_the_deletion_mark_and_keeps_the_discarded_draft() {
    let mut recipe = recipe(vec![header(1, LiveDraft, 1)]);
    recipe.delete(Administrator, ts(50)).unwrap();
    recipe.restore().unwrap();
    assert_eq!(recipe.deleted_at(), None);
    assert!(!recipe.is_deleted());
    assert_eq!(states(&recipe), vec![(1, Discarded, 1)]);
}

#[test]
fn restore_of_a_recipe_that_is_not_deleted_gives_recipe_not_deleted() {
    let mut recipe = recipe(vec![header(1, Approved, 2)]);
    let before = recipe.clone();
    let error = recipe.restore().unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_025);
    assert_eq!(error.to_string(), "Recipe R-1 is not deleted.");
    assert_eq!(recipe, before);
}

#[test]
fn delete_errors_have_registry_rows() {
    for code in [ErrorCode::RCP_010, ErrorCode::RCP_025, ErrorCode::RCP_026] {
        assert!(REGISTRY.contains(&code), "{code}");
    }
}
