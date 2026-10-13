use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{LeafPath, PlcPlatform};
use recipe_domain::recipe::{LimitInput, LimitList};
use serde_json::json;

use super::{ReplaceLimits, ReplaceLimitsInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, lock_line, meta, parameter_on, pid, pkey, reference, rid, uid, vid,
};
use crate::memory::{JournalEntry, LockMode};
use crate::ports::Isolation;
use crate::recipe::DraftOutput;
use crate::recipe::test_support::{
    DRAFT_VERSION, seed_deleted, seed_speed_draft, speed_key, stored_parameters,
};

fn row(path: &str, min: Option<&str>, max: Option<&str>, max_length: Option<u16>) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: min.map(str::to_owned),
        max: max.map(str::to_owned),
        max_length,
    }
}

fn input(rows: Vec<LimitInput>, expected: i64) -> ReplaceLimitsInput {
    ReplaceLimitsInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: expected,
        key: speed_key(),
        limits: LimitList::parse(rows).unwrap(),
    }
}

fn assert_nothing_written(fixture: &Fixture) {
    assert!(fixture.store.audit_entries().is_empty());
    assert!(
        fixture
            .store
            .state()
            .editors_of(vid(DRAFT_VERSION))
            .is_empty()
    );
    let parameters = stored_parameters(fixture);
    let speed = parameters.first().unwrap();
    assert_eq!(speed.limits.len(), 1);
    assert_eq!(
        speed.limits.first().unwrap().max,
        Some("250".parse().unwrap())
    );
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(ReplaceLimits::OPERATION, Operation::ReplaceLimits);
}

#[tokio::test]
async fn success_replaces_the_limits_and_keeps_value_and_translation() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let before = stored_parameters(&fixture);
    let rows = vec![row("", Some("0"), Some("200"), None)];
    let output = ReplaceLimits::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(rows, 1))
        .await
        .unwrap();
    assert_eq!(
        output,
        DraftOutput {
            version: vid(DRAFT_VERSION),
            row_version: 2
        }
    );
    let after = stored_parameters(&fixture);
    let (old, new) = (before.first().unwrap(), after.first().unwrap());
    assert_eq!(new.limits.len(), 1);
    assert_eq!(new.limits.first().unwrap().min, Some("0".parse().unwrap()));
    assert_eq!(
        new.limits.first().unwrap().max,
        Some("200".parse().unwrap())
    );
    assert_eq!(new.value, old.value);
    assert_eq!(new.translations, old.translations);
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
            lock_line(
                "recipe_version",
                LockMode::ForUpdate,
                vid(DRAFT_VERSION).to_string()
            ),
        ]
    );
}

#[tokio::test]
async fn the_audit_entry_is_a_limit_diff_that_names_the_leaf_path() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let engineer = actor(Role::ProcessEngineer);
    let rows = vec![row("", Some("0"), Some("200"), None)];
    ReplaceLimits::new(fixture.context())
        .execute(&engineer, &meta(), input(rows, 1))
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::LimitsReplaced);
    assert_eq!(entry.target().entity_type(), "parameter");
    assert_eq!(entry.target().entity_id(), pid(1).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(entry.target().aggregate_id(), rid(1).as_uuid());
    assert_eq!(
        entry.before(),
        Some(&json!({"limits": {"": {"min_value": "10", "max_value": "250", "max_length": null}}}))
    );
    assert_eq!(
        entry.after(),
        Some(&json!({"limits": {"": {"min_value": "0", "max_value": "200", "max_length": null}}}))
    );
    assert_eq!(entry.actor(), Some(engineer.user()));
}

#[tokio::test]
async fn an_empty_list_removes_every_limit() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    ReplaceLimits::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(Vec::new(), 1))
        .await
        .unwrap();
    let after = stored_parameters(&fixture);
    assert!(after.first().unwrap().limits.is_empty());
    let entries = fixture.store.audit_entries();
    assert_eq!(
        entries.first().unwrap().after(),
        Some(&json!({"limits": {"": null}}))
    );
}

async fn rejected(rows: Vec<LimitInput>) -> (Fixture, AppError) {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = ReplaceLimits::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(rows, 1))
        .await
        .unwrap_err();
    (fixture, error)
}

#[tokio::test]
async fn a_limit_below_the_current_value_gives_rcp_004() {
    let (fixture, error) = rejected(vec![row("", Some("0"), Some("100"), None)]).await;
    assert_eq!(error.code(), ErrorCode::RCP_004);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn a_limit_outside_the_type_range_gives_rcp_006() {
    let (fixture, error) = rejected(vec![row("", Some("0"), Some("100000"), None)]).await;
    assert_eq!(error.code(), ErrorCode::RCP_006);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn an_unknown_leaf_path_gives_rcp_017() {
    let (fixture, error) = rejected(vec![row(".speed", Some("0"), Some("200"), None)]).await;
    assert_eq!(error.code(), ErrorCode::RCP_017);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn a_minimum_above_the_maximum_gives_rcp_018() {
    let (fixture, error) = rejected(vec![row("", Some("200"), Some("150"), None)]).await;
    assert_eq!(error.code(), ErrorCode::RCP_018);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn a_length_limit_on_a_number_gives_rcp_019() {
    let (fixture, error) = rejected(vec![row("", None, None, Some(5))]).await;
    assert_eq!(error.code(), ErrorCode::RCP_019);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn a_draft_above_the_parameter_budget_gives_rcp_021() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    for n in 1_u128..=4_001 {
        let tag = format!("P{n}");
        let parameter = parameter_on(PlcPlatform::TwinCat3, n, &tag, uid(101), 0);
        fixture.store.seed_parameter(vid(DRAFT_VERSION), &parameter);
    }
    let rows = vec![row("", Some("0"), Some("200"), None)];
    let error = ReplaceLimits::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(rows, 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_021);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_unknown_key_gives_rcp_051() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let mut request = input(vec![row("", Some("0"), Some("200"), None)], 1);
    request.key = pkey(99);
    let error = ReplaceLimits::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_051);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn an_old_row_version_gives_web_001() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let rows = vec![row("", Some("0"), Some("200"), None)];
    let error = ReplaceLimits::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(rows, 3))
        .await
        .unwrap_err();
    assert!(matches!(
        &error,
        AppError::ConcurrentEdit {
            expected: 3,
            actual: 1,
            ..
        }
    ));
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn an_approved_version_gives_rcp_002_and_a_deleted_recipe_gives_rcp_012() {
    let approved = Fixture::new();
    let machine = approved.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    approved.seed_approved_and_draft(&machine, 1, "R-1");
    let rows = vec![row("", Some("0"), Some("200"), None)];
    let not_editable = ReplaceLimits::new(approved.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(rows, 1))
        .await
        .unwrap_err();
    assert_eq!(not_editable.code(), ErrorCode::RCP_002);
    let deleted = Fixture::new();
    let machine = deleted.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&deleted, &machine, 1, "R-1");
    let rows = vec![row("", Some("0"), Some("200"), None)];
    let gone = ReplaceLimits::new(deleted.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(rows, 1))
        .await
        .unwrap_err();
    assert_eq!(gone.code(), ErrorCode::RCP_012);
}

#[tokio::test]
async fn an_operator_is_forbidden_and_opens_no_transaction() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let rows = vec![row("", Some("0"), Some("200"), None)];
    let error = ReplaceLimits::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input(rows, 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert_nothing_written(&fixture);
}
