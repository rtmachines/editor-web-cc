use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlcPlatform, UnitPath, UnitUsage};
use serde_json::Value;

use super::{ArchiveUnit, ArchiveUnitInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{
    Fixture, actor, draft_recipe, lock_line, meta, mid, param, timestamp, uid, user, vid,
};
use crate::memory::{JournalEntry, LockMode};
use crate::plant::short_rows::short_rows_context;
use crate::ports::Isolation;

fn input(code: &str, path: &str) -> ArchiveUnitInput {
    ArchiveUnitInput {
        code: MachineCode::parse(code).unwrap(),
        path: UnitPath::parse(path).unwrap(),
    }
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(ArchiveUnit::OPERATION, Operation::ArchiveUnit);
}

#[tokio::test]
async fn the_journal_holds_the_machine_lock_and_then_the_unit_locks_in_id_order() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    ArchiveUnit::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M1", "U1.2"))
        .await
        .unwrap();
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForNoKeyUpdate, mid(1).to_string()),
            lock_line("unit", LockMode::ForUpdate, uid(102).to_string()),
            lock_line("unit", LockMode::ForUpdate, uid(103).to_string()),
        ]
    );
}

#[tokio::test]
async fn success_archives_the_unit_and_its_descendants_with_one_entry_each() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let output = ArchiveUnit::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("M1", "U1.2"))
        .await
        .unwrap();
    assert_eq!(output.units, vec![uid(102), uid(103)]);
    let machine = fixture.store.state().machine(mid(1)).unwrap();
    for path in ["U1.2", "U1.2.1"] {
        let unit = machine.unit(&UnitPath::parse(path).unwrap()).unwrap();
        assert!(!unit.is_live(), "{path}");
    }
    assert!(
        machine
            .unit(&UnitPath::parse("U1").unwrap())
            .unwrap()
            .is_live()
    );
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 2);
    for (entry, id) in entries.iter().zip([uid(102), uid(103)]) {
        assert_eq!(entry.action(), AuditAction::UnitArchived);
        assert_eq!(entry.target().entity_type(), "unit");
        assert_eq!(entry.target().entity_id(), id.as_uuid());
        assert_eq!(entry.target().aggregate_type(), AggregateType::Machine);
        assert_eq!(entry.target().aggregate_id(), mid(1).as_uuid());
        assert_eq!(
            entry.before().unwrap().get("archived_at"),
            Some(&Value::Null)
        );
        assert_ne!(
            entry.after().unwrap().get("archived_at"),
            Some(&Value::Null)
        );
    }
}

#[tokio::test]
async fn an_unknown_code_gives_web_005() {
    let fixture = Fixture::new();
    let error = ArchiveUnit::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M9", "U1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
}

#[tokio::test]
async fn an_unknown_path_gives_web_005_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = ArchiveUnit::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M1", "U1.9"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound {
            entity: Missing::Unit { .. }
        }
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn the_last_main_unit_gives_plt_008_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = ArchiveUnit::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M1", "U1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_008);
    assert!(fixture.store.audit_entries().is_empty());
    let machine = fixture.store.state().machine(mid(1)).unwrap();
    assert!(
        machine
            .unit(&UnitPath::parse("U1").unwrap())
            .unwrap()
            .is_live()
    );
}

#[tokio::test]
async fn a_live_draft_that_uses_a_descendant_gives_plt_012_and_writes_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let (_, version) = fixture.seed_draft(&machine, 1, "R-1");
    fixture.store.seed_parameter(version, &param(1, 103, "A"));
    let error = ArchiveUnit::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M1", "U1.2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_012);
    assert!(fixture.store.audit_entries().is_empty());
    let stored = fixture.store.state().machine(mid(1)).unwrap();
    assert!(
        stored
            .unit(&UnitPath::parse("U1.2").unwrap())
            .unwrap()
            .is_live()
    );
}

#[tokio::test]
async fn an_adapter_that_returns_too_few_images_gives_db_008_and_commits_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = ArchiveUnit::new(short_rows_context(&fixture))
        .execute(&actor(Role::Administrator), &meta(), input("M1", "U1.2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(fixture.store.audit_entries().is_empty());
    let stored = fixture.store.state().machine(mid(1)).unwrap();
    assert!(
        stored
            .unit(&UnitPath::parse("U1.2").unwrap())
            .unwrap()
            .is_live()
    );
}

#[tokio::test]
async fn an_approved_version_that_uses_a_descendant_gives_plt_014() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let (_, _, approved) = fixture.seed_retired(&machine, 1, "R-1");
    fixture.store.seed_parameter(approved, &param(1, 103, "A"));
    let error = ArchiveUnit::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M1", "U1.2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_014);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_archived_unit_gives_plt_009_with_no_entry_and_no_change() {
    let fixture = Fixture::new();
    let mut machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    machine
        .archive_unit(
            &UnitPath::parse("U1.2.1").unwrap(),
            timestamp(),
            &UnitUsage::default(),
        )
        .unwrap();
    fixture.store.seed_machine(&machine);
    let before = fixture.store.state().machine(mid(1)).unwrap();
    let error = ArchiveUnit::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M1", "U1.2.1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_009);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(fixture.store.state().machine(mid(1)).unwrap(), before);
}

#[tokio::test]
async fn a_draft_that_appears_between_the_read_and_the_locks_gives_plt_012() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let platform = machine.platform();
    let context = fixture.context_racing(move |store| {
        store.seed_recipe(&draft_recipe((1, 1), "R-1", 11), platform, user(9));
        store.seed_parameter(vid(11), &param(1, 103, "A"));
    });
    let error = ArchiveUnit::new(context)
        .execute(&actor(Role::Administrator), &meta(), input("M1", "U1.2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_012);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_machine_archived_between_the_read_and_the_lock_gives_web_005_and_writes_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived.archive(timestamp(), None).unwrap();
        store.seed_machine(&archived);
    });
    let before = fixture.store.state().machine(mid(1)).unwrap();
    let error = ArchiveUnit::new(context)
        .execute(&actor(Role::Administrator), &meta(), input("M1", "U1.2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound {
            entity: Missing::Machine(_)
        }
    ));
    assert!(fixture.store.audit_entries().is_empty());
    let stored = fixture.store.state().machine(mid(1)).unwrap();
    let path = UnitPath::parse("U1.2").unwrap();
    assert_eq!(
        stored.unit(&path).unwrap().is_live(),
        before.unit(&path).unwrap().is_live()
    );
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = ArchiveUnit::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M1", "U1.2"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
}
