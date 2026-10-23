//! Tests of design D20: the permission check is the first statement of every use case.
//!
//! An Operator has no permission for these operations. Each call returns `E-WEB-002` before it
//! opens a transaction. The sink then holds one `forbidden` event, and the store journal and
//! the audit log stay empty (ADR-0013). `ExportApprovedVersion` is not here, because every
//! role holds `VersionExport`. `purpose_tests.rs` holds the tests of the form queries (ADR-0046).

use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::SecurityEventKind;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{Login, Role};

use crate::audit::{
    AuditListInput, GetAuditEntry, ListAuditEntries, ListSecurityEvents, SecurityEventListInput,
};
use crate::auth::{CurrentUser, Operation};
use crate::context::UseCase;
use crate::error::AppError;
use crate::export::ExportVersionByNumber;
use crate::identity::fixtures::IdentityFixture;
use crate::identity::{
    ActivateUser, CreateUser, CreateUserInput, DeactivateUser, GetUser, GetUserForm, ListUsers,
    ListUsersInput, ResetPassword, SetUserRole, SetUserRoleInput, UnlockLogin,
};
use crate::memory::fixtures::scenarios;
use crate::memory::fixtures::{Fixture, actor, meta};
use crate::plant::{
    AddUnit, ArchiveMachine, ArchiveUnit, CreateMachine, DefineUdt, RenameMachine, RenameUnit,
};
use crate::ports::{AuditEntryId, AuditFilter, TimeRange};
use crate::recipe::{
    AddParameter, ApprovalPreviewInput, ApproveVersion, CreateRecipe, CreateVersion, DeleteRecipe,
    DiffChoice, DiscardDraft, GetApprovalPreview, ListDeletedRecipes, RebindTypeRevision,
    RecipeListInput, RemoveParameter, RemoveTranslation, RenameRecipe, ReplaceLimits,
    RestoreRecipe, RetireVersion, SetParameterValue, UpdateParameterDefinition, UpsertTranslation,
};

/// The Operator `mhoffman`, who has no recipe or plant permission.
pub(super) fn operator() -> CurrentUser {
    actor(Role::Operator)
}

/// Assert that `result` is the refusal of `operation` and that nothing else happened.
pub(super) fn assert_refused<T>(
    fixture: &Fixture,
    result: Result<T, AppError>,
    operation: Operation,
    name: &str,
) {
    let error = result.err();
    assert_eq!(
        error.as_ref().map(AppError::code),
        Some(ErrorCode::WEB_002),
        "{name}: error code"
    );
    assert!(
        matches!(error, Some(AppError::Forbidden { operation: found }) if found == operation),
        "{name}: refused operation"
    );
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{name}: one security event");
    assert!(
        events
            .first()
            .is_some_and(|event| event.kind() == SecurityEventKind::Forbidden),
        "{name}: forbidden event"
    );
    assert!(
        fixture.store.journal().is_empty(),
        "{name}: no transaction is opened"
    );
    assert!(
        fixture.store.audit_entries().is_empty(),
        "{name}: no audit entry"
    );
}

#[tokio::test]
async fn create_machine_checks_the_permission_first() {
    let fixture = Fixture::new();
    let result = CreateMachine::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::create_machine_input())
        .await;
    assert_refused(&fixture, result, CreateMachine::OPERATION, "CreateMachine");
}

#[tokio::test]
async fn rename_machine_checks_the_permission_first() {
    let fixture = scenarios::plant();
    let result = RenameMachine::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::rename_machine_input())
        .await;
    assert_refused(&fixture, result, RenameMachine::OPERATION, "RenameMachine");
}

#[tokio::test]
async fn archive_machine_checks_the_permission_first() {
    let fixture = scenarios::plant();
    let result = ArchiveMachine::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::archive_machine_input())
        .await;
    assert_refused(
        &fixture,
        result,
        ArchiveMachine::OPERATION,
        "ArchiveMachine",
    );
}

#[tokio::test]
async fn add_unit_checks_the_permission_first() {
    let fixture = scenarios::plant();
    let result = AddUnit::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::add_unit_input())
        .await;
    assert_refused(&fixture, result, AddUnit::OPERATION, "AddUnit");
}

#[tokio::test]
async fn rename_unit_checks_the_permission_first() {
    let fixture = scenarios::plant();
    let result = RenameUnit::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::rename_unit_input())
        .await;
    assert_refused(&fixture, result, RenameUnit::OPERATION, "RenameUnit");
}

#[tokio::test]
async fn archive_unit_checks_the_permission_first() {
    let fixture = scenarios::plant();
    let result = ArchiveUnit::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::archive_unit_input())
        .await;
    assert_refused(&fixture, result, ArchiveUnit::OPERATION, "ArchiveUnit");
}

#[tokio::test]
async fn define_udt_checks_the_permission_first() {
    let fixture = scenarios::plant();
    let result = DefineUdt::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::define_udt_input())
        .await;
    assert_refused(&fixture, result, DefineUdt::OPERATION, "DefineUdt");
}

#[tokio::test]
async fn create_recipe_checks_the_permission_first() {
    let fixture = scenarios::plant();
    let result = CreateRecipe::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::create_recipe_input())
        .await;
    assert_refused(&fixture, result, CreateRecipe::OPERATION, "CreateRecipe");
}

#[tokio::test]
async fn rename_recipe_checks_the_permission_first() {
    let fixture = scenarios::draft();
    let result = RenameRecipe::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::rename_recipe_input())
        .await;
    assert_refused(&fixture, result, RenameRecipe::OPERATION, "RenameRecipe");
}

#[tokio::test]
async fn delete_recipe_checks_the_permission_first() {
    let fixture = scenarios::draft();
    let result = DeleteRecipe::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::delete_recipe_input())
        .await;
    assert_refused(&fixture, result, DeleteRecipe::OPERATION, "DeleteRecipe");
}

#[tokio::test]
async fn restore_recipe_checks_the_permission_first() {
    let fixture = scenarios::deleted();
    let result = RestoreRecipe::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::restore_recipe_input())
        .await;
    assert_refused(&fixture, result, RestoreRecipe::OPERATION, "RestoreRecipe");
}

#[tokio::test]
async fn list_deleted_recipes_checks_the_permission_first() {
    let fixture = scenarios::deleted();
    let input = RecipeListInput {
        text: None,
        machine: None,
        status: None,
        page: 0,
    };
    let result = ListDeletedRecipes::new(fixture.context())
        .execute(&operator(), &meta(), input)
        .await;
    assert_refused(
        &fixture,
        result,
        ListDeletedRecipes::OPERATION,
        "ListDeletedRecipes",
    );
}

#[tokio::test]
async fn create_version_checks_the_permission_first() {
    let fixture = scenarios::retired();
    let result = CreateVersion::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::create_version_input())
        .await;
    assert_refused(&fixture, result, CreateVersion::OPERATION, "CreateVersion");
}

#[tokio::test]
async fn discard_draft_checks_the_permission_first() {
    let fixture = scenarios::draft();
    let result = DiscardDraft::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::discard_draft_input())
        .await;
    assert_refused(&fixture, result, DiscardDraft::OPERATION, "DiscardDraft");
}

#[tokio::test]
async fn approve_version_checks_the_permission_first() {
    let fixture = scenarios::approved_and_draft();
    let result = ApproveVersion::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::approve_input(2))
        .await;
    assert_refused(
        &fixture,
        result,
        ApproveVersion::OPERATION,
        "ApproveVersion",
    );
}

#[tokio::test]
async fn get_approval_preview_checks_the_permission_first() {
    let fixture = scenarios::approved_and_draft();
    let input = ApprovalPreviewInput {
        reference: scenarios::approve_input(2).reference,
        number: 2,
        choice: DiffChoice {
            kind: None,
            page: 0,
        },
    };
    let result = GetApprovalPreview::new(fixture.context())
        .execute(&operator(), &meta(), input)
        .await;
    assert_refused(
        &fixture,
        result,
        GetApprovalPreview::OPERATION,
        "GetApprovalPreview",
    );
}

#[tokio::test]
async fn retire_version_checks_the_permission_first() {
    let fixture = scenarios::retired();
    let result = RetireVersion::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::retire_version_input())
        .await;
    assert_refused(&fixture, result, RetireVersion::OPERATION, "RetireVersion");
}

#[tokio::test]
async fn add_parameter_checks_the_permission_first() {
    let fixture = scenarios::draft();
    let result = AddParameter::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::add_parameter_input())
        .await;
    assert_refused(&fixture, result, AddParameter::OPERATION, "AddParameter");
}

#[tokio::test]
async fn update_parameter_definition_checks_the_permission_first() {
    let fixture = scenarios::draft();
    let result = UpdateParameterDefinition::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::update_definition_input())
        .await;
    assert_refused(
        &fixture,
        result,
        UpdateParameterDefinition::OPERATION,
        "UpdateParameterDefinition",
    );
}

#[tokio::test]
async fn set_parameter_value_checks_the_permission_first() {
    let fixture = scenarios::draft();
    let result = SetParameterValue::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::set_value_input())
        .await;
    assert_refused(
        &fixture,
        result,
        SetParameterValue::OPERATION,
        "SetParameterValue",
    );
}

#[tokio::test]
async fn replace_limits_checks_the_permission_first() {
    let fixture = scenarios::draft();
    let result = ReplaceLimits::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::replace_limits_input())
        .await;
    assert_refused(&fixture, result, ReplaceLimits::OPERATION, "ReplaceLimits");
}

#[tokio::test]
async fn upsert_translation_checks_the_permission_first() {
    let fixture = scenarios::draft();
    let result = UpsertTranslation::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::upsert_translation_input())
        .await;
    assert_refused(
        &fixture,
        result,
        UpsertTranslation::OPERATION,
        "UpsertTranslation",
    );
}

#[tokio::test]
async fn remove_translation_checks_the_permission_first() {
    let fixture = scenarios::draft();
    let result = RemoveTranslation::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::remove_translation_input())
        .await;
    assert_refused(
        &fixture,
        result,
        RemoveTranslation::OPERATION,
        "RemoveTranslation",
    );
}

#[tokio::test]
async fn remove_parameter_checks_the_permission_first() {
    let fixture = scenarios::draft();
    let result = RemoveParameter::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::remove_parameter_input())
        .await;
    assert_refused(
        &fixture,
        result,
        RemoveParameter::OPERATION,
        "RemoveParameter",
    );
}

#[tokio::test]
async fn rebind_type_revision_checks_the_permission_first() {
    let fixture = scenarios::zone_draft();
    let result = RebindTypeRevision::new(fixture.context())
        .execute(&operator(), &meta(), scenarios::rebind_input())
        .await;
    assert_refused(
        &fixture,
        result,
        RebindTypeRevision::OPERATION,
        "RebindTypeRevision",
    );
}

#[tokio::test]
async fn export_version_by_number_checks_the_permission_first() {
    let fixture = scenarios::retired();
    let result = ExportVersionByNumber::new(fixture.context(), scenarios::body_writer())
        .execute(&operator(), &meta(), scenarios::export_number_input())
        .await;
    assert_refused(
        &fixture,
        result,
        ExportVersionByNumber::OPERATION,
        "ExportVersionByNumber",
    );
}

#[tokio::test]
async fn list_audit_entries_checks_the_permission_first() {
    let fixture = Fixture::new();
    let input = AuditListInput {
        filter: AuditFilter::default(),
        before: None,
    };
    let result = ListAuditEntries::new(fixture.context())
        .execute(&operator(), &meta(), input)
        .await;
    assert_refused(
        &fixture,
        result,
        ListAuditEntries::OPERATION,
        "ListAuditEntries",
    );
}

#[tokio::test]
async fn get_audit_entry_checks_the_permission_first() {
    let fixture = Fixture::new();
    let id = AuditEntryId::from(std::num::NonZeroU64::MIN);
    let result = GetAuditEntry::new(fixture.context())
        .execute(&operator(), &meta(), id)
        .await;
    assert_refused(&fixture, result, GetAuditEntry::OPERATION, "GetAuditEntry");
}

#[tokio::test]
async fn retire_version_is_refused_for_a_process_engineer() {
    let fixture = scenarios::retired();
    let engineer = actor(Role::ProcessEngineer);
    let result = RetireVersion::new(fixture.context())
        .execute(&engineer, &meta(), scenarios::retire_version_input())
        .await;
    assert_refused(
        &fixture,
        result,
        RetireVersion::OPERATION,
        "RetireVersion (ProcessEngineer)",
    );
}

#[tokio::test]
async fn restore_recipe_is_refused_for_a_process_engineer() {
    let fixture = scenarios::deleted();
    let engineer = actor(Role::ProcessEngineer);
    let result = RestoreRecipe::new(fixture.context())
        .execute(&engineer, &meta(), scenarios::restore_recipe_input())
        .await;
    assert_refused(
        &fixture,
        result,
        RestoreRecipe::OPERATION,
        "RestoreRecipe (ProcessEngineer)",
    );
}

/// Assert that an identity use case refused `operation` before it opened a transaction.
///
/// `mark` is the journal length after the actor is built, because building the actor journals
/// one session lookup. No new journal line may follow it.
fn assert_identity_refused<T>(
    fixture: &IdentityFixture,
    mark: usize,
    result: Result<T, AppError>,
    operation: Operation,
    name: &str,
) {
    let error = result.err();
    assert_eq!(
        error.as_ref().map(AppError::code),
        Some(ErrorCode::WEB_002),
        "{name}: error code"
    );
    assert!(
        matches!(error, Some(AppError::Forbidden { operation: found }) if found == operation),
        "{name}: refused operation"
    );
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1, "{name}: one security event");
    assert!(
        events
            .first()
            .is_some_and(|event| event.kind() == SecurityEventKind::Forbidden),
        "{name}: forbidden event"
    );
    assert_eq!(
        fixture.store.journal().len(),
        mark,
        "{name}: no transaction is opened"
    );
    assert!(
        fixture.store.audit_entries().is_empty(),
        "{name}: no audit entry"
    );
}

/// Return the login `text` as a value object.
fn login(text: &str) -> Login {
    Login::parse(text).unwrap()
}

/// The client address of the identity calls.
fn client() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9))
}

#[tokio::test]
async fn create_user_checks_the_permission_first() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    let (auth, request) = fixture.actor(Role::Operator).await;
    let mark = fixture.store.journal().len();
    let input = CreateUserInput {
        login: login("berta"),
        role: Role::Operator,
    };
    let result = CreateUser::new(fixture.context.clone())
        .execute(&auth, &request, input)
        .await;
    assert_identity_refused(&fixture, mark, result, CreateUser::OPERATION, "CreateUser");
}

#[tokio::test]
async fn set_user_role_checks_the_permission_first() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    let (auth, request) = fixture.actor(Role::Operator).await;
    let mark = fixture.store.journal().len();
    let input = SetUserRoleInput {
        login: login("anna"),
        role: Role::Administrator,
    };
    let result = SetUserRole::new(fixture.context.clone())
        .execute(&auth, &request, input)
        .await;
    assert_identity_refused(
        &fixture,
        mark,
        result,
        SetUserRole::OPERATION,
        "SetUserRole",
    );
}

#[tokio::test]
async fn deactivate_user_checks_the_permission_first() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    let (auth, request) = fixture.actor(Role::Operator).await;
    let mark = fixture.store.journal().len();
    let result = DeactivateUser::new(fixture.context.clone())
        .execute(&auth, &request, &login("anna"))
        .await;
    assert_identity_refused(
        &fixture,
        mark,
        result,
        DeactivateUser::OPERATION,
        "DeactivateUser",
    );
}

#[tokio::test]
async fn activate_user_checks_the_permission_first() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    let (auth, request) = fixture.actor(Role::Operator).await;
    let mark = fixture.store.journal().len();
    let result = ActivateUser::new(fixture.context.clone())
        .execute(&auth, &request, &login("anna"))
        .await;
    assert_identity_refused(
        &fixture,
        mark,
        result,
        ActivateUser::OPERATION,
        "ActivateUser",
    );
}

#[tokio::test]
async fn reset_password_checks_the_permission_first() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    let (auth, request) = fixture.actor(Role::Operator).await;
    let mark = fixture.store.journal().len();
    let result = ResetPassword::new(fixture.context.clone())
        .execute(&auth, &request, &login("anna"))
        .await;
    assert_identity_refused(
        &fixture,
        mark,
        result,
        ResetPassword::OPERATION,
        "ResetPassword",
    );
}

#[tokio::test]
async fn unlock_login_checks_the_permission_first() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    let (auth, request) = fixture.actor(Role::Operator).await;
    let mark = fixture.store.journal().len();
    let result = UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &request, &login("anna"), client())
        .await;
    assert_identity_refused(
        &fixture,
        mark,
        result,
        UnlockLogin::OPERATION,
        "UnlockLogin",
    );
}

#[tokio::test]
async fn list_users_checks_the_permission_first() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    let (auth, request) = fixture.actor(Role::Operator).await;
    let mark = fixture.store.journal().len();
    let input = ListUsersInput { q: None, page: 0 };
    let result = ListUsers::new(fixture.context.clone())
        .execute(&auth, &request, input)
        .await;
    assert_identity_refused(&fixture, mark, result, ListUsers::OPERATION, "ListUsers");
}

#[tokio::test]
async fn get_user_checks_the_permission_first() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    let (auth, request) = fixture.actor(Role::Operator).await;
    let mark = fixture.store.journal().len();
    let result = GetUser::new(fixture.context.clone())
        .execute(&auth, &request, &login("anna"))
        .await;
    assert_identity_refused(&fixture, mark, result, GetUser::OPERATION, "GetUser");
}

#[tokio::test]
async fn list_security_events_is_refused_for_a_process_engineer_before_a_read() {
    let fixture = IdentityFixture::new();
    let (auth, request) = fixture.actor(Role::ProcessEngineer).await;
    let mark = fixture.store.journal().len();
    let input = SecurityEventListInput {
        kind: None,
        login: None,
        client_ip: None,
        range: TimeRange::default(),
        before: None,
    };
    let result = ListSecurityEvents::new(fixture.context.clone())
        .execute(&auth, &request, input)
        .await;
    assert_identity_refused(
        &fixture,
        mark,
        result,
        ListSecurityEvents::OPERATION,
        "ListSecurityEvents (ProcessEngineer)",
    );
}

#[tokio::test]
async fn get_user_form_checks_the_permission_first() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    let (auth, request) = fixture.actor(Role::Operator).await;
    let mark = fixture.store.journal().len();
    let result = GetUserForm::new(fixture.context.clone())
        .execute(&auth, &request)
        .await;
    assert_identity_refused(
        &fixture,
        mark,
        result,
        GetUserForm::OPERATION,
        "GetUserForm",
    );
}
