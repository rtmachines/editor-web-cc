use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::Translation;
use serde_json::json;

use super::{RemoveTranslation, RemoveTranslationInput, UpsertTranslation, UpsertTranslationInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, lang, lock_line, long_text, meta, pid, pkey, reference, rid, text, vid,
};
use crate::memory::{JournalEntry, LockMode};
use crate::ports::Isolation;
use crate::recipe::DraftOutput;
use crate::recipe::test_support::{
    DRAFT_VERSION, seed_deleted, seed_speed_draft, speed_key, stored_parameters,
};

fn translation(code: &str, name: &str, description: Option<&str>) -> Translation {
    Translation {
        lang: lang(code),
        name: text(name),
        description: description.map(long_text),
    }
}

fn upsert(translation: Translation, expected: i64) -> UpsertTranslationInput {
    UpsertTranslationInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: expected,
        key: speed_key(),
        translation,
    }
}

fn remove(code: &str, expected: i64) -> RemoveTranslationInput {
    RemoveTranslationInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: expected,
        key: speed_key(),
        lang: lang(code),
    }
}

fn write_journal() -> Vec<JournalEntry> {
    vec![
        JournalEntry::Begin(Isolation::ReadOnly),
        JournalEntry::Begin(Isolation::ReadCommitted),
        lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
        lock_line(
            "recipe_version",
            LockMode::ForUpdate,
            vid(DRAFT_VERSION).to_string(),
        ),
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
    let parameters = stored_parameters(fixture);
    let speed = parameters.first().unwrap();
    assert_eq!(speed.translations.len(), 1);
}

#[test]
fn the_use_cases_declare_their_operations() {
    assert_eq!(UpsertTranslation::OPERATION, Operation::UpsertTranslation);
    assert_eq!(RemoveTranslation::OPERATION, Operation::RemoveTranslation);
}

#[tokio::test]
async fn upsert_of_a_new_language_adds_one_row_and_a_diff_entry() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let before = stored_parameters(&fixture);
    let engineer = actor(Role::ProcessEngineer);
    let output = UpsertTranslation::new(fixture.context())
        .execute(
            &engineer,
            &meta(),
            upsert(translation("de", "Geschwindigkeit", None), 1),
        )
        .await
        .unwrap();
    assert_eq!(
        output,
        DraftOutput {
            version: vid(DRAFT_VERSION),
            row_version: 2
        }
    );
    let after = stored_parameters(&fixture);
    let (old, new) = (before.first().unwrap(), after.first().unwrap());
    assert_eq!(new.translations.len(), 2);
    assert_eq!(new.value, old.value);
    assert_eq!(new.limits, old.limits);
    assert_eq!(fixture.store.journal(), write_journal());
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::TranslationUpserted);
    assert_eq!(entry.target().entity_type(), "parameter");
    assert_eq!(entry.target().entity_id(), pid(1).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(entry.target().aggregate_id(), rid(1).as_uuid());
    assert_eq!(entry.before(), Some(&json!({"translations": {"de": null}})));
    assert_eq!(
        entry.after(),
        Some(&json!({"translations": {"de": {"name": "Geschwindigkeit", "description": ""}}}))
    );
    assert_eq!(entry.actor(), Some(engineer.user()));
    assert_eq!(
        fixture.store.state().editors_of(vid(DRAFT_VERSION)),
        vec![engineer.user()]
    );
}

#[tokio::test]
async fn upsert_of_an_existing_language_keeps_one_row_for_it() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    UpsertTranslation::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            upsert(translation("pl", "Predkosc", Some("Nowy opis")), 1),
        )
        .await
        .unwrap();
    let after = stored_parameters(&fixture);
    let translations = &after.first().unwrap().translations;
    assert_eq!(translations.len(), 1);
    assert_eq!(translations.first().unwrap().name, text("Predkosc"));
    let entries = fixture.store.audit_entries();
    let entry = entries.first().unwrap();
    assert_eq!(
        entry.before(),
        Some(&json!({"translations": {"pl": {"name": "Nazwa", "description": "Opis"}}}))
    );
    assert_eq!(
        entry.after(),
        Some(&json!({"translations": {"pl": {"name": "Predkosc", "description": "Nowy opis"}}}))
    );
}

#[tokio::test]
async fn upsert_with_an_unknown_key_gives_rcp_051() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let mut request = upsert(translation("de", "X", None), 1);
    request.key = pkey(99);
    let error = UpsertTranslation::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_051);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn upsert_with_an_old_row_version_gives_web_001() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = UpsertTranslation::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            upsert(translation("de", "X", None), 6),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        &error,
        AppError::ConcurrentEdit {
            expected: 6,
            actual: 1,
            ..
        }
    ));
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn upsert_in_an_approved_version_gives_rcp_002_and_in_a_deleted_recipe_rcp_012() {
    let approved = Fixture::new();
    let machine = approved.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    approved.seed_approved_and_draft(&machine, 1, "R-1");
    let not_editable = UpsertTranslation::new(approved.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            upsert(translation("de", "X", None), 1),
        )
        .await
        .unwrap_err();
    assert_eq!(not_editable.code(), ErrorCode::RCP_002);
    let deleted = Fixture::new();
    let machine = deleted.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&deleted, &machine, 1, "R-1");
    let gone = UpsertTranslation::new(deleted.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            upsert(translation("de", "X", None), 1),
        )
        .await
        .unwrap_err();
    assert_eq!(gone.code(), ErrorCode::RCP_012);
}

#[tokio::test]
async fn an_operator_cannot_upsert_or_remove() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let operator = actor(Role::Operator);
    let upserted = UpsertTranslation::new(fixture.context())
        .execute(&operator, &meta(), upsert(translation("de", "X", None), 1))
        .await
        .unwrap_err();
    let removed = RemoveTranslation::new(fixture.context())
        .execute(&operator, &meta(), remove("pl", 1))
        .await
        .unwrap_err();
    assert_eq!(upserted.code(), ErrorCode::WEB_002);
    assert_eq!(removed.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|event| event.kind() == SecurityEventKind::Forbidden)
    );
    assert!(fixture.store.journal().is_empty());
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn remove_deletes_the_row_and_keeps_the_row_in_the_before_state() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let before = stored_parameters(&fixture);
    let engineer = actor(Role::ProcessEngineer);
    let output = RemoveTranslation::new(fixture.context())
        .execute(&engineer, &meta(), remove("pl", 1))
        .await
        .unwrap();
    assert_eq!(output.row_version, 2);
    assert_eq!(output.version, vid(DRAFT_VERSION));
    let after = stored_parameters(&fixture);
    let (old, new) = (before.first().unwrap(), after.first().unwrap());
    assert!(new.translations.is_empty());
    assert_eq!(new.value, old.value);
    assert_eq!(new.limits, old.limits);
    assert_eq!(fixture.store.journal(), write_journal());
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::TranslationRemoved);
    assert_eq!(entry.target().entity_type(), "parameter");
    assert_eq!(entry.target().entity_id(), pid(1).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(entry.target().aggregate_id(), rid(1).as_uuid());
    let removed = entry.before().unwrap();
    assert_eq!(
        removed.get("parameter_id"),
        Some(&json!(pid(1).to_string()))
    );
    assert_eq!(removed.get("lang"), Some(&json!("pl")));
    assert_eq!(removed.get("name"), Some(&json!("Nazwa")));
    assert_eq!(removed.get("description"), Some(&json!("Opis")));
    assert_eq!(entry.after(), None);
    assert_eq!(entry.actor(), Some(engineer.user()));
}

#[tokio::test]
async fn remove_of_an_unknown_language_gives_rcp_050() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = RemoveTranslation::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), remove("de", 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_050);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn remove_with_an_unknown_key_gives_rcp_051() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let mut request = remove("pl", 1);
    request.key = pkey(99);
    let error = RemoveTranslation::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_051);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn remove_with_an_old_row_version_gives_web_001() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let error = RemoveTranslation::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), remove("pl", 4))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_001);
    assert_nothing_written(&fixture);
}

#[tokio::test]
async fn remove_in_an_approved_version_gives_rcp_002_and_in_a_deleted_recipe_rcp_012() {
    let approved = Fixture::new();
    let machine = approved.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    approved.seed_approved_and_draft(&machine, 1, "R-1");
    let not_editable = RemoveTranslation::new(approved.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), remove("pl", 1))
        .await
        .unwrap_err();
    assert_eq!(not_editable.code(), ErrorCode::RCP_002);
    let deleted = Fixture::new();
    let machine = deleted.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&deleted, &machine, 1, "R-1");
    let gone = RemoveTranslation::new(deleted.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), remove("pl", 1))
        .await
        .unwrap_err();
    assert_eq!(gone.code(), ErrorCode::RCP_012);
}
