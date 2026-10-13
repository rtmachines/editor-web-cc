//! Tests of `Recipe::approve` and `Recipe::approval_replay` (design D4, D5, ADR-0028).

use serde_json::json;

use super::super::support::{header, recipe as recipe_of, reference, vid};
use super::*;
use crate::error_codes::ErrorCode;
use crate::identity::UserId;
use crate::plant::PlcPlatform;
use crate::recipe::draft::support::{stored_draft_with, user};
use crate::recipe::snapshot::fixtures::{INT, limited, param, range, trees, units};
use crate::recipe::{ApprovalReady, Draft};

use VersionState::{Approved, Discarded, LiveDraft, Retired};

/// The row version of the draft header and of the token.
const DRAFT_ROW_VERSION: i64 = 7;

fn login(text: &str) -> Login {
    Login::parse(text).unwrap()
}

/// The approver with the user number `n`. The login names the number.
fn approver(n: u128) -> Approver {
    Approver {
        user: user(n),
        login: login(&format!("user{n}")),
    }
}

/// Version 3 of recipe R-1. The creator is user 1. The editors are users 2 and 3.
fn token_draft() -> Draft {
    let parameter = limited(param(1, "SPEED", INT, json!(5)), vec![range("", 0, 100)]);
    let stored = stored_draft_with(LiveDraft, PlcPlatform::TwinCat3, vec![parameter]);
    Draft::load(stored).unwrap()
}

/// A real token of `draft`.
fn ready(draft: Draft) -> ApprovalReady {
    draft
        .check_for_approval(&units(false), &trees(), false)
        .unwrap()
}

/// The header of the draft of the token, with the given state and row version.
fn draft_header(state: VersionState, row_version: i64) -> StoredVersionHeader {
    StoredVersionHeader {
        id: vid(30),
        number: 3,
        state,
        row_version,
    }
}

/// Version 2 is approved with the row version 5. Version 3 is the draft of the token.
fn recipe_with_approved() -> Recipe {
    recipe_of(vec![
        header(1, Retired, 4),
        header(2, Approved, 5),
        draft_header(LiveDraft, DRAFT_ROW_VERSION),
    ])
}

fn state_of(recipe: &Recipe, number: u32) -> VersionState {
    recipe.version(number).unwrap().state()
}

fn policy_on() -> ApprovalPolicy {
    ApprovalPolicy::default()
}

#[test]
fn the_default_policy_requires_separation_of_duties() {
    assert!(ApprovalPolicy::default().separation_of_duties);
}

#[test]
fn approve_retires_the_approved_version_and_approves_the_draft() {
    let mut recipe = recipe_with_approved();
    let outcome = recipe
        .approve(
            ready(token_draft()),
            DRAFT_ROW_VERSION,
            &approver(9),
            &policy_on(),
        )
        .unwrap();
    assert_eq!(
        outcome,
        Optimistic::Applied(ApprovalOutcome::Approved {
            approved: vid(30),
            retired: Some(vid(2)),
        })
    );
    assert_eq!(state_of(&recipe, 1), Retired);
    assert_eq!(state_of(&recipe, 2), Retired);
    assert_eq!(state_of(&recipe, 3), Approved);
}

#[test]
fn approve_of_the_first_draft_retires_nothing() {
    let mut recipe = recipe_of(vec![
        header(1, Discarded, 4),
        header(2, Retired, 5),
        draft_header(LiveDraft, DRAFT_ROW_VERSION),
    ]);
    let outcome = recipe
        .approve(
            ready(token_draft()),
            DRAFT_ROW_VERSION,
            &approver(9),
            &policy_on(),
        )
        .unwrap();
    assert_eq!(
        outcome,
        Optimistic::Applied(ApprovalOutcome::Approved {
            approved: vid(30),
            retired: None,
        })
    );
    assert_eq!(state_of(&recipe, 1), Discarded);
    assert_eq!(state_of(&recipe, 2), Retired);
    assert_eq!(state_of(&recipe, 3), Approved);
}

#[test]
fn the_creator_cannot_approve_and_the_error_names_the_login() {
    let mut recipe = recipe_with_approved();
    let before = recipe.clone();
    let error = recipe
        .approve(
            ready(token_draft()),
            DRAFT_ROW_VERSION,
            &approver(1),
            &policy_on(),
        )
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_015);
    assert_eq!(
        error.to_string(),
        "User user1 created or edited draft 3 of recipe R-1. A different user must approve it."
    );
    assert_eq!(recipe, before);
}

#[test]
fn each_editor_cannot_approve() {
    for n in [2, 3] {
        let mut recipe = recipe_with_approved();
        let before = recipe.clone();
        let error = recipe
            .approve(
                ready(token_draft()),
                DRAFT_ROW_VERSION,
                &approver(n),
                &policy_on(),
            )
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_015, "user {n}");
        assert_eq!(recipe, before, "user {n}");
    }
}

#[test]
fn with_separation_of_duties_off_the_creator_can_approve() {
    let mut recipe = recipe_with_approved();
    let policy = ApprovalPolicy {
        separation_of_duties: false,
    };
    for n in [1, 2] {
        let mut recipe_n = recipe.clone();
        let outcome = recipe_n
            .approve(
                ready(token_draft()),
                DRAFT_ROW_VERSION,
                &approver(n),
                &policy,
            )
            .unwrap();
        assert!(matches!(
            outcome,
            Optimistic::Applied(ApprovalOutcome::Approved { .. })
        ));
        assert_eq!(state_of(&recipe_n, 3), Approved);
    }
    // The original recipe is not changed by the clones.
    assert_eq!(state_of(&recipe, 3), LiveDraft);
    recipe
        .approve(
            ready(token_draft()),
            DRAFT_ROW_VERSION,
            &approver(1),
            &policy,
        )
        .unwrap();
    assert_eq!(state_of(&recipe, 3), Approved);
}

#[test]
fn replay_answers_already_approved_for_the_next_row_version() {
    let recipe = recipe_of(vec![header(1, Retired, 4), draft_header(Approved, 7)]);
    assert_eq!(
        recipe.approval_replay(3, 6),
        Some(ApprovalOutcome::AlreadyApproved)
    );
    assert_eq!(recipe.approval_replay(3, 4), None);
    assert_eq!(recipe.approval_replay(3, 7), None);
    assert_eq!(recipe.approval_replay(1, 3), None);
    assert_eq!(recipe.approval_replay(9, 6), None);
}

#[test]
fn replay_needs_an_approved_header() {
    let recipe = recipe_of(vec![draft_header(Retired, 7)]);
    assert_eq!(recipe.approval_replay(3, 6), None);
}

#[test]
fn replay_with_the_largest_expected_row_version_gives_none() {
    let recipe = recipe_of(vec![draft_header(Approved, i64::MAX)]);
    assert_eq!(recipe.approval_replay(3, i64::MAX), None);
}

#[test]
fn approve_of_an_already_approved_header_at_the_next_row_version_is_idempotent() {
    let mut recipe = recipe_of(vec![header(1, Retired, 4), draft_header(Approved, 8)]);
    let before = recipe.clone();
    let outcome = recipe
        .approve(
            ready(token_draft()),
            DRAFT_ROW_VERSION,
            &approver(9),
            &policy_on(),
        )
        .unwrap();
    assert_eq!(
        outcome,
        Optimistic::Applied(ApprovalOutcome::AlreadyApproved)
    );
    assert_eq!(recipe, before);
}

#[test]
fn an_expected_row_version_older_than_the_header_gives_conflict() {
    let mut recipe = recipe_with_approved();
    let before = recipe.clone();
    let outcome = recipe
        .approve(ready(token_draft()), 6, &approver(9), &policy_on())
        .unwrap();
    assert_eq!(
        outcome,
        Optimistic::Conflict {
            expected: 6,
            actual: DRAFT_ROW_VERSION
        }
    );
    assert_eq!(recipe, before);
}

#[test]
fn a_token_of_an_older_snapshot_gives_conflict() {
    let mut recipe = recipe_of(vec![header(2, Approved, 5), draft_header(LiveDraft, 9)]);
    let before = recipe.clone();
    let outcome = recipe
        .approve(ready(token_draft()), 9, &approver(9), &policy_on())
        .unwrap();
    assert_eq!(
        outcome,
        Optimistic::Conflict {
            expected: DRAFT_ROW_VERSION,
            actual: 9
        }
    );
    assert_eq!(recipe, before);
}

#[test]
fn approve_of_a_deleted_recipe_gives_recipe_deleted() {
    let mut recipe =
        super::super::support::deleted_unchecked(vec![draft_header(LiveDraft, DRAFT_ROW_VERSION)]);
    let before = recipe.clone();
    let error = recipe
        .approve(
            ready(token_draft()),
            DRAFT_ROW_VERSION,
            &approver(9),
            &policy_on(),
        )
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert_eq!(recipe, before);
}

#[test]
fn a_retired_or_discarded_header_gives_version_not_draft() {
    for state in [Retired, Discarded] {
        let mut recipe = recipe_of(vec![draft_header(state, DRAFT_ROW_VERSION)]);
        let before = recipe.clone();
        let error = recipe
            .approve(
                ready(token_draft()),
                DRAFT_ROW_VERSION,
                &approver(9),
                &policy_on(),
            )
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_007, "{state:?}");
        assert_eq!(
            error.to_string(),
            "Version 3 of recipe R-1 is not a draft.",
            "{state:?}"
        );
        assert_eq!(recipe, before, "{state:?}");
    }
}

#[test]
fn an_approved_header_with_another_row_version_gives_version_not_draft() {
    let mut recipe = recipe_of(vec![draft_header(Approved, DRAFT_ROW_VERSION)]);
    let before = recipe.clone();
    let error = recipe
        .approve(
            ready(token_draft()),
            DRAFT_ROW_VERSION,
            &approver(9),
            &policy_on(),
        )
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_007);
    assert_eq!(recipe, before);
}

#[test]
fn a_token_for_an_unknown_version_gives_version_not_draft() {
    let mut recipe = recipe_of(vec![header(1, Approved, 5), header(3, LiveDraft, 7)]);
    let before = recipe.clone();
    let error = recipe
        .approve(
            ready(token_draft()),
            DRAFT_ROW_VERSION,
            &approver(9),
            &policy_on(),
        )
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_007);
    assert_eq!(
        error.to_string(),
        format!("Version 3 of recipe {} is not a draft.", reference())
    );
    assert_eq!(recipe, before);
}

#[test]
fn the_approver_user_is_compared_by_identifier() {
    let stranger = Approver {
        user: UserId::from(uuid::Uuid::from_u128(77)),
        login: login("stranger"),
    };
    let mut recipe = recipe_with_approved();
    assert!(
        recipe
            .approve(
                ready(token_draft()),
                DRAFT_ROW_VERSION,
                &stranger,
                &policy_on()
            )
            .is_ok()
    );
}

#[test]
fn check_approvable_accepts_a_live_draft() {
    assert!(recipe_with_approved().check_approvable(3).is_ok());
}

#[test]
fn check_approvable_gives_version_not_draft_for_a_retired_or_discarded_version() {
    let recipe = recipe_with_approved();
    for number in [1, 2] {
        let error = recipe.check_approvable(number).unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_007, "{number}");
    }
    let discarded = recipe_of(vec![draft_header(Discarded, 2)]);
    assert_eq!(
        discarded.check_approvable(3).unwrap_err().code(),
        ErrorCode::RCP_007
    );
}

#[test]
fn check_approvable_gives_version_not_draft_for_an_unknown_number() {
    let error = recipe_with_approved().check_approvable(9).unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_007);
    assert!(error.to_string().contains('9'), "{error}");
}

#[test]
fn check_approvable_gives_recipe_deleted_for_a_deleted_recipe() {
    let recipe =
        super::super::support::deleted_unchecked(vec![draft_header(LiveDraft, DRAFT_ROW_VERSION)]);
    assert_eq!(
        recipe.check_approvable(3).unwrap_err().code(),
        ErrorCode::RCP_012
    );
}
