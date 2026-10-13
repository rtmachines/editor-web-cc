//! Tests of the audit coverage of every use case (`data-model.md` sections 6 and 10).
//!
//! Each use case runs its success path once. The audit log then holds the actions of its row
//! of section 6, in order. Each entry names the table and the aggregate root of section 10.
//! Each entry also names the acting user, the request, and a state.

use recipe_domain::audit::{AggregateType, AuditAction, AuditEntry};
use recipe_domain::identity::Role;

use crate::auth::{CurrentUser, RequestMeta};
use crate::export::{ExportApprovedVersion, ExportVersionByNumber};
use crate::memory::fixtures::scenarios;
use crate::memory::fixtures::{Fixture, actor, meta, second_engineer};
use crate::plant::{
    AddUnit, ArchiveMachine, ArchiveUnit, CreateMachine, DefineUdt, RenameMachine, RenameUnit,
};
use crate::recipe::{
    AddParameter, ApproveVersion, CreateRecipe, CreateVersion, DeleteRecipe, DiscardDraft,
    RebindTypeRevision, RemoveParameter, RemoveTranslation, RenameRecipe, ReplaceLimits,
    RestoreRecipe, RetireVersion, SetParameterValue, UpdateParameterDefinition, UpsertTranslation,
};

/// One expected audit entry: the action, the table of the changed row, and the aggregate root.
type Expected = (AuditAction, &'static str, AggregateType);

/// Return the entries of the log after the first `skip` entries.
///
/// A test that writes after a seeding command, such as an approval, skips the entries before it.
fn entries_after(fixture: &Fixture, skip: usize) -> Vec<AuditEntry> {
    fixture
        .store
        .audit_entries()
        .into_iter()
        .skip(skip)
        .collect()
}

/// Assert the actions of `entries` in order, and the actor, the request, and the state of each.
fn assert_trail(
    entries: &[AuditEntry],
    user: &CurrentUser,
    request: &RequestMeta,
    name: &str,
    expected: &[Expected],
) {
    let actual: Vec<Expected> = entries
        .iter()
        .map(|entry| {
            (
                entry.action(),
                entry.target().entity_type(),
                entry.target().aggregate_type(),
            )
        })
        .collect();
    assert_eq!(actual, expected, "{name}: audit actions");
    for entry in entries {
        assert_eq!(entry.actor(), Some(user.user()), "{name}: actor");
        assert_eq!(
            entry.request_id(),
            request.request_id(),
            "{name}: request id"
        );
        assert!(
            entry.before().is_some() || entry.after().is_some(),
            "{name}: state"
        );
    }
}

#[tokio::test]
async fn create_machine_writes_the_machine_then_its_main_unit() {
    let fixture = Fixture::new();
    let user = actor(Role::Administrator);
    let request = meta();
    CreateMachine::new(fixture.context())
        .execute(&user, &request, scenarios::create_machine_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "CreateMachine",
        &[
            (
                AuditAction::MachineCreated,
                "machine",
                AggregateType::Machine,
            ),
            (AuditAction::UnitAdded, "unit", AggregateType::Machine),
        ],
    );
}

#[tokio::test]
async fn rename_machine_writes_one_machine_entry() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    RenameMachine::new(fixture.context())
        .execute(&user, &request, scenarios::rename_machine_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "RenameMachine",
        &[(
            AuditAction::MachineRenamed,
            "machine",
            AggregateType::Machine,
        )],
    );
}

#[tokio::test]
async fn archive_machine_writes_one_machine_entry() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    ArchiveMachine::new(fixture.context())
        .execute(&user, &request, scenarios::archive_machine_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "ArchiveMachine",
        &[(
            AuditAction::MachineArchived,
            "machine",
            AggregateType::Machine,
        )],
    );
}

#[tokio::test]
async fn add_unit_writes_one_unit_entry() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    AddUnit::new(fixture.context())
        .execute(&user, &request, scenarios::add_unit_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "AddUnit",
        &[(AuditAction::UnitAdded, "unit", AggregateType::Machine)],
    );
}

#[tokio::test]
async fn rename_unit_writes_one_unit_entry() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    RenameUnit::new(fixture.context())
        .execute(&user, &request, scenarios::rename_unit_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "RenameUnit",
        &[(AuditAction::UnitRenamed, "unit", AggregateType::Machine)],
    );
}

#[tokio::test]
async fn archive_unit_writes_one_entry_per_archived_unit() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    ArchiveUnit::new(fixture.context())
        .execute(&user, &request, scenarios::archive_unit_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "ArchiveUnit",
        &[
            (AuditAction::UnitArchived, "unit", AggregateType::Machine),
            (AuditAction::UnitArchived, "unit", AggregateType::Machine),
        ],
    );
}

#[tokio::test]
async fn define_udt_writes_the_derived_type_before_the_struct() {
    let fixture = scenarios::plant();
    let user = actor(Role::Administrator);
    let request = meta();
    DefineUdt::new(fixture.context())
        .execute(&user, &request, scenarios::define_udt_input())
        .await
        .unwrap();
    assert_trail(
        &fixture.store.audit_entries(),
        &user,
        &request,
        "DefineUdt",
        &[
            (
                AuditAction::DerivedTypeCreated,
                "plc_type",
                AggregateType::Machine,
            ),
            (AuditAction::UdtDefined, "plc_type", AggregateType::Machine),
        ],
    );
}

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
    ExportApprovedVersion::new(fixture.context())
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
    ExportVersionByNumber::new(fixture.context())
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
