//! Tests of `ListVersions` and `GetVersionForm` (design D4, ADR-0043 (e) and (h)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::VersionState::{self, Approved, LiveDraft, Retired};

use super::test_world::{seed_recipe, seed_users};
use super::versions::{
    GetVersionForm, GetVersionFormInput, ListVersions, ListVersionsInput, VersionForm, VersionList,
};
use crate::auth::Operation;
use crate::blocking::{AppSettings, PageSizes};
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::JournalEntry;
use crate::memory::fixtures::{Fixture, actor, meta, reference, scenarios};
use crate::ports::Isolation;

/// A fixture with recipe `R-1` whose versions have the given states.
fn recipe_with_states(states: &[VersionState]) -> Fixture {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_recipe(&fixture, &machine, (1, "R-1"), states);
    fixture
}

async fn versions(
    fixture: &Fixture,
    role: Role,
    (name, page): (&str, u32),
) -> Result<VersionList, AppError> {
    let settings = AppSettings {
        pages: PageSizes {
            versions: 2,
            ..PageSizes::default()
        },
        ..AppSettings::default()
    };
    ListVersions::new(fixture.context_with(settings))
        .execute(
            &actor(role),
            &meta(),
            ListVersionsInput {
                reference: reference(name),
                page,
            },
        )
        .await
}

async fn form(fixture: &Fixture, role: Role, name: &str) -> Result<VersionForm, AppError> {
    GetVersionForm::new(fixture.context())
        .execute(
            &actor(role),
            &meta(),
            GetVersionFormInput {
                purpose: crate::auth::VersionFormPurpose::CreateVersion,
                reference: reference(name),
            },
        )
        .await
}

fn numbers(list: &VersionList) -> Vec<u32> {
    list.rows.iter().map(|row| row.number).collect()
}

#[test]
fn the_queries_declare_their_operations() {
    assert_eq!(ListVersions::OPERATION, Operation::ListVersions);
    assert_eq!(GetVersionForm::OPERATION, Operation::GetVersionForm);
}

#[tokio::test]
async fn list_versions_returns_the_newest_versions_first_one_page_at_a_time() {
    let fixture = recipe_with_states(&[Retired, Retired, Approved, LiveDraft]);

    let first = versions(&fixture, Role::Operator, ("R-1", 0))
        .await
        .unwrap();
    let second = versions(&fixture, Role::Operator, ("R-1", 1))
        .await
        .unwrap();
    let beyond = versions(&fixture, Role::Operator, ("R-1", 7))
        .await
        .unwrap();

    assert_eq!(numbers(&first), [4, 3]);
    assert_eq!((first.page, first.has_next), (0, true));
    assert_eq!(numbers(&second), [2, 1]);
    assert!(!second.has_next);
    assert!(beyond.rows.is_empty());
    assert_eq!(first.rows.first().unwrap().state, LiveDraft);
}

#[tokio::test]
async fn list_versions_reads_in_one_read_only_transaction_and_writes_nothing() {
    let fixture = recipe_with_states(&[Approved]);

    versions(&fixture, Role::Operator, ("R-1", 0))
        .await
        .unwrap();

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn list_versions_of_an_unknown_reference_is_not_found() {
    let fixture = recipe_with_states(&[Approved]);

    let error = versions(&fixture, Role::Operator, ("R-9", 0))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        AppError::NotFound { entity: Missing::Recipe(ref found) } if found.to_string() == "R-9"
    ));
}

#[tokio::test]
async fn list_versions_of_a_deleted_recipe_is_refused_except_for_the_role_that_restores() {
    let fixture = scenarios::deleted();
    seed_users(&fixture);

    let operator = versions(&fixture, Role::Operator, ("R-1", 0))
        .await
        .unwrap_err();
    let engineer = versions(&fixture, Role::ProcessEngineer, ("R-1", 0))
        .await
        .unwrap_err();
    let administrator = versions(&fixture, Role::Administrator, ("R-1", 0))
        .await
        .unwrap();

    assert_eq!(operator.code(), ErrorCode::RCP_012);
    assert_eq!(engineer.code(), ErrorCode::RCP_012);
    assert_eq!(numbers(&administrator), [1]);
}

#[tokio::test]
async fn the_version_form_offers_every_version_newest_first() {
    let fixture = recipe_with_states(&[Retired, Approved, LiveDraft]);

    let result = form(&fixture, Role::ProcessEngineer, "R-1").await.unwrap();

    assert_eq!(result.reference.to_string(), "R-1");
    let choices: Vec<_> = result
        .choices
        .iter()
        .map(|choice| (choice.number, choice.state))
        .collect();
    assert_eq!(choices, [(3, LiveDraft), (2, Approved), (1, Retired)]);
    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
}

#[tokio::test]
async fn the_version_form_defaults_to_the_approved_version() {
    let fixture = recipe_with_states(&[Retired, Approved, LiveDraft]);

    let result = form(&fixture, Role::Administrator, "R-1").await.unwrap();

    assert_eq!(result.default_based_on, Some(2));
}

#[tokio::test]
async fn the_version_form_defaults_to_the_newest_version_when_none_is_approved() {
    let fixture = recipe_with_states(&[Retired, Retired]);

    let result = form(&fixture, Role::ProcessEngineer, "R-1").await.unwrap();

    assert_eq!(result.default_based_on, Some(2));
}

#[tokio::test]
async fn the_version_form_of_an_unknown_or_deleted_recipe_is_refused() {
    let unknown = recipe_with_states(&[Approved]);
    let deleted = scenarios::deleted();
    seed_users(&deleted);

    let missing = form(&unknown, Role::ProcessEngineer, "R-9")
        .await
        .unwrap_err();
    let gone = form(&deleted, Role::Administrator, "R-1")
        .await
        .unwrap_err();

    assert_eq!(missing.code(), ErrorCode::WEB_005);
    assert_eq!(gone.code(), ErrorCode::RCP_012);
}
