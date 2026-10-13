use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::recipe::{ParameterKey, ParameterTag, StoredParameter};
use serde_json::json;

use super::UpdateParameterDefinition;
use super::requests::{run, speed_request, with_retype, with_type};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{PARAMETER_TAG_UNIQUE, StorageError};
use crate::memory::fixtures::{Fixture, actor, lock_line, meta, mid, pkey, rid, text, uid, vid};
use crate::memory::{DERIVED_TYPE_LOCK_TABLE, JournalEntry, LockMode, WriteKind};
use crate::ports::Isolation;
use crate::recipe::DraftOutput;
use crate::recipe::definition_steps::map_tag_error;
use crate::recipe::test_support::{
    DRAFT_VERSION, DraftSnapshot, bounds, seed_speed_draft, speed_key, stored_parameters,
    twincat_draft,
};

fn speed(fixture: &Fixture) -> StoredParameter {
    stored_parameters(fixture)
        .into_iter()
        .find(|row| row.key == speed_key())
        .unwrap()
}

fn advisory(name: &str) -> JournalEntry {
    lock_line(
        DERIVED_TYPE_LOCK_TABLE,
        LockMode::Advisory,
        format!("{}:{name}", mid(1)),
    )
}

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
        lock_line("unit", LockMode::ForShare, uid(101).to_string()),
    ]
}

fn actions(fixture: &Fixture) -> Vec<AuditAction> {
    fixture
        .store
        .audit_entries()
        .iter()
        .map(|entry| entry.action())
        .collect()
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(
        UpdateParameterDefinition::OPERATION,
        Operation::UpdateParameterDefinition
    );
}

#[tokio::test]
async fn a_type_change_without_a_value_gives_rcp_005_and_changes_nothing() {
    let fixture = twincat_draft();
    let before = DraftSnapshot::capture(&fixture);
    let error = run(&fixture, with_type(speed_request(), "DINT"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn the_same_type_with_a_new_name_keeps_the_value_and_the_limits() {
    let fixture = twincat_draft();
    let mut request = speed_request();
    request.name_en = text("Renamed");
    let output = run(&fixture, request).await.unwrap();
    assert_eq!(
        output,
        DraftOutput {
            version: vid(DRAFT_VERSION),
            row_version: 2
        }
    );
    let row = speed(&fixture);
    assert_eq!(row.name_en.as_str(), "Renamed");
    assert_eq!(row.value.as_json(), &json!(120));
    assert_eq!(row.limits.len(), 1);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::ParameterDefinitionUpdated);
    assert_eq!(entry.target().entity_type(), "parameter");
    assert_eq!(entry.target().entity_id(), row.id.as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(entry.target().aggregate_id(), rid(1).as_uuid());
    assert_eq!(entry.before(), Some(&json!({"name_en": "Name"})));
    assert_eq!(entry.after(), Some(&json!({"name_en": "Renamed"})));
    assert_eq!(
        fixture.store.state().editors_of(vid(DRAFT_VERSION)),
        vec![actor(Role::ProcessEngineer).user()]
    );
}

fn writes_of(fixture: &Fixture) -> Vec<(WriteKind, ParameterKey)> {
    fixture
        .store
        .draft_writes()
        .into_iter()
        .map(|write| (write.kind, write.key))
        .collect()
}

#[tokio::test]
async fn a_request_without_retype_writes_no_value_and_no_limit_row() {
    let fixture = twincat_draft();
    let mut request = speed_request();
    request.name_en = text("Renamed");
    run(&fixture, request).await.unwrap();
    assert_eq!(
        writes_of(&fixture),
        vec![(WriteKind::Definition, speed_key())]
    );
}

#[tokio::test]
async fn a_request_with_retype_writes_the_definition_then_the_value_then_the_limits() {
    let fixture = twincat_draft();
    let request = with_retype(speed_request(), "130", vec![bounds("10", "250")]);
    run(&fixture, request).await.unwrap();
    assert_eq!(
        writes_of(&fixture),
        vec![
            (WriteKind::Definition, speed_key()),
            (WriteKind::Value, speed_key()),
            (WriteKind::Limits, speed_key()),
        ]
    );
}

#[tokio::test]
async fn the_same_type_with_a_new_value_and_limits_replaces_both() {
    let fixture = twincat_draft();
    let request = with_retype(speed_request(), "130", vec![bounds("10", "250")]);
    run(&fixture, request).await.unwrap();
    let row = speed(&fixture);
    assert_eq!(row.value.as_json(), &json!(130));
    assert_eq!(row.limits.len(), 1);
    let entries = fixture.store.audit_entries();
    let entry = entries.first().unwrap();
    let before = entry.before().unwrap();
    let after = entry.after().unwrap();
    assert!(before.get("value").is_some());
    assert!(after.get("value").is_some());
    assert_ne!(before.get("value"), after.get("value"));
    assert!(after.get("name_en").is_none());
}

#[tokio::test]
async fn the_same_type_with_a_value_above_the_new_limit_gives_rcp_004() {
    let fixture = twincat_draft();
    let before = DraftSnapshot::capture(&fixture);
    let request = with_retype(speed_request(), "300", vec![bounds("10", "250")]);
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_004);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn a_type_change_with_a_value_and_limits_succeeds_and_the_diff_holds_the_type() {
    let fixture = twincat_draft();
    let old_type = speed(&fixture).plc_type;
    let request = with_retype(
        with_type(speed_request(), "DINT"),
        "130",
        vec![bounds("10", "250")],
    );
    run(&fixture, request).await.unwrap();
    let row = speed(&fixture);
    assert_ne!(row.plc_type, old_type);
    assert_eq!(row.value.as_json(), &json!(130));
    let entries = fixture.store.audit_entries();
    let entry = entries.first().unwrap();
    assert_eq!(
        entry.before().unwrap().get("plc_type_id"),
        Some(&json!(old_type.to_string()))
    );
    assert_eq!(
        entry.after().unwrap().get("plc_type_id"),
        Some(&json!(row.plc_type.to_string()))
    );
}

#[tokio::test]
async fn a_tag_that_another_parameter_holds_gives_rcp_014() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let other = crate::memory::fixtures::parameter_on(
        recipe_domain::plant::PlcPlatform::TwinCat3,
        3,
        "OTHER",
        uid(102),
        0,
    );
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &other);
    let before = DraftSnapshot::capture(&fixture);
    let mut request = speed_request();
    request.tag = ParameterTag::parse("OTHER").unwrap();
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_014);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn an_unknown_key_gives_rcp_051() {
    let fixture = twincat_draft();
    let before = DraftSnapshot::capture(&fixture);
    let mut request = speed_request();
    request.key = pkey(9);
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_051);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn an_old_row_version_gives_web_001_and_writes_nothing() {
    let fixture = twincat_draft();
    let before = DraftSnapshot::capture(&fixture);
    let mut request = speed_request();
    request.expected_row_version = 7;
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_001);
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn an_array_of_sized_strings_takes_the_string_and_array_keys_after_the_unit_lock() {
    let fixture = twincat_draft();
    let request = with_retype(
        with_type(speed_request(), "ARRAY[1..2] OF STRING(16)"),
        r#"["a","b"]"#,
        Vec::new(),
    );
    run(&fixture, request).await.unwrap();
    let mut expected = lock_prefix();
    expected.extend([advisory("STRING"), advisory("ARRAY")]);
    assert_eq!(fixture.store.journal(), expected);
}

#[tokio::test]
async fn a_same_type_update_takes_no_advisory_key() {
    let fixture = twincat_draft();
    run(&fixture, speed_request()).await.unwrap();
    assert_eq!(fixture.store.journal(), lock_prefix());
}

#[tokio::test]
async fn a_new_sized_string_gives_a_derived_type_entry_before_the_definition_entry() {
    let fixture = twincat_draft();
    let request = with_retype(
        with_type(speed_request(), "STRING(20)"),
        "\"text\"",
        Vec::new(),
    );
    run(&fixture, request).await.unwrap();
    assert_eq!(
        actions(&fixture),
        vec![
            AuditAction::DerivedTypeCreated,
            AuditAction::ParameterDefinitionUpdated
        ]
    );
}

#[test]
fn the_constraint_parameter_tag_unique_maps_to_rcp_014() {
    let tag = ParameterTag::parse("FEED").unwrap();
    let error = map_tag_error(
        StorageError::Duplicate {
            constraint: PARAMETER_TAG_UNIQUE.to_owned(),
        },
        &tag,
        3,
    );
    assert_eq!(error.code(), ErrorCode::RCP_014);
}

#[test]
fn another_constraint_stays_a_storage_error() {
    let tag = ParameterTag::parse("FEED").unwrap();
    let error = map_tag_error(
        StorageError::Duplicate {
            constraint: "other_unique".to_owned(),
        },
        &tag,
        3,
    );
    assert_eq!(error.code(), ErrorCode::DB_006);
}

#[tokio::test]
async fn an_operator_is_forbidden_and_opens_no_transaction() {
    let fixture = twincat_draft();
    let error = UpdateParameterDefinition::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), speed_request())
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
}
