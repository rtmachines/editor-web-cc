use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    DataBlockNumber, MachineCode, PlcConnection, PlcHost, PlcPlatform, Rack, Slot,
};
use serde_json::json;

use super::{SetMachineDataBlock, SetMachineDataBlockInput};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{Fixture, actor, lock_line, meta, mid, timestamp};
use crate::memory::{Fault, JournalEntry, LockMode};
use crate::ports::{Isolation, Transaction};

fn input(code: &str, number: Option<&str>) -> SetMachineDataBlockInput {
    SetMachineDataBlockInput {
        code: MachineCode::parse(code).unwrap(),
        db_number: number.map(|text| DataBlockNumber::parse(text).unwrap()),
    }
}

fn s7_record() -> PlcConnection {
    PlcConnection::S7 {
        host: PlcHost::parse("10.0.0.9").unwrap(),
        rack: Rack::parse("0").unwrap(),
        slot: Slot::parse("1").unwrap(),
    }
}

fn stored_number(fixture: &Fixture) -> Option<u16> {
    fixture
        .store
        .state()
        .machine(mid(1))
        .unwrap()
        .db_number()
        .map(DataBlockNumber::value)
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(
        SetMachineDataBlock::OPERATION,
        Operation::SetMachineDataBlock
    );
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    let error = SetMachineDataBlock::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M1", Some("7")))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(stored_number(&fixture), Some(1));
}

#[tokio::test]
async fn a_forbidden_role_with_a_failing_security_sink_gives_the_storage_error_and_changes_nothing()
{
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    fixture.sink.fail_with(Fault::Unavailable);
    let error = SetMachineDataBlock::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M1", Some("7")))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert!(fixture.store.journal().is_empty());
    assert_eq!(stored_number(&fixture), Some(1));
}

#[tokio::test]
async fn an_iec_machine_with_a_number_gives_plt_051_before_the_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::CodesysIec, 1, "M1");
    let error = SetMachineDataBlock::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", Some("10")),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_051);
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_iec_machine_without_a_number_gives_plt_051_before_the_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = SetMachineDataBlock::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("M1", None))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_051);
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
}

#[tokio::test]
async fn an_s7_machine_without_a_number_gives_plt_050_before_the_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let error = SetMachineDataBlock::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input("M1", None))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_050);
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert_eq!(stored_number(&fixture), Some(1));
}

#[tokio::test]
async fn a_number_above_the_platform_limit_gives_plt_024_before_the_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    let error = SetMachineDataBlock::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", Some("16001")),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_024);
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert_eq!(stored_number(&fixture), Some(1));
}

#[tokio::test]
async fn an_unknown_code_gives_web_005_and_opens_no_command_transaction() {
    let fixture = Fixture::new();
    let error = SetMachineDataBlock::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M9", Some("7")),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound {
            entity: Missing::Machine(_)
        }
    ));
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
}

#[tokio::test]
async fn a_machine_archived_between_the_read_and_the_lock_gives_plt_010_and_writes_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived.archive(timestamp(), None).unwrap();
        store.seed_machine(&archived);
    });
    let error = SetMachineDataBlock::new(context)
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", Some("7")),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(stored_number(&fixture), Some(1));
}

#[tokio::test]
async fn the_journal_holds_the_read_the_transaction_and_the_machine_lock() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1200, 1, "M1");
    SetMachineDataBlock::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("M1", Some("7")),
        )
        .await
        .unwrap();
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForNoKeyUpdate, mid(1).to_string()),
        ]
    );
}

#[tokio::test]
async fn success_sets_the_number_and_writes_one_entry_with_both_images() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    let output = SetMachineDataBlock::new(fixture.context())
        .execute(&user, &request, input("M1", Some("200")))
        .await
        .unwrap();
    assert_eq!(output.machine, mid(1));
    assert_eq!(stored_number(&fixture), Some(200));
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::MachineDataBlockSet);
    assert_eq!(entry.target().entity_type(), "machine");
    assert_eq!(entry.target().entity_id(), mid(1).as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Machine);
    assert_eq!(entry.before().unwrap().get("db_number"), Some(&json!(1)));
    assert_eq!(entry.after().unwrap().get("db_number"), Some(&json!(200)));
    assert_eq!(entry.actor(), Some(user.user()));
    assert_eq!(entry.request_id(), request.request_id());
}

#[tokio::test]
async fn the_current_number_still_writes_one_entry_and_leaves_the_record_unchanged() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.seed_connection(mid(1), &s7_record(), None);
    let user = actor(Role::ProcessEngineer);
    SetMachineDataBlock::new(fixture.context())
        .execute(&user, &meta(), input("M1", Some("1")))
        .await
        .unwrap();
    assert_eq!(stored_number(&fixture), Some(1));
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let stored = tx
        .plc_connections()
        .connection_by_machine(mid(1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.row_version, 1);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::MachineDataBlockSet);
    assert_eq!(entry.before().unwrap().get("db_number"), Some(&json!(1)));
    assert_eq!(entry.after().unwrap().get("db_number"), Some(&json!(1)));
}

#[tokio::test]
async fn the_output_carries_the_platform_of_the_machine() {
    for platform in [PlcPlatform::S7_300, PlcPlatform::S7_1500] {
        let fixture = Fixture::new();
        fixture.seed_machine_on(platform, 1, "M1");
        let output = SetMachineDataBlock::new(fixture.context())
            .execute(
                &actor(Role::ProcessEngineer),
                &meta(),
                input("M1", Some("5")),
            )
            .await
            .unwrap();
        assert_eq!(output.platform, platform);
        assert_eq!(output.platform.max_data_block(), platform.max_data_block());
    }
}
