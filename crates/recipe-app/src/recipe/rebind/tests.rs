//! Tests of the locks and the plan of `RebindTypeRevision` (design D14, steps 1 to 3).

use recipe_domain::audit::SecurityEventKind;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{LeafPath, PlcPlatform};
use recipe_domain::recipe::{RebindTargetReason, RecipeError, StoredLimit};
use serde_json::json;

use super::fixtures::{
    DROP_1, DROP_2, OTHER_2, ZONE_1, ZONE_2, empty_draft, request, run, seed_parameter, seed_rows,
    with_value, zone_draft, zone_request, zones_draft, zones_request, zones_value,
};
use super::{RebindTypeRevision, RebindTypeRevisionInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, actor, lock_line, meta, mid, rid, tid, vid};
use crate::memory::{DERIVED_TYPE_LOCK_TABLE, JournalEntry, LockMode};
use crate::ports::Isolation;
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot};

fn lock_prefix() -> Vec<JournalEntry> {
    vec![
        JournalEntry::Begin(Isolation::ReadOnly),
        JournalEntry::Begin(Isolation::ReadCommitted),
        lock_line("machine", LockMode::ForShare, mid(1).to_string()),
        lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
        lock_line(
            "recipe_version",
            LockMode::ForUpdate,
            vid(DRAFT_VERSION).to_string(),
        ),
    ]
}

fn array_key() -> JournalEntry {
    lock_line(
        DERIVED_TYPE_LOCK_TABLE,
        LockMode::Advisory,
        format!("{}:ARRAY", mid(1)),
    )
}

fn target_reason(error: &AppError) -> Option<RebindTargetReason> {
    match error {
        AppError::Recipe(RecipeError::RebindTargetInvalid { reason, .. }) => Some(*reason),
        _ => None,
    }
}

fn drop_limit() -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(".old").unwrap(),
        min: Some("0".parse().unwrap()),
        max: Some("9".parse().unwrap()),
        max_length: None,
    }
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(RebindTypeRevision::OPERATION, Operation::RebindTypeRevision);
}

#[tokio::test]
async fn a_plain_struct_parameter_takes_the_three_locks_and_no_advisory_key() {
    let fixture = zone_draft();
    run(&fixture, zone_request()).await.unwrap();
    assert_eq!(fixture.store.journal(), lock_prefix());
}

#[tokio::test]
async fn an_array_parameter_takes_the_array_key_after_the_three_locks() {
    let fixture = zones_draft();
    run(&fixture, zones_request()).await.unwrap();
    let mut expected = lock_prefix();
    expected.push(array_key());
    assert_eq!(fixture.store.journal(), expected);
}

/// Run `input` on `fixture`, assert the code, and assert that nothing was written.
async fn assert_rejected(
    fixture: &Fixture,
    input: RebindTypeRevisionInput,
    code: ErrorCode,
) -> AppError {
    let before = DraftSnapshot::capture(fixture);
    let error = run(fixture, input).await.unwrap_err();
    assert_eq!(error.code(), code);
    before.assert_unchanged(fixture);
    error
}

#[tokio::test]
async fn an_older_target_gives_rcp_052() {
    let fixture = zone_draft();
    let error = assert_rejected(&fixture, request(ZONE_2, ZONE_1), ErrorCode::RCP_052).await;
    assert_eq!(target_reason(&error), Some(RebindTargetReason::NotNewer));
}

#[tokio::test]
async fn a_target_of_another_struct_gives_rcp_052() {
    let fixture = zone_draft();
    let error = assert_rejected(&fixture, request(ZONE_1, OTHER_2), ErrorCode::RCP_052).await;
    assert_eq!(target_reason(&error), Some(RebindTargetReason::OtherStruct));
}

#[tokio::test]
async fn a_source_that_no_parameter_uses_gives_rcp_052() {
    let fixture = empty_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::TwinCat3,
        (11, "ZONE1"),
        (DROP_1, json!({"setpoint": 1, "old": 2})),
        Vec::new(),
    );
    let error = assert_rejected(&fixture, request(ZONE_1, ZONE_2), ErrorCode::RCP_052).await;
    assert_eq!(target_reason(&error), Some(RebindTargetReason::NotUsed));
}

#[tokio::test]
async fn a_target_that_does_not_exist_gives_rcp_052() {
    let fixture = zone_draft();
    let error = assert_rejected(&fixture, request(ZONE_1, 999), ErrorCode::RCP_052).await;
    assert_eq!(
        target_reason(&error),
        Some(RebindTargetReason::TargetNotFound)
    );
}

#[tokio::test]
async fn a_new_member_without_a_value_gives_rcp_028() {
    let fixture = zone_draft();
    assert_rejected(&fixture, request(ZONE_1, ZONE_2), ErrorCode::RCP_028).await;
}

#[tokio::test]
async fn two_parameters_with_missing_values_give_one_list_with_both_entries() {
    let fixture = zone_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::TwinCat3,
        (13, "ZONE2"),
        (ZONE_1, json!({"setpoint": 2})),
        Vec::new(),
    );
    let error = assert_rejected(&fixture, request(ZONE_1, ZONE_2), ErrorCode::RCP_028).await;
    assert!(
        matches!(
            &error,
            AppError::Recipes(list)
                if list.len() == 2 && list.iter().all(|entry| entry.code() == ErrorCode::RCP_028)
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_limit_on_a_dropped_member_gives_rcp_029() {
    let fixture = empty_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::TwinCat3,
        (14, "DROP1"),
        (DROP_1, json!({"setpoint": 1, "old": 2})),
        vec![drop_limit()],
    );
    assert_rejected(&fixture, request(DROP_1, DROP_2), ErrorCode::RCP_029).await;
}

#[tokio::test]
async fn a_sent_value_of_the_wrong_shape_gives_rcp_005() {
    let fixture = zone_draft();
    let input = with_value(request(ZONE_1, ZONE_2), (11, ".extra"), &json!("abc"));
    assert_rejected(&fixture, input, ErrorCode::RCP_005).await;
}

#[tokio::test]
async fn a_target_tree_above_the_leaf_limit_gives_plt_018_and_changes_nothing() {
    let fixture = empty_draft();
    fixture.store.seed_type(&super::fixtures::array_row(
        PlcPlatform::TwinCat3,
        920,
        tid(ZONE_1),
        16_384,
    ));
    seed_parameter(
        &fixture,
        PlcPlatform::TwinCat3,
        (15, "HUGE"),
        (920, zones_value(16_384)),
        Vec::new(),
    );
    assert_rejected(&fixture, request(ZONE_1, ZONE_2), ErrorCode::PLT_018).await;
}

#[tokio::test]
async fn a_failed_plan_takes_no_advisory_key() {
    let fixture = zones_draft();
    run(&fixture, request(ZONE_1, ZONE_2)).await.unwrap_err();
    assert_eq!(fixture.store.journal(), lock_prefix());
}

#[tokio::test]
async fn an_approved_version_gives_rcp_002() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    seed_rows(&fixture, PlcPlatform::TwinCat3);
    let error = run(&fixture, zone_request()).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_002);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_deleted_recipe_gives_rcp_012() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    crate::recipe::test_support::seed_deleted(&fixture, &machine, 1, "R-1");
    seed_rows(&fixture, PlcPlatform::TwinCat3);
    let error = run(&fixture, zone_request()).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_old_row_version_gives_web_001() {
    let fixture = zone_draft();
    let mut input = zone_request();
    input.expected_row_version = 7;
    assert_rejected(&fixture, input, ErrorCode::WEB_001).await;
}

#[tokio::test]
async fn an_operator_is_forbidden_and_opens_no_transaction() {
    let fixture = zone_draft();
    let error = RebindTypeRevision::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), zone_request())
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
}
