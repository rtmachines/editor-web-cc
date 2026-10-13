use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use serde_json::Value;

use super::{RenameRecipe, RenameRecipeInput, RestoreRecipe, RestoreRecipeInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::memory::fixtures::{
    Fixture, actor, field, lock_line, meta, mid, reference, rid, text, timestamp,
};
use crate::memory::{JournalEntry, LockMode};
use crate::ports::Isolation;
use crate::recipe::test_support::seed_deleted;

fn rename_input(reference_text: &str, name: &str) -> RenameRecipeInput {
    RenameRecipeInput {
        reference: reference(reference_text),
        name: text(name),
    }
}

fn restore_input(reference_text: &str) -> RestoreRecipeInput {
    RestoreRecipeInput {
        reference: reference(reference_text),
    }
}

#[test]
fn the_use_cases_declare_their_operations() {
    assert_eq!(RenameRecipe::OPERATION, Operation::RenameRecipe);
    assert_eq!(RestoreRecipe::OPERATION, Operation::RestoreRecipe);
}

#[tokio::test]
async fn rename_locks_the_recipe_and_writes_one_update_entry() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    let output = RenameRecipe::new(fixture.context())
        .execute(&user, &request, rename_input("R-1", "Renamed"))
        .await
        .unwrap();
    assert_eq!(output.recipe, rid(1));
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
        ]
    );
    let stored = fixture.store.state().recipe(rid(1)).unwrap();
    assert_eq!(stored.name(), &text("Renamed"));
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::RecipeRenamed);
    assert_eq!(entry.target().entity_type(), "recipe");
    assert_eq!(entry.target().entity_id(), rid(1).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(
        field(entry.before().unwrap(), "name"),
        Value::from("Recipe")
    );
    assert_eq!(
        field(entry.after().unwrap(), "name"),
        Value::from("Renamed")
    );
    assert_eq!(entry.actor(), Some(user.user()));
    assert_eq!(entry.request_id(), request.request_id());
}

#[tokio::test]
async fn rename_of_a_deleted_recipe_gives_rcp_012_and_writes_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let error = RenameRecipe::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            rename_input("R-1", "Late"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert!(fixture.store.audit_entries().is_empty());
    let stored = fixture.store.state().recipe(rid(1)).unwrap();
    assert_eq!(stored.name(), &text("Recipe"));
}

#[tokio::test]
async fn rename_finds_a_recipe_that_is_deleted_after_the_read_under_the_lock() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let (mut recipe, _) = fixture.seed_draft(&machine, 1, "R-1");
    let context = fixture.context_racing(move |store| {
        recipe
            .delete(
                recipe_domain::recipe::RecipeDeleter::Administrator,
                timestamp(),
            )
            .unwrap();
        store.seed_recipe(
            &recipe,
            PlcPlatform::TwinCat3,
            crate::memory::fixtures::user(2),
        );
    });
    let error = RenameRecipe::new(context)
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            rename_input("R-1", "Late"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn rename_of_an_unknown_reference_gives_web_005() {
    let fixture = Fixture::new();
    let error = RenameRecipe::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            rename_input("R-9", "Name"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_operator_cannot_rename() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let error = RenameRecipe::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), rename_input("R-1", "X"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert_eq!(fixture.sink.events().len(), 1);
    assert!(fixture.store.journal().is_empty());
}

#[tokio::test]
async fn restore_locks_the_machine_and_then_the_recipe_and_writes_one_update_entry() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let user = actor(Role::Administrator);
    let output = RestoreRecipe::new(fixture.context())
        .execute(&user, &meta(), restore_input("R-1"))
        .await
        .unwrap();
    assert_eq!(output.recipe, rid(1));
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForShare, mid(1).to_string()),
            lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
        ]
    );
    assert!(!fixture.store.state().recipe(rid(1)).unwrap().is_deleted());
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::RecipeRestored);
    assert_eq!(entry.target().entity_id(), rid(1).as_uuid());
    assert_ne!(field(entry.before().unwrap(), "deleted_at"), Value::Null);
    assert_eq!(field(entry.after().unwrap(), "deleted_at"), Value::Null);
    assert_eq!(entry.actor(), Some(user.user()));
}

#[tokio::test]
async fn restore_of_a_recipe_that_is_not_deleted_gives_rcp_025() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let error = RestoreRecipe::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), restore_input("R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_025);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn restore_on_an_archived_machine_gives_plt_010_and_keeps_the_recipe_deleted() {
    let fixture = Fixture::new();
    let mut machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    machine.archive(timestamp(), None).unwrap();
    fixture.store.seed_machine(&machine);
    let error = RestoreRecipe::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), restore_input("R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
    assert!(fixture.store.state().recipe(rid(1)).unwrap().is_deleted());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn restore_of_an_unknown_reference_gives_web_005() {
    let fixture = Fixture::new();
    let error = RestoreRecipe::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), restore_input("R-9"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
}

#[tokio::test]
async fn a_process_engineer_cannot_restore() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let error = RestoreRecipe::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), restore_input("R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.state().recipe(rid(1)).unwrap().is_deleted());
}
