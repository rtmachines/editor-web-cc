//! Audit coverage of the recipe, version, parameter, and export use cases.

use recipe_domain::audit::{AggregateType, AuditAction};
use recipe_domain::identity::Role;

use super::{assert_trail, entries_after};
use crate::export::{ExportApprovedVersion, ExportVersionByNumber};
use crate::memory::fixtures::scenarios;
use crate::memory::fixtures::{actor, meta, second_engineer};
use crate::recipe::{
    AddParameter, ApproveVersion, CreateRecipe, CreateVersion, DeleteRecipe, DiscardDraft,
    RebindTypeRevision, RemoveParameter, RemoveTranslation, RenameRecipe, ReplaceLimits,
    RestoreRecipe, RetireVersion, SetParameterValue, UpdateParameterDefinition, UpsertTranslation,
};

#[tokio::test]
async fn create_recipe_writes_the_recipe_then_its_first_version() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    CreateRecipe::new(fixture.context())
        .execute(&user, &request, scenarios::create_recipe_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "CreateRecipe",
        &[
            (AuditAction::RecipeCreated, "recipe", AggregateType::Recipe),
            (
                AuditAction::VersionCreated,
                "recipe_version",
                AggregateType::Recipe,
            ),
        ],
    );
}

#[tokio::test]
async fn rename_recipe_writes_one_recipe_entry() {
    let fixture = scenarios::draft();
    let user = actor(Role::Administrator);
    let request = meta();
    RenameRecipe::new(fixture.context())
        .execute(&user, &request, scenarios::rename_recipe_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "RenameRecipe",
        &[(AuditAction::RecipeRenamed, "recipe", AggregateType::Recipe)],
    );
}

#[tokio::test]
async fn delete_recipe_discards_the_draft_then_deletes_the_recipe() {
    let fixture = scenarios::draft();
    let user = actor(Role::Administrator);
    let request = meta();
    DeleteRecipe::new(fixture.context())
        .execute(&user, &request, scenarios::delete_recipe_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "DeleteRecipe",
        &[
            (
                AuditAction::DraftDiscarded,
                "recipe_version",
                AggregateType::Recipe,
            ),
            (AuditAction::RecipeDeleted, "recipe", AggregateType::Recipe),
        ],
    );
}

#[tokio::test]
async fn restore_recipe_writes_one_recipe_entry() {
    let fixture = scenarios::deleted();
    let user = actor(Role::Administrator);
    let request = meta();
    RestoreRecipe::new(fixture.context())
        .execute(&user, &request, scenarios::restore_recipe_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "RestoreRecipe",
        &[(AuditAction::RecipeRestored, "recipe", AggregateType::Recipe)],
    );
}

#[tokio::test]
async fn create_version_writes_one_version_entry() {
    let fixture = scenarios::retired();
    let user = actor(Role::Administrator);
    let request = meta();
    CreateVersion::new(fixture.context())
        .execute(&user, &request, scenarios::create_version_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "CreateVersion",
        &[(
            AuditAction::VersionCreated,
            "recipe_version",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn discard_draft_writes_one_version_entry() {
    let fixture = scenarios::draft();
    let user = actor(Role::Administrator);
    let request = meta();
    DiscardDraft::new(fixture.context())
        .execute(&user, &request, scenarios::discard_draft_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "DiscardDraft",
        &[(
            AuditAction::DraftDiscarded,
            "recipe_version",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn approve_version_retires_the_previous_approved_version_first() {
    let fixture = scenarios::approved_and_draft();
    let user = second_engineer();
    let request = meta();
    ApproveVersion::new(fixture.context())
        .execute(&user, &request, scenarios::approve_input(2))
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "ApproveVersion",
        &[
            (
                AuditAction::VersionRetired,
                "recipe_version",
                AggregateType::Recipe,
            ),
            (
                AuditAction::VersionApproved,
                "recipe_version",
                AggregateType::Recipe,
            ),
        ],
    );
}

#[tokio::test]
async fn retire_version_writes_one_version_entry() {
    let fixture = scenarios::retired();
    let user = actor(Role::Administrator);
    let request = meta();
    RetireVersion::new(fixture.context())
        .execute(&user, &request, scenarios::retire_version_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "RetireVersion",
        &[(
            AuditAction::VersionRetired,
            "recipe_version",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn add_parameter_writes_one_parameter_entry() {
    let fixture = scenarios::draft();
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    AddParameter::new(fixture.context())
        .execute(&user, &request, scenarios::add_parameter_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "AddParameter",
        &[(
            AuditAction::ParameterAdded,
            "parameter",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn update_parameter_definition_writes_one_parameter_entry() {
    let fixture = scenarios::draft();
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    UpdateParameterDefinition::new(fixture.context())
        .execute(&user, &request, scenarios::update_definition_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "UpdateParameterDefinition",
        &[(
            AuditAction::ParameterDefinitionUpdated,
            "parameter",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn set_parameter_value_writes_one_parameter_entry() {
    let fixture = scenarios::draft();
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    SetParameterValue::new(fixture.context())
        .execute(&user, &request, scenarios::set_value_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "SetParameterValue",
        &[(
            AuditAction::ParameterValueSet,
            "parameter",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn replace_limits_writes_one_parameter_entry() {
    let fixture = scenarios::draft();
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    ReplaceLimits::new(fixture.context())
        .execute(&user, &request, scenarios::replace_limits_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "ReplaceLimits",
        &[(
            AuditAction::LimitsReplaced,
            "parameter",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn upsert_translation_writes_one_parameter_entry() {
    let fixture = scenarios::draft();
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    UpsertTranslation::new(fixture.context())
        .execute(&user, &request, scenarios::upsert_translation_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "UpsertTranslation",
        &[(
            AuditAction::TranslationUpserted,
            "parameter",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn remove_translation_writes_one_parameter_entry() {
    let fixture = scenarios::draft();
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    RemoveTranslation::new(fixture.context())
        .execute(&user, &request, scenarios::remove_translation_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "RemoveTranslation",
        &[(
            AuditAction::TranslationRemoved,
            "parameter",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn remove_parameter_writes_one_parameter_entry() {
    let fixture = scenarios::draft();
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    RemoveParameter::new(fixture.context())
        .execute(&user, &request, scenarios::remove_parameter_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "RemoveParameter",
        &[(
            AuditAction::ParameterRemoved,
            "parameter",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn rebind_type_revision_writes_one_parameter_entry() {
    let fixture = scenarios::zone_draft();
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    RebindTypeRevision::new(fixture.context())
        .execute(&user, &request, scenarios::rebind_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "RebindTypeRevision",
        &[(
            AuditAction::TypeRevisionRebound,
            "parameter",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn export_approved_version_writes_one_export_entry() {
    let fixture = scenarios::approved_version().await;
    let skip = fixture.store.audit_entries().len();
    let user = actor(Role::Operator);
    let request = meta();
    ExportApprovedVersion::new(fixture.context(), scenarios::body_writer())
        .execute(&user, &request, scenarios::export_approved_input())
        .await
        .unwrap();
    assert_trail(
        &entries_after(&fixture, skip),
        &user,
        &request,
        "ExportApprovedVersion",
        &[(
            AuditAction::VersionExported,
            "recipe_version",
            AggregateType::Recipe,
        )],
    );
}

#[tokio::test]
async fn export_version_by_number_writes_one_export_entry() {
    let fixture = scenarios::approved_version().await;
    let skip = fixture.store.audit_entries().len();
    let user = actor(Role::Administrator);
    let request = meta();
    ExportVersionByNumber::new(fixture.context(), scenarios::body_writer())
        .execute(&user, &request, scenarios::export_number_input())
        .await
        .unwrap();
    assert_trail(
        &entries_after(&fixture, skip),
        &user,
        &request,
        "ExportVersionByNumber",
        &[(
            AuditAction::VersionExported,
            "recipe_version",
            AggregateType::Recipe,
        )],
    );
}
