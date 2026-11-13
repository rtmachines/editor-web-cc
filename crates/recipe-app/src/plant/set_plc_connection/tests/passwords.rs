//! Tests of the password handling of `SetPlcConnection`: keep, replace, clear, and sealing (ADR-0035, ADR-0109).

use super::*;

#[tokio::test]
async fn keep_leaves_the_stored_ciphertext_and_does_not_seal() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let before = sealed_before(mid(2), "stored-secret");
    fixture
        .store
        .seed_connection(mid(2), &opcua_record(), Some(before.clone()));
    let cipher = MemoryCredentialCipher::default();
    let output = use_case_with(fixture.context(), &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, opcua_fields(None)),
        )
        .await
        .unwrap();
    assert_eq!(output.form.row_version, 2);
    let (sealed, state) = stored_password(&fixture, mid(2)).await;
    assert_eq!(sealed, Some(before));
    assert_eq!(state, PasswordState::Stored);
    assert!(cipher.calls().is_empty());
}

#[tokio::test]
async fn replace_stores_the_new_sealed_password_and_keeps_it_out_of_the_entry() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let before = sealed_before(mid(2), "stored-secret");
    fixture
        .store
        .seed_connection(mid(2), &opcua_record(), Some(before.clone()));
    let cipher = MemoryCredentialCipher::default();
    use_case_with(fixture.context(), &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap();
    assert_eq!(cipher.calls(), vec![CipherCall::Seal(mid(2))]);
    let (sealed, state) = stored_password(&fixture, mid(2)).await;
    let sealed = sealed.unwrap();
    assert_ne!(sealed, before);
    assert_eq!(state, PasswordState::Stored);
    let opened = cipher.open(mid(2), &sealed).unwrap();
    assert_eq!(opened.expose(), NEW_PASSWORD);
    let entries = fixture.store.audit_entries();
    let entry = entries.first().unwrap();
    let text = format!("{:?}", (entry.before(), entry.after()));
    assert!(!text.contains(NEW_PASSWORD));
    assert!(!text.contains("stored-secret"));
    assert!(!text.contains("password_sealed"));
}

#[tokio::test]
async fn a_first_save_with_a_password_stores_it_sealed() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let cipher = MemoryCredentialCipher::default();
    let output = use_case_with(fixture.context(), &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap();
    assert_eq!(output.form.row_version, 1);
    let (sealed, state) = stored_password(&fixture, mid(2)).await;
    assert_eq!(state, PasswordState::Stored);
    let opened = cipher.open(mid(2), &sealed.unwrap()).unwrap();
    assert_eq!(opened.expose(), NEW_PASSWORD);
}

#[tokio::test]
async fn clear_removes_the_stored_ciphertext_when_the_mode_has_no_credentials() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.store.seed_connection(
        mid(2),
        &opcua_record(),
        Some(sealed_before(mid(2), "stored-secret")),
    );
    let cipher = MemoryCredentialCipher::default();
    let output = use_case_with(fixture.context(), &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, modbus_fields()),
        )
        .await
        .unwrap();
    assert_eq!(output.form.row_version, 2);
    let (sealed, state) = stored_password(&fixture, mid(2)).await;
    assert!(sealed.is_none());
    assert_eq!(state, PasswordState::Absent);
    assert!(cipher.calls().is_empty());
}

#[tokio::test]
async fn an_s7_first_save_and_update_pass_without_a_password_and_never_seal() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let cipher = MemoryCredentialCipher::default();
    let command = use_case_with(fixture.context(), &cipher);
    command
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 0, s7_fields()),
        )
        .await
        .unwrap();
    command
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M1", 1, s7_fields()),
        )
        .await
        .unwrap();
    let (sealed, state) = stored_password(&fixture, mid(1)).await;
    assert!(sealed.is_none());
    assert_eq!(state, PasswordState::Absent);
    assert!(cipher.calls().is_empty());
}

#[tokio::test]
async fn the_password_is_sealed_before_the_command_transaction_opens() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let cipher = MemoryCredentialCipher::default();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let context = fixture.context_racing({
        let cipher = cipher.clone();
        let seen = Arc::clone(&seen);
        // The hook runs at the first `begin(ReadCommitted)`, so it sees the calls made before it.
        move |_store| *seen.lock() = cipher.calls()
    });
    use_case_with(context, &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap();
    assert_eq!(*seen.lock(), vec![CipherCall::Seal(mid(2))]);
    assert_eq!(
        fixture.store.journal().first(),
        Some(&JournalEntry::Begin(Isolation::ReadOnly))
    );
}

#[tokio::test]
async fn a_failed_seal_gives_task_failed_token_before_the_command_transaction() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let cipher = MemoryCredentialCipher::default();
    cipher.fail_seal(CipherFault::Random);
    let error = use_case_with(fixture.context(), &cipher)
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::TaskFailed {
            stage: TaskStage::Token
        }
    ));
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.store.state().connections.is_empty());
}
