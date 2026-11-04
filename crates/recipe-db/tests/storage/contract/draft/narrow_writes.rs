//! Contract cases of `set_value`, `replace_limits`, and `upsert_translation`.
//!
//! Each write changes its own rows and leaves the others unchanged.

use recipe_app::error::StorageError;
use recipe_domain::recipe::StoredValue;
use serde_json::{Map, Value, json};

use super::fixtures::{commit_draft, full_parameter, live_draft, number, read_back};
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
        .set_value(d.version, original.key, &StoredValue::from(json!(42)))
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;

    let mut expected = original;
    expected.value = StoredValue::from(json!(42));
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
            .set_value(d.version, original.key, &StoredValue::from(number(text)))
            .await
            .unwrap();
        commit_draft(h, &d, tx).await;

        let draft = read_back(h, d.version).await;

        assert_eq!(draft.parameters[0].value.as_json(), &number(text));
    }
}

/// The five 64-bit extreme leaves of a value, each with the canonical text that storage must keep.
const EXTREMES: [(&str, &str); 5] = [
    ("ulint", "18446744073709551615"),
    ("lword", "18446744073709551615"),
    ("lint", "-9223372036854775808"),
    ("ltime", "-9223372036854775808"),
    ("ldt", "9223372036854775807"),
];

async fn set_value_keeps_the_64_bit_extremes(h: &dyn Harness) {
    let d = live_draft(h, 46).await;
    let original = full_parameter(&d.plant, 1, "EXTREME");
    seed(h, d.version, &original).await;
    let mut written: Vec<Value> = EXTREMES.iter().map(|(_, text)| number(text)).collect();
    let object: Map<String, Value> = EXTREMES
        .iter()
        .map(|(key, text)| ((*key).to_owned(), number(text)))
        .collect();
    written.push(Value::Object(object));
    for value in written {
        let mut tx = begin(h).await;
        tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
        tx.drafts()
            .set_value(d.version, original.key, &StoredValue::from(value.clone()))
            .await
            .unwrap();
        commit_draft(h, &d, tx).await;

        let draft = read_back(h, d.version).await;

        let read = draft.parameters[0].value.as_json();
        assert_eq!(read, &value);
        assert_eq!(read.to_string(), value.to_string());
        for (key, text) in EXTREMES {
            if let Some(leaf) = read.get(key) {
                assert_eq!(leaf.to_string(), text, "{key}");
            }
        }
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

async fn replace_limits_keeps_the_64_bit_extremes(h: &dyn Harness) {
    let d = live_draft(h, 47).await;
    let original = full_parameter(&d.plant, 1, "EDGE");
    seed(h, d.version, &original).await;
    let replacement = vec![range("", "-9223372036854775808", "18446744073709551615")];
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    tx.drafts()
        .replace_limits(d.version, original.key, &replacement)
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;

    let draft = read_back(h, d.version).await;
    let limits = &draft.parameters[0].limits;
    assert_eq!(limits.len(), 1);
    assert_eq!(
        limits[0].min.as_ref().map(ToString::to_string).as_deref(),
        Some("-9223372036854775808")
    );
    assert_eq!(
        limits[0].max.as_ref().map(ToString::to_string).as_deref(),
        Some("18446744073709551615")
    );
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
    set_value_keeps_the_64_bit_extremes,
    set_value_keeps_the_64_bit_extremes_memory,
    set_value_keeps_the_64_bit_extremes_postgres
);
contract_case!(
    replace_limits_replaces_the_limit_rows_and_keeps_the_value,
    replace_limits_replaces_the_limit_rows_and_keeps_the_value_memory,
    replace_limits_replaces_the_limit_rows_and_keeps_the_value_postgres
);
contract_case!(
    replace_limits_keeps_the_64_bit_extremes,
    replace_limits_keeps_the_64_bit_extremes_memory,
    replace_limits_keeps_the_64_bit_extremes_postgres
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
