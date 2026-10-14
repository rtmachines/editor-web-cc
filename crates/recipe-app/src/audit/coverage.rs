//! Tests of the audit coverage of every use case (`data-model.md` sections 6 and 10).
//!
//! Each use case runs its success path once. The audit log then holds the actions of its row
//! of section 6, in order. Each entry names the table and the aggregate root of section 10.
//! Each entry also names the acting user, the request, and a state.

use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::{AggregateType, AuditAction, AuditEntry, RequestId};
use recipe_domain::identity::{Login as UserLogin, Password, Role, TypedLogin};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::{CurrentUser, RequestMeta};
use crate::export::{ExportApprovedVersion, ExportVersionByNumber};
use crate::identity::fixtures::{IdentityFixture, anna_id};
use crate::identity::{
    ActivateUser, Authenticated, ChangeOwnPassword, ChangeOwnPasswordInput, CreateUser,
    CreateUserInput, DeactivateUser, Login, LoginInput, Logout, ResetPassword, SetUserRole,
    SetUserRoleInput, UnlockLogin,
};
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

/// The password of the fixture user `anna`.
const PASSWORD: &str = "correct-horse-battery";
/// The start of the clock of a fixture, in Unix seconds.
const START_SECONDS: i64 = 1_000_000;
/// The new password of the change of `anna`.
const NEW_PASSWORD: &str = "brand-new-secret-phrase";

/// Return the client address of the identity calls.
fn client() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))
}

/// Return the login `anna` as a domain value.
fn anna_login() -> UserLogin {
    UserLogin::parse("anna").unwrap()
}

/// Return a fixture with the active Operator `anna`, who has the password `PASSWORD`.
fn fixture_with_anna() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.seed_anna(PASSWORD, false);
    fixture
}

/// Build the request of the session that `auth` holds.
fn session_meta(auth: &Authenticated) -> RequestMeta {
    RequestMeta::new(
        RequestId::from(Uuid::from_u128(0x88)),
        Some(client()),
        None,
        Some(auth.session_ref().clone()),
    )
}

/// Assert that no image of an audit entry holds the key `password_hash`.
fn assert_no_password_hash(entries: &[AuditEntry], name: &str) {
    for entry in entries {
        for image in [entry.before(), entry.after()].into_iter().flatten() {
            assert!(
                image.get("password_hash").is_none(),
                "{name}: password_hash in an image"
            );
        }
    }
}

#[tokio::test]
async fn login_writes_user_logged_in_for_the_user_that_logs_in() {
    let fixture = fixture_with_anna();
    let request = RequestMeta::new(RequestId::from(Uuid::from_u128(0x77)), None, None, None);
    let input = LoginInput {
        login: TypedLogin::parse("anna").unwrap(),
        password: Password::parse(PASSWORD).unwrap(),
        client_ip: client(),
        old_session: None,
    };
    Login::new(fixture.context.clone())
        .execute(&request, input)
        .await
        .unwrap();
    let anna = CurrentUser::new(anna_id(), anna_login(), Role::Operator);
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        &anna,
        &request,
        "Login",
        &[(
            AuditAction::UserLoggedIn,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "Login");
}

#[tokio::test]
async fn logout_writes_user_logged_out() {
    let fixture = fixture_with_anna();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let request = session_meta(&auth);
    Logout::new(fixture.context.clone())
        .execute(&auth, &request)
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "Logout",
        &[(
            AuditAction::UserLoggedOut,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "Logout");
}

#[tokio::test]
async fn create_user_writes_user_created() {
    let fixture = IdentityFixture::new();
    let (auth, request) = fixture.actor(Role::Administrator).await;
    let input = CreateUserInput {
        login: UserLogin::parse("berta").unwrap(),
        role: Role::Operator,
    };
    CreateUser::new(fixture.context.clone())
        .execute(&auth, &request, input)
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "CreateUser",
        &[(AuditAction::UserCreated, "app_user", AggregateType::AppUser)],
    );
    assert_no_password_hash(&entries, "CreateUser");
}

#[tokio::test]
async fn set_user_role_writes_user_role_set() {
    let fixture = fixture_with_anna();
    let (auth, request) = fixture.actor(Role::Administrator).await;
    let input = SetUserRoleInput {
        login: anna_login(),
        role: Role::ProcessEngineer,
    };
    SetUserRole::new(fixture.context.clone())
        .execute(&auth, &request, input)
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "SetUserRole",
        &[(AuditAction::UserRoleSet, "app_user", AggregateType::AppUser)],
    );
    assert_no_password_hash(&entries, "SetUserRole");
}

#[tokio::test]
async fn deactivate_user_writes_user_deactivated() {
    let fixture = fixture_with_anna();
    let (auth, request) = fixture.actor(Role::Administrator).await;
    DeactivateUser::new(fixture.context.clone())
        .execute(&auth, &request, &anna_login())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "DeactivateUser",
        &[(
            AuditAction::UserDeactivated,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "DeactivateUser");
}

#[tokio::test]
async fn activate_user_writes_user_activated() {
    let fixture = fixture_with_anna();
    fixture.store.update_state(|state, now| {
        let row = state.users.get_mut(&anna_id()).unwrap();
        row.is_active = false;
        row.deactivated_at = Some(now);
        row.session_epoch = 3;
    });
    let (auth, request) = fixture.actor(Role::Administrator).await;
    ActivateUser::new(fixture.context.clone())
        .execute(&auth, &request, &anna_login())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "ActivateUser",
        &[(
            AuditAction::UserActivated,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "ActivateUser");
}

#[tokio::test]
async fn reset_password_writes_password_reset() {
    let fixture = fixture_with_anna();
    let (auth, request) = fixture.actor(Role::Administrator).await;
    ResetPassword::new(fixture.context.clone())
        .execute(&auth, &request, &anna_login())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "ResetPassword",
        &[(
            AuditAction::PasswordReset,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "ResetPassword");
}

#[tokio::test]
async fn change_own_password_writes_own_password_changed() {
    let fixture = fixture_with_anna();
    let token = fixture.seed_anna_session(1);
    let auth = fixture.authenticated(&token).await;
    let request = session_meta(&auth);
    let input = ChangeOwnPasswordInput {
        client_ip: client(),
        current_password: Password::parse(PASSWORD).unwrap(),
        new_password: Password::parse(NEW_PASSWORD).unwrap(),
    };
    ChangeOwnPassword::new(fixture.context.clone())
        .execute(&auth, &request, input)
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "ChangeOwnPassword",
        &[(
            AuditAction::OwnPasswordChanged,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "ChangeOwnPassword");
}

#[tokio::test]
async fn unlock_login_writes_login_unlocked() {
    let fixture = fixture_with_anna();
    for offset in [0_i64, 1] {
        let seconds = START_SECONDS.checked_add(offset).unwrap();
        let at = Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap());
        fixture
            .store
            .seed_attempt(&TypedLogin::parse("anna").unwrap(), client(), at, false);
    }
    let (auth, request) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &request, &anna_login(), client())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_trail(
        &entries,
        auth.user(),
        &request,
        "UnlockLogin",
        &[(
            AuditAction::LoginUnlocked,
            "app_user",
            AggregateType::AppUser,
        )],
    );
    assert_no_password_hash(&entries, "UnlockLogin");
    let listed = entries
        .first()
        .and_then(|entry| entry.before())
        .and_then(|image| image.get("login_attempts"))
        .and_then(|value| value.as_array());
    assert_eq!(
        listed.map(Vec::len),
        Some(2),
        "UnlockLogin: listed attempts"
    );
}
