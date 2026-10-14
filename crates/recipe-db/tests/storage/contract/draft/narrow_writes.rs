//! Contract cases of `set_value`, `replace_limits`, and `upsert_translation`.
//!
//! Each write changes its own rows and leaves the others unchanged.

use recipe_app::error::StorageError;
use recipe_domain::recipe::StoredValue;
use serde_json::json;

use super::fixtures::{commit_draft, full_parameter, live_draft, read_back};
use crate::contract::harness::{Harness, begin};
use crate::contract::recipe_seed::{describe, length, range, seed, translation};

async fn set_value_changes_only_the_value(h: &dyn Harness) {
    let d = live_draft(h, 40).await;
    let original = full_parameter(&d.plant, 1, "VALUE");
    seed(h, d.version, &original).await;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    tx.drafts().touch_draft(d.version, h.user()).await.unwrap();

    tx.drafts()
        .set_value(d.version, original.key, &StoredValue::from(json!("42")))
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;

    let mut expected = original;
    expected.value = StoredValue::from(json!("42"));
    let draft = read_back(h, d.version).await;
    assert_eq!(describe(&draft.parameters), describe(&[expected]));
}

async fn set_value_keeps_the_canonical_decimal_text(h: &dyn Harness) {
    let d = live_draft(h, 41).await;
    let original = full_parameter(&d.plant, 1, "DECIMAL");
    seed(h, d.version, &original).await;
    for text in ["150", "0.1", "1234567890123456789012345678"] {
        let mut tx = begin(h).await;
        tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
        tx.drafts()
            .set_value(d.version, original.key, &StoredValue::from(json!(text)))
            .await
            .unwrap();
        commit_draft(h, &d, tx).await;

        let draft = read_back(h, d.version).await;

        assert_eq!(draft.parameters[0].value.as_json(), &json!(text));
    }
}

async fn replace_limits_replaces_the_limit_rows_and_keeps_the_value(h: &dyn Harness) {
    let d = live_draft(h, 42).await;
    let original = full_parameter(&d.plant, 1, "LIMITS");
    seed(h, d.version, &original).await;
    let replacement = vec![range("", "1", "2"), range(".C", "-3.50", "4")];
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    tx.drafts().touch_draft(d.version, h.user()).await.unwrap();

    tx.drafts()
        .replace_limits(d.version, original.key, &replacement)
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;

    let mut expected = original;
    expected.limits = replacement;
    let draft = read_back(h, d.version).await;
    assert_eq!(describe(&draft.parameters), describe(&[expected]));
}

async fn replace_limits_with_no_limit_removes_every_limit_row(h: &dyn Harness) {
    let d = live_draft(h, 43).await;
    let original = full_parameter(&d.plant, 1, "CLEAR");
    seed(h, d.version, &original).await;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    tx.drafts()
        .replace_limits(d.version, original.key, &[])
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;

    let mut expected = original;
    expected.limits = Vec::new();
    let draft = read_back(h, d.version).await;
    assert_eq!(describe(&draft.parameters), describe(&[expected]));
}

async fn replace_limits_with_a_repeated_path_gives_duplicate(h: &dyn Harness) {
    let d = live_draft(h, 44).await;
    let original = full_parameter(&d.plant, 1, "TWICE");
    seed(h, d.version, &original).await;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    let error = tx
        .drafts()
        .replace_limits(d.version, original.key, &[length(".B", 5), length(".B", 6)])
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint }
            if constraint == "parameter_limit_pkey"),
        "{error:?}"
    );
}

async fn upsert_translation_inserts_and_then_updates_the_same_language(h: &dyn Harness) {
    let d = live_draft(h, 45).await;
    let original = full_parameter(&d.plant, 1, "WORDS");
    seed(h, d.version, &original).await;
    let first = translation("fr", "Nom", Some("Description"));
    let second = translation("fr", "Nom 2", None);
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    tx.drafts()
        .upsert_translation(d.version, original.key, &first)
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;
    let inserted = read_back(h, d.version).await;

    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    tx.drafts()
        .upsert_translation(d.version, original.key, &second)
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;
    let updated = read_back(h, d.version).await;

    let mut after_insert = original.clone();
    after_insert.translations = vec![
        translation("de", "Name", Some("Beschreibung")),
        first.clone(),
        translation("pl", "Nazwa", None),
    ];
    let mut after_update = original;
    after_update.translations = vec![
        translation("de", "Name", Some("Beschreibung")),
        second,
        translation("pl", "Nazwa", None),
    ];
    assert_eq!(describe(&inserted.parameters), describe(&[after_insert]));
    assert_eq!(describe(&updated.parameters), describe(&[after_update]));
}

contract_case!(
    set_value_changes_only_the_value,
    set_value_changes_only_the_value_memory,
    set_value_changes_only_the_value_postgres
);
contract_case!(
    set_value_keeps_the_canonical_decimal_text,
    set_value_keeps_the_canonical_decimal_text_memory,
    set_value_keeps_the_canonical_decimal_text_postgres
);
contract_case!(
    replace_limits_replaces_the_limit_rows_and_keeps_the_value,
    replace_limits_replaces_the_limit_rows_and_keeps_the_value_memory,
    replace_limits_replaces_the_limit_rows_and_keeps_the_value_postgres
);
contract_case!(
    replace_limits_with_no_limit_removes_every_limit_row,
    replace_limits_with_no_limit_removes_every_limit_row_memory,
    replace_limits_with_no_limit_removes_every_limit_row_postgres
);
contract_case!(
    replace_limits_with_a_repeated_path_gives_duplicate,
    replace_limits_with_a_repeated_path_gives_duplicate_memory,
    replace_limits_with_a_repeated_path_gives_duplicate_postgres
);
contract_case!(
    upsert_translation_inserts_and_then_updates_the_same_language,
    upsert_translation_inserts_and_then_updates_the_same_language_memory,
    upsert_translation_inserts_and_then_updates_the_same_language_postgres
);
