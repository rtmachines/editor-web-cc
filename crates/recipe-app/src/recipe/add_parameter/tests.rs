use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{LeafPath, PlcPlatform, TypeExpr, UnitPath};
use recipe_domain::recipe::{
    LimitInput, LimitList, Ordinal, ParameterTag, StoredParameter, VersionState,
};
use serde_json::json;

use super::{AddParameter, AddParameterInput, AddParameterOutput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, PARAMETER_TAG_UNIQUE, StorageError};
use crate::memory::fixtures::{
    Fixture, actor, field, lock_line, meta, mid, recipe_with, reference, rid, text, uid, user, vid,
};
use crate::memory::{DERIVED_TYPE_LOCK_TABLE, JournalEntry, LockMode};
use crate::ports::Isolation;
use crate::recipe::DraftOutput;
use crate::recipe::definition_steps::map_tag_error;
use crate::recipe::test_support::{
    DRAFT_VERSION, archive_unit, seed_deleted, seed_speed_draft, speed_key, stored_parameters,
};

fn limit(min: &str, max: &str) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some(min.to_owned()),
        max: Some(max.to_owned()),
        max_length: None,
    }
}

fn input(type_text: &str, value: &str, limits: Vec<LimitInput>) -> AddParameterInput {
    AddParameterInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        unit_path: UnitPath::parse("U1.2").unwrap(),
        tag: ParameterTag::parse("FEED").unwrap(),
        name_en: text("Feed"),
        description_en: recipe_domain::text::LongText::parse("Feed rate").unwrap(),
        type_expr: TypeExpr::parse(type_text).unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(1).unwrap(),
        value_json: value.to_owned(),
        limits: LimitList::parse(limits).unwrap(),
    }
}

fn int_input() -> AddParameterInput {
    input("INT", "120", vec![limit("10", "250")])
}

async fn run(
    fixture: &Fixture,
    request: AddParameterInput,
) -> Result<AddParameterOutput, AppError> {
    AddParameter::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
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
        lock_line("unit", LockMode::ForShare, uid(102).to_string()),
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
}

fn assert_nothing_added(fixture: &Fixture) {
    assert_nothing_written(fixture);
    assert_eq!(stored_parameters(fixture).len(), 1);
}

fn found(fixture: &Fixture, tag: &str) -> StoredParameter {
    stored_parameters(fixture)
        .into_iter()
        .find(|row| row.tag.as_str() == tag)
        .unwrap()
}

/// A draft of a machine on `platform` with no parameter.
fn empty_draft(platform: PlcPlatform) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(AddParameter::OPERATION, Operation::AddParameter);
}

#[tokio::test]
async fn success_adds_the_parameter_with_a_new_key_and_its_limit_row() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let output = run(&fixture, int_input()).await.unwrap();
    assert_eq!(
        output.draft,
        DraftOutput {
            version: vid(DRAFT_VERSION),
            row_version: 2
        }
    );
    assert_ne!(output.key, speed_key());
    assert_eq!(stored_parameters(&fixture).len(), 2);
    let added = found(&fixture, "FEED");
    assert_eq!(added.key, output.key);
    assert_eq!(added.unit, uid(102));
    assert_eq!(added.value.as_json(), &json!(120));
    assert_eq!(added.limits.len(), 1);
    assert_eq!(fixture.store.journal(), lock_prefix());
}

#[tokio::test]
async fn the_entry_is_a_row_create_with_the_limit_rows() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let engineer = actor(Role::ProcessEngineer);
    let request = meta();
    let output = AddParameter::new(fixture.context())
        .execute(&engineer, &request, int_input())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    let added = found(&fixture, "FEED");
    assert_eq!(entry.action(), AuditAction::ParameterAdded);
    assert_eq!(entry.target().entity_type(), "parameter");
    assert_eq!(entry.target().entity_id(), added.id.as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(entry.target().aggregate_id(), rid(1).as_uuid());
    assert_eq!(entry.before(), None);
    let after = entry.after().unwrap();
    assert_eq!(field(after, "tag"), json!("FEED"));
    assert_eq!(field(after, "parameter_key"), json!(output.key.to_string()));
    let limits = field(after, "limits");
    assert_eq!(limits.as_array().map(Vec::len), Some(1));
    assert_eq!(entry.actor(), Some(engineer.user()));
    assert_eq!(entry.request_id(), request.request_id());
    assert_eq!(
        fixture.store.state().editors_of(vid(DRAFT_VERSION)),
        vec![engineer.user()]
    );
}

#[tokio::test]
async fn an_array_of_sized_strings_takes_the_string_and_array_keys() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let request = input(
        "ARRAY[1..4] OF STRING(16)",
        r#"["a","b","c","d"]"#,
        Vec::new(),
    );
    run(&fixture, request).await.unwrap();
    let mut expected = lock_prefix();
    expected.extend([advisory("STRING"), advisory("ARRAY")]);
    assert_eq!(fixture.store.journal(), expected);
}

#[tokio::test]
async fn a_string_without_a_length_takes_no_advisory_key() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    run(&fixture, input("STRING", "\"text\"", Vec::new()))
        .await
        .unwrap();
    assert_eq!(fixture.store.journal(), lock_prefix());
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
}

#[tokio::test]
async fn a_new_sized_string_gives_a_derived_type_entry_before_the_parameter_entry() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    run(&fixture, input("STRING(20)", "\"text\"", Vec::new()))
        .await
        .unwrap();
    let actions: Vec<AuditAction> = fixture
        .store
        .audit_entries()
        .iter()
        .map(|entry| entry.action())
        .collect();
    assert_eq!(
        actions,
        vec![AuditAction::DerivedTypeCreated, AuditAction::ParameterAdded]
    );
    let mut expected = lock_prefix();
    expected.push(advisory("STRING"));
    assert_eq!(fixture.store.journal(), expected);
}

#[tokio::test]
async fn an_unknown_unit_path_gives_plt_020() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let mut request = int_input();
    request.unit_path = UnitPath::parse("U9").unwrap();
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_020);
    assert_nothing_added(&fixture);
}

#[tokio::test]
async fn an_archived_unit_gives_plt_009() {
    let fixture = Fixture::new();
    let mut machine = seed_speed_draft(&fixture);
    archive_unit(&mut machine, "U1.2.1");
    fixture.store.seed_machine(&machine);
    let mut request = int_input();
    request.unit_path = UnitPath::parse("U1.2.1").unwrap();
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_009);
    assert_nothing_added(&fixture);
}

#[tokio::test]
async fn a_unit_archived_between_the_read_and_the_locks_gives_plt_009() {
    let fixture = Fixture::new();
    let mut machine = seed_speed_draft(&fixture);
    let context = fixture.context_racing(move |store| {
        archive_unit(&mut machine, "U1.2");
        store.seed_machine(&machine);
    });
    let error = AddParameter::new(context)
        .execute(&actor(Role::ProcessEngineer), &meta(), int_input())
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_009);
    assert_nothing_added(&fixture);
}

#[tokio::test]
async fn a_wide_string_on_s7_300_gives_plt_013() {
    let fixture = empty_draft(PlcPlatform::S7_300);
    let error = run(&fixture, input("WSTRING(10)", "\"x\"", Vec::new()))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_unknown_struct_revision_gives_plt_013() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = run(&fixture, input("ST_Missing@1", "{}", Vec::new()))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert_nothing_added(&fixture);
}

#[tokio::test]
async fn a_type_of_another_platform_gives_plt_013() {
    let fixture = empty_draft(PlcPlatform::S7_300);
    let error = run(&fixture, input("LREAL", "1", Vec::new()))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_array_above_the_leaf_budget_gives_plt_018() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = run(&fixture, input("ARRAY[1..16385] OF INT", "[]", Vec::new()))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_018);
    assert_nothing_added(&fixture);
}

#[tokio::test]
async fn a_used_tag_gives_rcp_014() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let mut request = int_input();
    request.tag = ParameterTag::parse("SPEED").unwrap();
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_014);
    assert_nothing_added(&fixture);
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
async fn an_approved_version_gives_rcp_002() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let error = run(&fixture, int_input()).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_002);
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
    let error = AddParameter::new(context)
        .execute(&actor(Role::ProcessEngineer), &meta(), int_input())
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_002);
    assert_nothing_added(&fixture);
}

#[tokio::test]
async fn a_deleted_recipe_gives_rcp_012() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let error = run(&fixture, int_input()).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_old_row_version_gives_web_001_and_writes_nothing() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let mut request = int_input();
    request.expected_row_version = 7;
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_001);
    assert_nothing_added(&fixture);
}

#[tokio::test]
async fn an_operator_is_forbidden_and_opens_no_transaction() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = AddParameter::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), int_input())
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert_nothing_added(&fixture);
}
