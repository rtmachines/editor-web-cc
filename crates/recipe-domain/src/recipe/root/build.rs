use super::support::*;
use super::*;
use crate::error_codes::ErrorCode;

#[test]
fn create_makes_one_live_draft_with_number_one_and_row_version_one() {
    let recipe = Recipe::create(rid(1), machine_id(), reference(), name("Cap"), vid(11));
    assert_eq!(recipe.id(), rid(1));
    assert_eq!(recipe.machine(), machine_id());
    assert_eq!(recipe.reference(), &reference());
    assert_eq!(recipe.name().as_str(), "Cap");
    assert_eq!(recipe.deleted_at(), None);
    assert!(!recipe.is_deleted());
    assert_eq!(states(&recipe), vec![(1, VersionState::LiveDraft, 1)]);
    let first = recipe.version(1).unwrap();
    assert_eq!(first.id(), vid(11));
}

#[test]
fn version_finds_a_header_by_number_and_none_for_an_unknown_number() {
    let recipe = recipe(vec![
        header(1, VersionState::Approved, 3),
        header(2, VersionState::LiveDraft, 1),
    ]);
    assert_eq!(recipe.version(2).map(VersionHeader::id), Some(vid(2)));
    assert!(recipe.version(3).is_none());
    assert!(recipe.version(0).is_none());
}

#[test]
fn try_from_sorts_the_headers_by_number() {
    let recipe = recipe(vec![
        header(3, VersionState::Discarded, 2),
        header(1, VersionState::Retired, 5),
        header(2, VersionState::Approved, 4),
    ]);
    let numbers: Vec<u32> = recipe
        .versions()
        .iter()
        .map(VersionHeader::number)
        .collect();
    assert_eq!(numbers, vec![1, 2, 3]);
}

#[test]
fn try_from_keeps_the_deletion_mark() {
    let recipe = deleted_recipe(vec![header(1, VersionState::Retired, 2)]);
    assert!(recipe.is_deleted());
    assert_eq!(recipe.deleted_at(), Some(ts(1_000)));
}

#[test]
fn try_from_accepts_discarded_drafts_and_one_approved_and_one_live_draft() {
    let result = Recipe::try_from(stored(
        None,
        vec![
            header(1, VersionState::Discarded, 2),
            header(2, VersionState::Discarded, 2),
            header(3, VersionState::Approved, 4),
            header(4, VersionState::LiveDraft, 1),
        ],
    ));
    assert!(result.is_ok());
}

#[test]
fn try_from_rejects_the_number_zero() {
    let result = Recipe::try_from(stored(None, vec![header(0, VersionState::LiveDraft, 1)]));
    assert_eq!(result.unwrap_err(), StoredRecipeReason::NumberZero);
}

#[test]
fn try_from_rejects_a_number_above_the_sql_integer_range() {
    let too_large = u32::try_from(i32::MAX).unwrap().checked_add(1).unwrap();
    let result = Recipe::try_from(stored(
        None,
        vec![header(too_large, VersionState::Retired, 2)],
    ));
    assert_eq!(result.unwrap_err(), StoredRecipeReason::NumberTooLarge);
}

#[test]
fn try_from_rejects_the_largest_sql_integer_number_without_the_lower_numbers() {
    let largest = u32::try_from(i32::MAX).unwrap();
    let result = Recipe::try_from(stored(
        None,
        vec![header(largest, VersionState::Retired, 2)],
    ));
    assert_eq!(result.unwrap_err(), StoredRecipeReason::NumberGap);
}

#[test]
fn try_from_rejects_a_duplicate_number() {
    let mut second = header(1, VersionState::Retired, 2);
    second.id = vid(77);
    let result = Recipe::try_from(stored(
        None,
        vec![header(1, VersionState::Approved, 2), second],
    ));
    assert_eq!(result.unwrap_err(), StoredRecipeReason::DuplicateNumber);
}

#[test]
fn try_from_rejects_a_duplicate_identifier() {
    let mut second = header(2, VersionState::Retired, 2);
    second.id = vid(1);
    let result = Recipe::try_from(stored(
        None,
        vec![header(1, VersionState::Approved, 2), second],
    ));
    assert_eq!(result.unwrap_err(), StoredRecipeReason::DuplicateId);
}

#[test]
fn try_from_rejects_a_row_version_below_one() {
    for row_version in [0, -1] {
        let result = Recipe::try_from(stored(
            None,
            vec![header(1, VersionState::LiveDraft, row_version)],
        ));
        assert_eq!(result.unwrap_err(), StoredRecipeReason::RowVersionBelowOne);
    }
}

#[test]
fn try_from_rejects_two_approved_headers() {
    let result = Recipe::try_from(stored(
        None,
        vec![
            header(1, VersionState::Approved, 2),
            header(2, VersionState::Approved, 2),
        ],
    ));
    assert_eq!(result.unwrap_err(), StoredRecipeReason::TwoApproved);
}

#[test]
fn try_from_rejects_two_live_drafts() {
    let result = Recipe::try_from(stored(
        None,
        vec![
            header(1, VersionState::LiveDraft, 1),
            header(2, VersionState::LiveDraft, 1),
        ],
    ));
    assert_eq!(result.unwrap_err(), StoredRecipeReason::TwoLiveDrafts);
}

#[test]
fn stored_recipe_reasons_have_their_phrases() {
    let cases = [
        (
            StoredRecipeReason::NumberZero,
            "a version number is at least 1",
        ),
        (
            StoredRecipeReason::NumberTooLarge,
            "a version number is at most 2147483647",
        ),
        (
            StoredRecipeReason::DuplicateNumber,
            "two versions of a recipe have the same number",
        ),
        (
            StoredRecipeReason::DuplicateId,
            "two versions of a recipe have the same identifier",
        ),
        (
            StoredRecipeReason::RowVersionBelowOne,
            "a row version is at least 1",
        ),
        (
            StoredRecipeReason::TwoApproved,
            "a recipe has at most one approved version",
        ),
        (
            StoredRecipeReason::TwoLiveDrafts,
            "a recipe has at most one live draft",
        ),
    ];
    for (reason, phrase) in cases {
        assert_eq!(reason.to_string(), phrase);
    }
}

#[test]
fn rename_replaces_the_name() {
    let mut recipe = recipe(vec![header(1, VersionState::LiveDraft, 1)]);
    recipe.rename(name("Bottle")).unwrap();
    assert_eq!(recipe.name().as_str(), "Bottle");
}

#[test]
fn rename_of_a_deleted_recipe_gives_recipe_deleted_and_keeps_the_name() {
    let mut recipe = deleted_recipe(vec![header(1, VersionState::Retired, 2)]);
    let error = recipe.rename(name("Bottle")).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert_eq!(
        error.to_string(),
        "Recipe R-1 is deleted. An administrator can restore it."
    );
    assert_eq!(recipe.name().as_str(), "Recipe");
}

#[test]
fn optimistic_holds_a_value_or_both_row_versions() {
    let applied: Optimistic<u8> = Optimistic::Applied(7);
    let conflict: Optimistic<u8> = Optimistic::Conflict {
        expected: 1,
        actual: 2,
    };
    assert_eq!(applied.clone(), Optimistic::Applied(7));
    assert_ne!(applied, conflict);
    assert_eq!(
        conflict,
        Optimistic::Conflict {
            expected: 1,
            actual: 2
        }
    );
}

#[test]
fn try_from_rejects_a_deleted_recipe_with_an_approved_version() {
    let result = Recipe::try_from(stored(
        Some(ts(1_000)),
        vec![header(1, VersionState::Approved, 2)],
    ));
    assert_eq!(result.unwrap_err(), StoredRecipeReason::DeletedWithApproved);
}

#[test]
fn try_from_rejects_a_deleted_recipe_with_a_live_draft() {
    let result = Recipe::try_from(stored(
        Some(ts(1_000)),
        vec![
            header(1, VersionState::Retired, 2),
            header(2, VersionState::LiveDraft, 1),
        ],
    ));
    assert_eq!(
        result.unwrap_err(),
        StoredRecipeReason::DeletedWithLiveDraft
    );
}

#[test]
fn try_from_accepts_a_deleted_recipe_with_retired_and_discarded_versions() {
    let result = Recipe::try_from(stored(
        Some(ts(1_000)),
        vec![
            header(1, VersionState::Retired, 2),
            header(2, VersionState::Discarded, 2),
        ],
    ));
    assert!(result.is_ok());
}

#[test]
fn try_from_rejects_numbers_with_a_gap() {
    for numbers in [vec![2], vec![1, 3], vec![1, 2, 4]] {
        let headers = numbers
            .into_iter()
            .map(|number| header(number, VersionState::Retired, 2))
            .collect();
        let result = Recipe::try_from(stored(None, headers));
        assert_eq!(result.unwrap_err(), StoredRecipeReason::NumberGap);
    }
}

#[test]
fn the_new_reasons_have_their_phrases() {
    assert_eq!(
        StoredRecipeReason::DeletedWithApproved.to_string(),
        "a deleted recipe has no approved version"
    );
    assert_eq!(
        StoredRecipeReason::DeletedWithLiveDraft.to_string(),
        "a deleted recipe has no live draft"
    );
    assert_eq!(
        StoredRecipeReason::NumberGap.to_string(),
        "the version numbers of a recipe run from 1 without a gap"
    );
}
