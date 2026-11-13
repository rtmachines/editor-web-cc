//! Tests of the save path of `SetPlcConnection`: access, first save, row version, race, and fault rules.

use super::*;

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(SetPlcConnection::OPERATION, Operation::SetPlcConnection);
}

#[tokio::test]
async fn an_operator_gets_forbidden_before_any_read_and_one_security_event() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = use_case(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("M1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(matches!(
        error,
        AppError::Forbidden {
            operation: Operation::SetPlcConnection
        }
    ));
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert!(
        events
            .first()
            .is_some_and(|event| event.kind() == SecurityEventKind::Forbidden)
    );
}

#[tokio::test]
async fn an_export_client_gets_forbidden() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let error = use_case(fixture.context())
        .execute(&actor(Role::ExportClient), &meta(), input("M1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(fixture.store.journal().is_empty());
}

#[tokio::test]
async fn an_unknown_machine_gives_web_005() {
    let fixture = Fixture::new();
    let error = use_case(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("M9"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound { entity: Missing::Machine(code) }
            if code == MachineCode::parse("M9").unwrap()
    ));
}

#[test]
fn the_command_stops_in_read_only_mode() {
    // The gate of `E-WEB-006` lives in `recipe-web` and reads this flag (`reliability.md` 7.8, A24).
    assert!(!SetPlcConnection::OPERATION.runs_in_read_only());
}

#[tokio::test]
async fn a_first_save_creates_the_record_at_row_version_1_and_writes_one_entry() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let output = use_case(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            save("M1", 0, s7_fields()),
        )
        .await
        .unwrap();
    assert_eq!(output.form.machine, mid(1));
    assert_eq!(output.form.row_version, 1);
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let stored = tx
        .plc_connections()
        .connection_by_machine(mid(1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.row_version, 1);
    assert!(matches!(
        stored.connection,
        PlcConnection::S7 { ref host, .. } if host.as_str() == "10.0.0.5"
    ));
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::PlcConnectionSet);
    assert!(entry.before().is_none());
    assert_eq!(
        entry.after().and_then(|after| after.get("row_version")),
        Some(&json!(1))
    );
}

#[tokio::test]
async fn a_first_save_reads_first_and_then_locks_the_machine() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields()),
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
async fn an_update_locks_the_machine_before_the_record_and_adds_1_to_the_row_version() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.seed_connection(mid(1), &s7_record(), None);
    let output = use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 1, s7_fields()),
        )
        .await
        .unwrap();
    assert_eq!(output.form.row_version, 2);
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForNoKeyUpdate, mid(1).to_string()),
            lock_line("plc_connection", LockMode::ForUpdate, mid(1).to_string()),
        ]
    );
    let entries = fixture.store.audit_entries();
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::PlcConnectionSet);
    assert_eq!(
        entry.before().and_then(|before| before.get("host")),
        Some(&json!("10.0.0.9"))
    );
    assert_eq!(
        entry.after().and_then(|after| after.get("host")),
        Some(&json!("10.0.0.5"))
    );
}

#[tokio::test]
async fn an_iec_first_save_without_credentials_passes() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let output = use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, modbus_fields()),
        )
        .await
        .unwrap();
    assert_eq!(output.form.machine, mid(2));
    assert_eq!(output.form.row_version, 1);
}

#[tokio::test]
async fn a_stale_row_version_gives_web_013_with_both_versions_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.seed_connection(mid(1), &s7_record(), None);
    let error = use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields()),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_013);
    assert!(matches!(
        error,
        AppError::ConnectionConcurrentEdit { expected: 0, actual: 1, code }
            if code == MachineCode::parse("M1").unwrap()
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_expected_version_above_zero_for_a_machine_without_a_record_gives_web_013() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let error = use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 3, s7_fields()),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::ConnectionConcurrentEdit {
            expected: 3,
            actual: 0,
            ..
        }
    ));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn two_first_saves_that_race_give_one_record_and_one_web_013() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    // The hook runs after the read of step 2 and before the transaction of step 4.
    // It stands for the first save of another user, which commits in between.
    let context = fixture.context_racing(|store| {
        store.seed_connection(mid(1), &s7_record(), None);
    });
    let error = use_case(context)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields()),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_013);
    assert!(matches!(
        error,
        AppError::ConnectionConcurrentEdit {
            expected: 0,
            actual: 1,
            ..
        }
    ));
    assert_eq!(fixture.store.state().connections.len(), 1);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_machine_archived_between_the_read_and_the_lock_gives_plt_010_and_writes_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived.archive(timestamp(), None).unwrap();
        store.seed_machine(&archived);
    });
    let error = use_case(context)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields()),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.store.state().connections.is_empty());
}

#[tokio::test]
async fn a_field_fault_gives_the_plant_errors_before_the_command_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let fields = ConnectionFields {
        port: Some(TcpPort::parse("502").unwrap()),
        ..s7_fields()
    };
    let error = use_case(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), save("M1", 0, fields))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_022);
    assert!(matches!(error, AppError::Plants(_)));
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
}

#[tokio::test]
async fn a_failed_audit_insert_keeps_no_record() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.fail_append_audit(Fault::Unavailable);
    let error = use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields()),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::AUD_001);
    assert!(fixture.store.state().connections.is_empty());
}

#[tokio::test]
async fn a_save_on_an_s7_machine_writes_the_copy_from_the_machine_number() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    set_machine_number(&fixture, 1, "77");
    let command = use_case(fixture.context());
    command
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields()),
        )
        .await
        .unwrap();
    assert_eq!(stored_copy(&fixture).await, json!(77));
    set_machine_number(&fixture, 1, "78");
    command
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 1, s7_fields()),
        )
        .await
        .unwrap();
    assert_eq!(stored_copy(&fixture).await, json!(78));
}
