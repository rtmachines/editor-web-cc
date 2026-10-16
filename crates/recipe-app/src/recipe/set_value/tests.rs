use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::{ValueInput, VersionState};
use serde_json::json;

use super::{SetParameterValue, SetParameterValueInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{
    Fixture, actor, lock_line, meta, parameter_on, pid, pkey, recipe_with, reference, rid, uid,
    user, vid,
};
use crate::memory::{JournalEntry, LockMode};
use crate::ports::Isolation;
use crate::recipe::DraftOutput;
use crate::recipe::test_support::{
    DRAFT_VERSION, seed_deleted, seed_speed_draft, speed_key, stored_parameters,
};

fn input(value: &str, expected: i64) -> SetParameterValueInput {
    SetParameterValueInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: expected,
        key: speed_key(),
        value: ValueInput::Json(value.to_owned()),
    }
}

fn write_journal() -> Vec<JournalEntry> {
    vec![
        JournalEntry::Begin(Isolation::ReadOnly),
        JournalEntry::Begin(Isolation::ReadCommitted),
        lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
        lock_line(
            "recipe_version",
            LockMode::ForUpdate,
            vid(DRAFT_VERSION).to_string(),
        ),
    ]
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
    assert_eq!(parameters.len(), 1);
    assert_eq!(
        parameters.first().unwrap().value.as_json(),
        &json!(120),
        "the value must stay"
    );
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(SetParameterValue::OPERATION, Operation::SetParameterValue);
}

#[tokio::test]
async fn success_returns_the_new_row_version_and_keeps_limits_and_translations() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let before = stored_parameters(&fixture);
    let output = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("150", 1))
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
    assert_eq!(new.value.as_json(), &json!(150));
    assert_eq!(new.limits, old.limits);
    assert_eq!(new.translations, old.translations);
    assert_eq!(new.tag, old.tag);
    assert_eq!(fixture.store.journal(), write_journal());
}

#[tokio::test]
async fn draft_editor_holds_the_user_once_after_two_commands() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let use_case = SetParameterValue::new(fixture.context());
    let engineer = actor(Role::ProcessEngineer);
    let first = use_case
        .execute(&engineer, &meta(), input("150", 1))
        .await
        .unwrap();
    let second = use_case
        .execute(&engineer, &meta(), input("160", first.row_version))
        .await
        .unwrap();
    assert_eq!(second.row_version, 3);
    let editors = fixture.store.state().editors_of(vid(DRAFT_VERSION));
    assert_eq!(editors, vec![engineer.user()]);
    assert_eq!(fixture.store.audit_entries().len(), 2);
}

#[tokio::test]
async fn the_audit_entry_is_a_value_diff_on_the_leaf_path() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let engineer = actor(Role::ProcessEngineer);
    let request = meta();
    SetParameterValue::new(fixture.context())
        .execute(&engineer, &request, input("150", 1))
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::ParameterValueSet);
    assert_eq!(entry.target().entity_type(), "parameter");
    assert_eq!(entry.target().entity_id(), pid(1).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(entry.target().aggregate_id(), rid(1).as_uuid());
    assert_eq!(entry.before(), Some(&json!({"value": {"": 120}})));
    assert_eq!(entry.after(), Some(&json!({"value": {"": 150}})));
    assert_eq!(entry.actor(), Some(engineer.user()));
    assert_eq!(entry.request_id(), request.request_id());
}

#[tokio::test]
async fn an_approved_version_gives_rcp_002() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let error = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("150", 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_002);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_deleted_recipe_gives_rcp_012() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let error = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("150", 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_old_row_version_gives_web_001_with_expected_and_actual() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("150", 7))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_001);
    assert!(matches!(
        &error,
        AppError::ConcurrentEdit {
            number: 1,
            expected: 7,
            actual: 1,
            ..
        }
    ));
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn an_unknown_key_gives_rcp_051() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let mut request = input("150", 1);
    request.key = pkey(99);
    let error = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_051);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn a_value_outside_the_limit_gives_rcp_004_and_writes_nothing() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("300", 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_004);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn a_value_of_the_wrong_shape_gives_rcp_005() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("\"fast\"", 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
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
    let error = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("150", 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_021);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_unknown_reference_or_number_gives_web_005() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let use_case = SetParameterValue::new(fixture.context());
    let engineer = actor(Role::ProcessEngineer);
    let mut other = input("150", 1);
    other.number = 9;
    let number = use_case
        .execute(&engineer, &meta(), other)
        .await
        .unwrap_err();
    assert!(matches!(
        &number,
        AppError::NotFound {
            entity: Missing::Version { number: 9, .. }
        }
    ));
    let mut unknown = input("150", 1);
    unknown.reference = reference("R-9");
    let reference_error = use_case
        .execute(&engineer, &meta(), unknown)
        .await
        .unwrap_err();
    assert_eq!(reference_error.code(), ErrorCode::WEB_005);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_version_approved_between_the_read_and_the_locks_gives_rcp_002() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let context = fixture.context_racing(|store| {
        let approved = recipe_with((1, 1), "R-1", &[(DRAFT_VERSION, VersionState::Approved)]);
        store.seed_recipe(&approved, PlcPlatform::TwinCat3, user(2));
    });
    let error = SetParameterValue::new(context)
        .execute(&actor(Role::ProcessEngineer), &meta(), input("150", 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_002);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_operator_is_forbidden_and_opens_no_transaction() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("150", 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert_nothing_written(&fixture);
}
