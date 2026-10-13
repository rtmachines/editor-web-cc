use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlantError, PlcPlatform};
use recipe_domain::recipe::{RecipeError, VersionState};

use super::{CreateRecipe, CreateRecipeInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, field, lock_line, long_text, meta, mid, reference, text,
};
use crate::memory::{JournalEntry, LockMode};
use crate::ports::Isolation;

fn input(code: &str, reference_text: &str) -> CreateRecipeInput {
    CreateRecipeInput {
        machine_code: MachineCode::parse(code).unwrap(),
        reference: reference(reference_text),
        name: text("Front door"),
        change_note: long_text("First version"),
    }
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(CreateRecipe::OPERATION, Operation::CreateRecipe);
}

#[tokio::test]
async fn success_creates_the_recipe_with_version_one_and_writes_two_entries() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    let output = CreateRecipe::new(fixture.context())
        .execute(&user, &request, input("M1", "R-1"))
        .await
        .unwrap();
    let state = fixture.store.state();
    let recipe = state.recipe(output.recipe).unwrap();
    assert_eq!(recipe.reference(), &reference("R-1"));
    assert_eq!(recipe.machine(), mid(1));
    let version = recipe.version(1).unwrap();
    assert_eq!(version.id(), output.version);
    assert_eq!(version.state(), VersionState::LiveDraft);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 2);
    let created = entries.first().unwrap();
    assert_eq!(created.action(), AuditAction::RecipeCreated);
    assert_eq!(created.target().entity_type(), "recipe");
    assert_eq!(created.target().entity_id(), output.recipe.as_uuid());
    assert_eq!(created.target().aggregate_type(), AggregateType::Recipe);
    assert!(created.before().is_none());
    assert_eq!(created.actor(), Some(user.user()));
    assert_eq!(created.request_id(), request.request_id());
    let version_entry = entries.get(1).unwrap();
    assert_eq!(version_entry.action(), AuditAction::VersionCreated);
    assert_eq!(version_entry.target().entity_type(), "recipe_version");
    assert_eq!(version_entry.target().entity_id(), output.version.as_uuid());
    let after = version_entry.after().unwrap();
    assert_eq!(field(after, "number"), serde_json::json!(1));
    assert_eq!(
        field(after, "change_note"),
        serde_json::json!("First version")
    );
    assert_eq!(
        field(after, "created_by"),
        serde_json::json!(user.user().to_string())
    );
}

#[tokio::test]
async fn the_journal_holds_the_machine_share_lock_only() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    CreateRecipe::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M1", "R-1"))
        .await
        .unwrap();
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForShare, mid(1).to_string()),
        ]
    );
}

#[tokio::test]
async fn a_used_reference_gives_rcp_001_and_writes_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 5, "R-1");
    let error = CreateRecipe::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_001);
    assert!(matches!(
        &error,
        AppError::Recipe(RecipeError::DuplicateReference { reference: found })
            if *found == reference("R-1")
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_unknown_machine_code_gives_plt_019() {
    let fixture = Fixture::new();
    let error = CreateRecipe::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("M9", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_019);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::MachineNotFound { code })
            if *code == MachineCode::parse("M9").unwrap()
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_archived_machine_gives_plt_010() {
    let fixture = Fixture::new();
    let mut machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    machine
        .archive(crate::memory::fixtures::timestamp(), None)
        .unwrap();
    fixture.store.seed_machine(&machine);
    let error = CreateRecipe::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(
        fixture
            .store
            .state()
            .recipe(crate::memory::fixtures::rid(1))
            .is_none()
    );
}

#[tokio::test]
async fn a_machine_archived_between_the_read_and_the_lock_gives_plt_010() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived
            .archive(crate::memory::fixtures::timestamp(), None)
            .unwrap();
        store.seed_machine(&archived);
    });
    let error = CreateRecipe::new(context)
        .execute(&actor(Role::ProcessEngineer), &meta(), input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = CreateRecipe::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
}
