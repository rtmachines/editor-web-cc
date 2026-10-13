use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::{RecipeDeleter, VersionState};
use serde_json::Value;

use super::{DeleteRecipe, DeleteRecipeInput, deleter_of};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, field, lock_line, meta, recipe_with, reference, rid, user, vid,
};
use crate::memory::{JournalEntry, LockMode};
use crate::ports::Isolation;
use crate::recipe::test_support::seed_deleted;

fn input(reference_text: &str) -> DeleteRecipeInput {
    DeleteRecipeInput {
        reference: reference(reference_text),
    }
}

/// Store recipe 1 of machine 1 with the given versions: `(id, state)` in number order.
fn seed_versions(fixture: &Fixture, versions: &[(u128, VersionState)]) {
    let recipe = recipe_with((1, 1), "R-1", versions);
    fixture
        .store
        .seed_recipe(&recipe, PlcPlatform::TwinCat3, user(2));
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(DeleteRecipe::OPERATION, Operation::DeleteRecipe);
}

#[test]
fn the_role_maps_to_the_deleter_and_an_operator_is_forbidden_without_a_panic() {
    assert_eq!(
        deleter_of(Role::Administrator).unwrap(),
        RecipeDeleter::Administrator
    );
    assert_eq!(
        deleter_of(Role::ProcessEngineer).unwrap(),
        RecipeDeleter::ProcessEngineer
    );
    let error = deleter_of(Role::Operator).unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(matches!(
        error,
        AppError::Forbidden {
            operation: Operation::DeleteRecipe
        }
    ));
}

#[tokio::test]
async fn an_administrator_deletes_a_recipe_with_a_retired_version() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_versions(&fixture, &[(101, VersionState::Retired)]);
    let output = DeleteRecipe::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("R-1"))
        .await
        .unwrap();
    assert_eq!(output.recipe, rid(1));
    assert!(fixture.store.state().recipe(rid(1)).unwrap().is_deleted());
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::RecipeDeleted);
    assert_eq!(entry.target().entity_type(), "recipe");
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(field(entry.before().unwrap(), "deleted_at"), Value::Null);
    assert_ne!(field(entry.after().unwrap(), "deleted_at"), Value::Null);
}

#[tokio::test]
async fn a_process_engineer_gets_rcp_026_for_a_retired_version() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_versions(&fixture, &[(101, VersionState::Retired)]);
    let error = DeleteRecipe::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_026);
    assert!(!fixture.store.state().recipe(rid(1)).unwrap().is_deleted());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_approved_version_gives_rcp_010_for_both_roles() {
    for role in [Role::Administrator, Role::ProcessEngineer] {
        let fixture = Fixture::new();
        let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
        fixture.seed_approved_and_draft(&machine, 1, "R-1");
        let error = DeleteRecipe::new(fixture.context())
            .execute(&actor(role), &meta(), input("R-1"))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_010);
        assert!(!fixture.store.state().recipe(rid(1)).unwrap().is_deleted());
        assert!(fixture.store.audit_entries().is_empty());
    }
}

#[tokio::test]
async fn a_deleted_recipe_gives_rcp_012() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let error = DeleteRecipe::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_live_draft_is_discarded_with_its_own_entry_before_the_recipe_entry() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let (_, draft) = fixture.seed_draft(&machine, 1, "R-1");
    let user = actor(Role::ProcessEngineer);
    DeleteRecipe::new(fixture.context())
        .execute(&user, &meta(), input("R-1"))
        .await
        .unwrap();
    let state = fixture.store.state();
    let stored = state.recipe(rid(1)).unwrap();
    assert!(stored.is_deleted());
    assert_eq!(stored.version(1).unwrap().state(), VersionState::Discarded);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 2);
    let discarded = entries.first().unwrap();
    assert_eq!(discarded.action(), AuditAction::DraftDiscarded);
    assert_eq!(discarded.target().entity_type(), "recipe_version");
    assert_eq!(discarded.target().entity_id(), draft.as_uuid());
    assert_eq!(
        field(discarded.before().unwrap(), "deleted_at"),
        Value::Null
    );
    assert_eq!(
        field(discarded.before().unwrap(), "row_version"),
        Value::from(1)
    );
    assert_ne!(field(discarded.after().unwrap(), "deleted_at"), Value::Null);
    assert_eq!(
        field(discarded.after().unwrap(), "row_version"),
        Value::from(2)
    );
    let deleted = entries.get(1).unwrap();
    assert_eq!(deleted.action(), AuditAction::RecipeDeleted);
    assert_eq!(deleted.actor(), Some(user.user()));
}

#[tokio::test]
async fn the_journal_holds_the_recipe_lock_and_then_the_version_locks_in_id_order() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_versions(
        &fixture,
        &[
            (103, VersionState::Retired),
            (101, VersionState::Retired),
            (102, VersionState::Retired),
        ],
    );
    DeleteRecipe::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("R-1"))
        .await
        .unwrap();
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
            lock_line("recipe_version", LockMode::ForUpdate, vid(101).to_string()),
            lock_line("recipe_version", LockMode::ForUpdate, vid(102).to_string()),
            lock_line("recipe_version", LockMode::ForUpdate, vid(103).to_string()),
        ]
    );
}

#[tokio::test]
async fn an_approval_between_the_read_and_the_locks_gives_rcp_010() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    // The hook runs after the read of step 2 and before the locks of step 4.
    let context = fixture.context_racing(|store| {
        let approved = recipe_with((1, 1), "R-1", &[(101, VersionState::Approved)]);
        store.seed_recipe(&approved, PlcPlatform::TwinCat3, user(2));
    });
    let error = DeleteRecipe::new(context)
        .execute(&actor(Role::Administrator), &meta(), input("R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_010);
    assert!(!fixture.store.state().recipe(rid(1)).unwrap().is_deleted());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_unknown_reference_gives_web_005() {
    let fixture = Fixture::new();
    let error = DeleteRecipe::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("R-9"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let error = DeleteRecipe::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert!(!fixture.store.state().recipe(rid(1)).unwrap().is_deleted());
}
