//! Tests of the audit entries of `SetPlcConnection`: targets, images, and password flags.

use super::*;

#[tokio::test]
async fn a_first_save_targets_the_row_id_with_the_machine_as_aggregate() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    use_case(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            save("M1", 0, s7_fields()),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    let target = entry.target();
    let row_id = stored_id(&fixture, mid(1)).await;
    assert_eq!(target.entity_type(), "plc_connection");
    assert_eq!(target.entity_id(), &row_id);
    assert_ne!(target.entity_id(), mid(1).as_uuid());
    assert_eq!(target.aggregate_type().as_sql(), "machine");
    assert_eq!(target.aggregate_id(), mid(1).as_uuid());
    let image_id = entry.after().and_then(|after| after.get("id"));
    assert_eq!(image_id, Some(&json!(row_id.to_string())));
}

#[tokio::test]
async fn an_update_targets_the_row_id_with_the_machine_as_aggregate() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.store.seed_connection(mid(1), &s7_record(), None);
    let row_id = stored_id(&fixture, mid(1)).await;
    use_case(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            save("M1", 1, s7_fields()),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    let target = entry.target();
    assert_eq!(target.entity_type(), "plc_connection");
    assert_eq!(target.entity_id(), &row_id);
    assert_ne!(target.entity_id(), mid(1).as_uuid());
    assert_eq!(target.aggregate_id(), mid(1).as_uuid());
    let image_id = entry.after().and_then(|after| after.get("id"));
    assert_eq!(image_id, Some(&json!(row_id.to_string())));
}

#[tokio::test]
async fn a_first_save_with_a_password_holds_the_full_row_and_both_flags_true() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    assert!(entry.before().is_none());
    let after = entry.after().unwrap();
    assert_eq!(after.get("host"), Some(&json!("plc.local")));
    assert_eq!(after.get("row_version"), Some(&json!(1)));
    assert_eq!(after.get("password_set"), Some(&json!(true)));
    assert_eq!(after.get("password_changed"), Some(&json!(true)));
    assert_only_allowed_keys(Some(after));
    assert!(!format!("{after:?}").contains(NEW_PASSWORD));
}

#[tokio::test]
async fn a_first_save_without_a_password_holds_both_flags_false() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    use_case(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            save("M1", 0, s7_fields()),
        )
        .await
        .unwrap();
    let after = only_entry(&fixture).after().cloned().unwrap();
    assert_eq!(after.get("password_set"), Some(&json!(false)));
    assert_eq!(after.get("password_changed"), Some(&json!(false)));
    assert_only_allowed_keys(Some(&after));
}

#[tokio::test]
async fn keep_holds_password_set_true_and_password_changed_false() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.store.seed_connection(
        mid(2),
        &opcua_record(),
        Some(sealed_before(mid(2), "stored-secret")),
    );
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, opcua_fields(None)),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    let (before, after) = (entry.before().unwrap(), entry.after().unwrap());
    assert_eq!(before.get("password_set"), Some(&json!(true)));
    assert_eq!(after.get("password_set"), Some(&json!(true)));
    assert_eq!(after.get("password_changed"), Some(&json!(false)));
    assert_only_allowed_keys(Some(before));
    assert_only_allowed_keys(Some(after));
}

#[tokio::test]
async fn replace_holds_password_set_true_and_password_changed_true() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.store.seed_connection(
        mid(2),
        &opcua_record(),
        Some(sealed_before(mid(2), "stored-secret")),
    );
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, opcua_fields(Some(NEW_PASSWORD))),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    let after = entry.after().unwrap();
    assert_eq!(after.get("password_set"), Some(&json!(true)));
    assert_eq!(after.get("password_changed"), Some(&json!(true)));
    let text = format!("{:?}", (entry.before(), entry.after()));
    assert!(!text.contains(NEW_PASSWORD) && !text.contains("stored-secret"));
}

#[tokio::test]
async fn clear_of_a_stored_password_holds_password_set_false_and_password_changed_true() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture.store.seed_connection(
        mid(2),
        &opcua_record(),
        Some(sealed_before(mid(2), "stored-secret")),
    );
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 1, modbus_fields()),
        )
        .await
        .unwrap();
    let entry = only_entry(&fixture);
    assert_eq!(
        entry.before().and_then(|before| before.get("password_set")),
        Some(&json!(true))
    );
    let after = entry.after().unwrap();
    assert_eq!(after.get("password_set"), Some(&json!(false)));
    assert_eq!(after.get("password_changed"), Some(&json!(true)));
}

#[tokio::test]
async fn clear_without_a_stored_password_holds_password_changed_false() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    use_case(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            save("M2", 0, modbus_fields()),
        )
        .await
        .unwrap();
    let after = only_entry(&fixture).after().cloned().unwrap();
    assert_eq!(after.get("password_set"), Some(&json!(false)));
    assert_eq!(after.get("password_changed"), Some(&json!(false)));
}
