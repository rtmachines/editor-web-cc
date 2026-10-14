use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{LeafPath, PlcPlatform, Primitive};
use recipe_domain::recipe::{
    Draft, Parameter, StoredLimit, StoredParameter, StoredValue, Translation, VersionState,
};
use serde_json::{Value, json};

use crate::error::{PARAMETER_TAG_UNIQUE, StorageError};
use crate::memory::fixtures::{
    draft_recipe, field, lang, lock_line, locks, long_text, mid, now_text, param, pid, pkey,
    recipe_with, reference, store, text, tid, uid, user, vid,
};
use crate::memory::{
    DraftWrite, LockMode, MemoryState, MemoryStore, MemoryTransaction, WriteKind, seed_type_id,
};
use crate::ports::{Isolation, Transaction};

fn begin(store: &MemoryStore) -> MemoryTransaction {
    store.begin_memory(Isolation::ReadCommitted).unwrap()
}

/// A store with the recipe R1 whose version V11 is a live draft with the parameters A and B.
fn store_with_draft() -> MemoryStore {
    let store = store();
    store.seed_recipe(
        &draft_recipe((1, 1), "R-1", 11),
        PlcPlatform::TwinCat3,
        user(9),
    );
    store.seed_parameter(vid(11), &param(1, 101, "A"));
    store.seed_parameter(vid(11), &param(2, 102, "B"));
    store
}

fn store_with_approved() -> MemoryStore {
    let store = store();
    store.seed_recipe(
        &recipe_with((1, 1), "R-1", &[(11, VersionState::Approved)]),
        PlcPlatform::TwinCat3,
        user(9),
    );
    store.seed_parameter(vid(11), &param(1, 101, "A"));
    store
}

fn stored(state: &MemoryState, n: u128) -> StoredParameter {
    state
        .parameters_of(vid(11))
        .into_iter()
        .find(|p| p.id == pid(n))
        .unwrap()
}

fn is_code(error: &StorageError, expected: &ErrorCode) -> bool {
    matches!(error, StorageError::Constraint { code, .. } if code == expected)
}

#[tokio::test]
async fn lock_draft_takes_for_update_and_returns_the_draft_with_its_editors() {
    let store = store_with_draft();
    store.seed_editor(vid(11), user(7));
    store.seed_editor(vid(11), user(5));
    let mut tx = begin(&store);
    let stored = tx.drafts().lock_draft(vid(11)).await.unwrap().unwrap();
    assert_eq!(stored.reference, reference("R-1"));
    assert_eq!(stored.machine, mid(1));
    assert_eq!(stored.platform, PlcPlatform::TwinCat3);
    assert_eq!(stored.version, vid(11));
    assert_eq!(stored.number, 1);
    assert_eq!(stored.state, VersionState::LiveDraft);
    assert_eq!(stored.row_version, 1);
    assert_eq!(stored.created_by, user(9));
    assert_eq!(stored.editors, vec![user(5), user(7)]);
    assert_eq!(stored.parameters.len(), 2);
    assert_eq!(
        locks(&store),
        vec![lock_line(
            "recipe_version",
            LockMode::ForUpdate,
            vid(11).to_string()
        )]
    );
    assert!(Draft::load(stored).is_ok());
}

#[tokio::test]
async fn lock_draft_of_an_approved_version_returns_its_state_for_the_domain_to_reject() {
    let store = store_with_approved();
    let mut tx = begin(&store);
    let stored = tx.drafts().lock_draft(vid(11)).await.unwrap().unwrap();
    assert_eq!(stored.state, VersionState::Approved);
    assert!(Draft::load(stored).is_err());
}

#[tokio::test]
async fn lock_draft_of_a_missing_version_gives_none_and_no_journal_line() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    assert!(tx.drafts().lock_draft(vid(99)).await.unwrap().is_none());
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn read_draft_returns_the_draft_and_takes_no_lock() {
    let store = store_with_draft();
    store.seed_editor(vid(11), user(7));
    let mut tx = store.begin_memory(Isolation::ReadOnly).unwrap();
    let stored = tx.drafts().read_draft(vid(11)).await.unwrap().unwrap();
    assert_eq!(stored.version, vid(11));
    assert_eq!(stored.state, VersionState::LiveDraft);
    assert_eq!(stored.editors, vec![user(7)]);
    assert_eq!(stored.parameters.len(), 2);
    assert!(locks(&store).is_empty());
}

#[tokio::test]
async fn read_draft_of_a_missing_version_gives_none() {
    let store = store_with_draft();
    let mut tx = store.begin_memory(Isolation::ReadOnly).unwrap();
    assert!(tx.drafts().read_draft(vid(99)).await.unwrap().is_none());
}

#[tokio::test]
async fn read_draft_runs_the_hook_once_and_then_sees_the_committed_change() {
    let store = store_with_draft();
    store.on_read_draft(|store| {
        let recipe = recipe_with((1, 1), "R-1", &[(11, VersionState::Approved)]);
        store.seed_recipe(&recipe, PlcPlatform::TwinCat3, user(9));
    });
    // The transaction opens before the hook runs, as a request that lost a race does.
    let mut tx = store.begin_memory(Isolation::ReadOnly).unwrap();
    let first = tx.drafts().read_draft(vid(11)).await.unwrap().unwrap();
    assert_eq!(first.state, VersionState::Approved);
    store.seed_recipe(
        &draft_recipe((1, 1), "R-1", 11),
        PlcPlatform::TwinCat3,
        user(9),
    );
    let second = tx.drafts().read_draft(vid(11)).await.unwrap().unwrap();
    assert_eq!(second.state, VersionState::LiveDraft);
}

#[tokio::test]
async fn touch_draft_returns_the_next_row_version_and_adds_the_editor_once() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    assert_eq!(tx.drafts().touch_draft(vid(11), user(5)).await.unwrap(), 2);
    assert_eq!(tx.drafts().touch_draft(vid(11), user(5)).await.unwrap(), 3);
    assert_eq!(tx.state().editors_of(vid(11)), vec![user(5)]);
    assert_eq!(tx.state().version_row_version(vid(11)), Some(3));
}

#[tokio::test]
async fn touch_draft_of_a_version_that_is_not_a_live_draft_gives_e_rcp_033() {
    let store = store_with_approved();
    let mut tx = begin(&store);
    let error = tx.drafts().touch_draft(vid(11), user(5)).await.unwrap_err();
    assert!(is_code(&error, &ErrorCode::RCP_033), "{error:?}");
    assert!(tx.state().editors_of(vid(11)).is_empty());
}

#[tokio::test]
async fn touch_draft_of_a_missing_version_is_a_storage_error() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    assert!(tx.drafts().touch_draft(vid(99), user(5)).await.is_err());
}

#[tokio::test]
async fn insert_parameter_returns_the_row_with_its_limits_and_translations() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let parameter = Parameter::from(param(3, 103, "C"));
    let image = tx
        .drafts()
        .insert_parameter(vid(11), &parameter)
        .await
        .unwrap();
    let value = image.as_value();
    assert_eq!(field(value, "id"), json!(pid(3).to_string()));
    assert_eq!(
        field(value, "recipe_version_id"),
        json!(vid(11).to_string())
    );
    assert_eq!(field(value, "plc_platform"), json!("TwinCat3"));
    assert_eq!(field(value, "parameter_key"), json!(pkey(3).to_string()));
    assert_eq!(field(value, "unit_id"), json!(uid(103).to_string()));
    assert_eq!(field(value, "tag"), json!("C"));
    assert_eq!(field(value, "name_en"), json!("Name"));
    assert_eq!(field(value, "description_en"), json!("Description"));
    assert_eq!(field(value, "s7_address"), Value::Null);
    assert_eq!(field(value, "value"), json!(5));
    assert_eq!(field(value, "ordinal"), json!(0));
    let limits = field(value, "limits");
    assert_eq!(
        limits,
        json!([{
            "parameter_id": pid(3).to_string(),
            "recipe_version_id": vid(11).to_string(),
            "leaf_path": "",
            "min_value": 0,
            "max_value": 10,
            "max_length": null,
            "created_at": now_text(),
        }])
    );
    assert_eq!(
        field(value, "translations"),
        json!([{
            "parameter_id": pid(3).to_string(),
            "recipe_version_id": vid(11).to_string(),
            "lang": "pl",
            "name": "Nazwa",
            "description": "Opis",
            "created_at": now_text(),
        }])
    );
    assert_eq!(tx.state().parameters_of(vid(11)).len(), 3);
}

#[tokio::test]
async fn insert_parameter_with_a_known_tag_gives_parameter_tag_unique() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let parameter = Parameter::from(param(3, 103, "A"));
    let error = tx
        .drafts()
        .insert_parameter(vid(11), &parameter)
        .await
        .unwrap_err();
    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == PARAMETER_TAG_UNIQUE),
        "{error:?}"
    );
}

#[tokio::test]
async fn insert_parameter_with_an_unknown_unit_or_type_gives_a_missing_reference() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let unknown_unit = Parameter::from(param(3, 999, "C"));
    let error = tx
        .drafts()
        .insert_parameter(vid(11), &unknown_unit)
        .await
        .unwrap_err();
    assert!(
        matches!(error, StorageError::MissingReference { .. }),
        "{error:?}"
    );
    let unknown_type = Parameter::from(StoredParameter {
        plc_type: tid(999),
        ..param(3, 103, "C")
    });
    let error = tx
        .drafts()
        .insert_parameter(vid(11), &unknown_type)
        .await
        .unwrap_err();
    assert!(
        matches!(error, StorageError::MissingReference { .. }),
        "{error:?}"
    );
}

#[tokio::test]
async fn insert_parameter_into_a_version_that_is_not_a_live_draft_gives_e_rcp_031() {
    let store = store_with_approved();
    let mut tx = begin(&store);
    let parameter = Parameter::from(param(3, 103, "C"));
    let error = tx
        .drafts()
        .insert_parameter(vid(11), &parameter)
        .await
        .unwrap_err();
    assert!(is_code(&error, &ErrorCode::RCP_031), "{error:?}");
}

#[tokio::test]
async fn insert_parameter_into_a_missing_version_gives_a_missing_reference() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let parameter = Parameter::from(param(3, 103, "C"));
    let error = tx
        .drafts()
        .insert_parameter(vid(99), &parameter)
        .await
        .unwrap_err();
    assert!(
        matches!(error, StorageError::MissingReference { .. }),
        "{error:?}"
    );
}

fn retyped() -> StoredParameter {
    StoredParameter {
        unit: uid(103),
        tag: recipe_domain::recipe::ParameterTag::parse("NEW").unwrap(),
        name_en: text("Other"),
        description_en: long_text("Other description"),
        plc_type: seed_type_id(Primitive::Dint, None).unwrap(),
        value: StoredValue::from(json!(7)),
        limits: vec![StoredLimit {
            leaf_path: LeafPath::parse("").unwrap(),
            min: None,
            max: Some("99".parse().unwrap()),
            max_length: None,
        }],
        ..param(1, 101, "A")
    }
}

#[tokio::test]
async fn update_definition_writes_only_the_columns_and_keeps_value_limits_and_translations() {
    let store = store_with_draft();
    let before = stored(&store.state(), 1);
    let mut tx = begin(&store);
    let parameter = Parameter::from(retyped());
    tx.drafts()
        .update_definition(vid(11), &parameter)
        .await
        .unwrap();
    let after = stored(tx.state(), 1);
    assert_eq!(after.id, before.id);
    assert_eq!(after.key, before.key);
    assert_eq!(after.tag.to_string(), "NEW");
    assert_eq!(after.unit, uid(103));
    assert_eq!(after.plc_type, seed_type_id(Primitive::Dint, None).unwrap());
    assert_eq!(after.value, before.value);
    assert_eq!(after.limits, before.limits);
    assert_eq!(after.translations, before.translations);
    assert_eq!(
        store.draft_writes(),
        vec![DraftWrite {
            kind: WriteKind::Definition,
            key: pkey(1)
        }]
    );
}

#[tokio::test]
async fn set_value_and_replace_limits_after_update_definition_write_the_retyped_data() {
    let store = store_with_draft();
    let before = stored(&store.state(), 1);
    let mut tx = begin(&store);
    let parameter = Parameter::from(retyped());
    let drafts = tx.drafts();
    drafts.update_definition(vid(11), &parameter).await.unwrap();
    drafts
        .set_value(vid(11), parameter.key(), parameter.value())
        .await
        .unwrap();
    drafts
        .replace_limits(vid(11), parameter.key(), parameter.limits())
        .await
        .unwrap();
    let after = stored(tx.state(), 1);
    assert_eq!(after.value, StoredValue::from(json!(7)));
    assert_eq!(after.limits, retyped().limits);
    assert_eq!(after.translations, before.translations);
    let kinds: Vec<WriteKind> = store.draft_writes().iter().map(|w| w.kind).collect();
    assert_eq!(
        kinds,
        vec![WriteKind::Definition, WriteKind::Value, WriteKind::Limits]
    );
}

#[tokio::test]
async fn update_definition_to_a_tag_of_another_parameter_gives_parameter_tag_unique() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let parameter = Parameter::from(StoredParameter {
        tag: recipe_domain::recipe::ParameterTag::parse("B").unwrap(),
        ..param(1, 101, "A")
    });
    let error = tx
        .drafts()
        .update_definition(vid(11), &parameter)
        .await
        .unwrap_err();
    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == PARAMETER_TAG_UNIQUE),
        "{error:?}"
    );
}

#[tokio::test]
async fn set_value_leaves_the_limit_and_translation_rows_equal() {
    let store = store_with_draft();
    let before = stored(&store.state(), 1);
    let mut tx = begin(&store);
    tx.drafts()
        .set_value(vid(11), pkey(1), &StoredValue::from(json!(9)))
        .await
        .unwrap();
    let after = stored(tx.state(), 1);
    assert_eq!(after.value, StoredValue::from(json!(9)));
    assert_eq!(after.limits, before.limits);
    assert_eq!(after.translations, before.translations);
    assert_eq!(after.tag, before.tag);
}

#[tokio::test]
async fn replace_limits_leaves_the_value_and_translation_rows_equal() {
    let store = store_with_draft();
    let before = stored(&store.state(), 1);
    let mut tx = begin(&store);
    let limits = [StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some("1".parse().unwrap()),
        max: None,
        max_length: None,
    }];
    tx.drafts()
        .replace_limits(vid(11), pkey(1), &limits)
        .await
        .unwrap();
    let after = stored(tx.state(), 1);
    assert_eq!(after.limits, limits.to_vec());
    assert_eq!(after.value, before.value);
    assert_eq!(after.translations, before.translations);
}

#[tokio::test]
async fn replace_limits_with_an_empty_list_removes_the_limit_rows() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    tx.drafts()
        .replace_limits(vid(11), pkey(1), &[])
        .await
        .unwrap();
    assert!(stored(tx.state(), 1).limits.is_empty());
}

#[tokio::test]
async fn replace_limits_rejects_a_repeated_path_and_an_empty_limit_row() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let first = StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some("1".parse().unwrap()),
        max: None,
        max_length: None,
    };
    let error = tx
        .drafts()
        .replace_limits(vid(11), pkey(1), &[first.clone(), first.clone()])
        .await
        .unwrap_err();
    assert!(matches!(error, StorageError::Duplicate { .. }), "{error:?}");
    let empty = StoredLimit { min: None, ..first };
    let error = tx
        .drafts()
        .replace_limits(vid(11), pkey(1), &[empty])
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_003);
}

#[tokio::test]
async fn upsert_translation_adds_a_language_and_replaces_an_existing_one() {
    let store = store_with_draft();
    let before = stored(&store.state(), 1);
    let mut tx = begin(&store);
    let german = Translation {
        lang: lang("de"),
        name: text("Name"),
        description: None,
    };
    tx.drafts()
        .upsert_translation(vid(11), pkey(1), &german)
        .await
        .unwrap();
    let polish = Translation {
        lang: lang("pl"),
        name: text("Nowa"),
        description: None,
    };
    tx.drafts()
        .upsert_translation(vid(11), pkey(1), &polish)
        .await
        .unwrap();
    let after = stored(tx.state(), 1);
    let names: Vec<(String, String)> = after
        .translations
        .iter()
        .map(|t| (t.lang.to_string(), t.name.to_string()))
        .collect();
    assert_eq!(
        names,
        vec![
            ("de".to_owned(), "Name".to_owned()),
            ("pl".to_owned(), "Nowa".to_owned())
        ]
    );
    assert_eq!(after.limits, before.limits);
    assert_eq!(after.value, before.value);
}

#[tokio::test]
async fn delete_translation_removes_one_language_and_a_missing_one_is_a_storage_error() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    tx.drafts()
        .delete_translation(vid(11), pkey(1), &lang("pl"))
        .await
        .unwrap();
    assert!(stored(tx.state(), 1).translations.is_empty());
    let again = tx
        .drafts()
        .delete_translation(vid(11), pkey(1), &lang("pl"))
        .await;
    assert!(again.is_err());
}

#[tokio::test]
async fn delete_translation_returns_the_row_before_the_delete() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let image = tx
        .drafts()
        .delete_translation(vid(11), pkey(1), &lang("pl"))
        .await
        .unwrap();
    let value = image.as_value();
    assert_eq!(field(value, "parameter_id"), json!(pid(1).to_string()));
    assert_eq!(field(value, "lang"), json!("pl"));
    assert_eq!(field(value, "name"), json!("Nazwa"));
    assert_eq!(field(value, "description"), json!("Opis"));
}

#[tokio::test]
async fn delete_parameter_returns_the_row_before_the_delete_with_its_translation_for_pl() {
    let store = store_with_draft();
    let mut tx = begin(&store);
    let image = tx
        .drafts()
        .delete_parameter(vid(11), pkey(1))
        .await
        .unwrap();
    let value = image.as_value();
    assert_eq!(field(value, "id"), json!(pid(1).to_string()));
    assert_eq!(field(value, "tag"), json!("A"));
    assert_eq!(field(value, "limits").as_array().map(Vec::len), Some(1));
    let translations = field(value, "translations");
    assert_eq!(
        translations.get(0).map(|t| field(t, "lang")),
        Some(json!("pl"))
    );
    assert_eq!(
        translations.get(0).map(|t| field(t, "name")),
        Some(json!("Nazwa"))
    );
    let remaining = tx.state().parameters_of(vid(11));
    assert_eq!(remaining.len(), 1);
    assert!(
        tx.drafts()
            .delete_parameter(vid(11), pkey(1))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_parameter_write_in_a_version_that_is_not_a_live_draft_gives_e_rcp_031() {
    let store = store_with_approved();
    let mut tx = begin(&store);
    let value = StoredValue::from(json!(1));
    let error = tx
        .drafts()
        .set_value(vid(11), pkey(1), &value)
        .await
        .unwrap_err();
    assert!(is_code(&error, &ErrorCode::RCP_031), "{error:?}");
    let error = tx
        .drafts()
        .delete_parameter(vid(11), pkey(1))
        .await
        .unwrap_err();
    assert!(is_code(&error, &ErrorCode::RCP_031), "{error:?}");
}

#[tokio::test]
async fn a_rollback_keeps_no_draft_change() {
    let store = store_with_draft();
    let before = store.state().parameters_of(vid(11));
    let mut tx = begin(&store);
    tx.drafts().touch_draft(vid(11), user(5)).await.unwrap();
    tx.drafts()
        .insert_parameter(vid(11), &Parameter::from(param(3, 103, "C")))
        .await
        .unwrap();
    tx.drafts()
        .set_value(vid(11), pkey(1), &StoredValue::from(json!(9)))
        .await
        .unwrap();
    tx.drafts()
        .delete_parameter(vid(11), pkey(2))
        .await
        .unwrap();
    drop(tx);
    let state = store.state();
    assert_eq!(state.version_row_version(vid(11)), Some(1));
    assert!(state.editors_of(vid(11)).is_empty());
    let after = state.parameters_of(vid(11));
    assert_eq!(after.len(), before.len());
    assert_eq!(
        after.iter().map(|p| p.value.clone()).collect::<Vec<_>>(),
        before.iter().map(|p| p.value.clone()).collect::<Vec<_>>()
    );
}
