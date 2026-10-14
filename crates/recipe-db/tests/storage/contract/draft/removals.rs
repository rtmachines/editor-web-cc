//! Contract cases of `delete_translation` and `delete_parameter`, and the error cases of every
//! narrow write: a version that is not a live draft, and a parameter that does not exist.

use recipe_app::ports::Transaction;
use recipe_domain::recipe::{LangCode, StoredValue};
use serde_json::{Value, json};

use super::fixtures::{
    Draft, assert_missing_row, assert_not_live_draft, commit_draft, full_parameter, keys,
    live_draft, missing_key, number, parameter_shape, read_back,
};
use crate::contract::harness::{Harness, begin};
use crate::contract::images::{Kind, assert_shape, field, text};
use crate::contract::recipe_seed::{approve, describe, length, seed, translation};

/// Open a transaction that holds the lock of the draft.
async fn locked(h: &dyn Harness, draft: &Draft) -> Box<dyn Transaction> {
    let mut tx = begin(h).await;
    tx.drafts()
        .lock_draft(draft.version)
        .await
        .unwrap()
        .unwrap();
    tx
}

async fn delete_translation_gives_the_image_of_the_deleted_row(h: &dyn Harness) {
    let d = live_draft(h, 50).await;
    let original = full_parameter(&d.plant, 1, "WORDS");
    seed(h, d.version, &original).await;
    let lang = LangCode::parse("de").unwrap();
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    tx.drafts().touch_draft(d.version, h.user()).await.unwrap();

    let image = tx
        .drafts()
        .delete_translation(d.version, original.key, &lang)
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;

    assert_shape(
        &image,
        &[
            ("parameter_id", Kind::String),
            ("recipe_version_id", Kind::String),
            ("lang", Kind::String),
            ("name", Kind::String),
            ("description", Kind::String),
            ("created_at", Kind::String),
        ],
    );
    assert_eq!(text(&image, "parameter_id"), original.id.to_string());
    assert_eq!(text(&image, "lang"), "de");
    assert_eq!(text(&image, "name"), "Name");
    assert_eq!(text(&image, "description"), "Beschreibung");
    let mut expected = original;
    expected.translations = vec![translation("pl", "Nazwa", None)];
    let draft = read_back(h, d.version).await;
    assert_eq!(describe(&draft.parameters), describe(&[expected]));
}

async fn delete_translation_of_a_missing_language_gives_a_storage_error(h: &dyn Harness) {
    let d = live_draft(h, 51).await;
    let original = full_parameter(&d.plant, 1, "WORDS");
    seed(h, d.version, &original).await;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    let error = tx
        .drafts()
        .delete_translation(d.version, original.key, &LangCode::parse("fr").unwrap())
        .await
        .unwrap_err();

    assert_missing_row(&error);
}

async fn delete_parameter_gives_the_image_with_limits_and_translations(h: &dyn Harness) {
    let d = live_draft(h, 52).await;
    let original = full_parameter(&d.plant, 1, "GONE");
    let other = full_parameter(&d.plant, 2, "STAYS");
    seed(h, d.version, &original).await;
    seed(h, d.version, &other).await;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    tx.drafts().touch_draft(d.version, h.user()).await.unwrap();

    let image = tx
        .drafts()
        .delete_parameter(d.version, original.key)
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;

    assert_shape(&image, &parameter_shape(Kind::String));
    assert_eq!(text(&image, "tag"), "GONE");
    assert_eq!(text(&image, "id"), original.id.to_string());
    let limits = field(&image, "limits").as_array().unwrap();
    assert_eq!(limits.len(), 2);
    assert_eq!(limits[0]["leaf_path"], json!(""));
    assert_eq!(limits[0]["min_value"], number("0"));
    assert_eq!(limits[0]["max_value"], number("10.25"));
    assert_eq!(limits[1]["leaf_path"], json!(".B"));
    assert_eq!(limits[1]["max_length"], json!(12));
    assert_eq!(
        keys(&limits[0]),
        [
            "created_at",
            "leaf_path",
            "max_length",
            "max_value",
            "min_value",
            "parameter_id",
            "recipe_version_id"
        ]
    );
    let translations = field(&image, "translations").as_array().unwrap();
    assert_eq!(translations.len(), 2);
    assert_eq!(translations[0]["lang"], json!("de"));
    assert_eq!(translations[0]["description"], json!("Beschreibung"));
    assert_eq!(translations[1]["lang"], json!("pl"));
    assert_eq!(translations[1]["description"], json!(""));
    assert_eq!(
        keys(&translations[0]),
        [
            "created_at",
            "description",
            "lang",
            "name",
            "parameter_id",
            "recipe_version_id"
        ]
    );
    let draft = read_back(h, d.version).await;
    assert_eq!(describe(&draft.parameters), describe(&[other]));
}

async fn every_write_in_an_approved_version_gives_e_rcp_031(h: &dyn Harness) {
    let d = live_draft(h, 53).await;
    let original = full_parameter(&d.plant, 1, "FROZEN");
    seed(h, d.version, &original).await;
    approve(h, d.version).await;
    let lang = LangCode::parse("de").unwrap();
    let key = original.key;

    // A failed statement aborts a PostgreSQL transaction, so each write gets its own transaction.
    let mut tx = locked(h, &d).await;
    let set_value = tx
        .drafts()
        .set_value(d.version, key, &StoredValue::from(json!("7")))
        .await
        .unwrap_err();
    drop(tx);
    let mut tx = locked(h, &d).await;
    let replace_limits = tx
        .drafts()
        .replace_limits(d.version, key, &[length(".B", 3)])
        .await
        .unwrap_err();
    drop(tx);
    let mut tx = locked(h, &d).await;
    let upsert = tx
        .drafts()
        .upsert_translation(d.version, key, &translation("fr", "Nom", None))
        .await
        .unwrap_err();
    drop(tx);
    let mut tx = locked(h, &d).await;
    let delete_translation = tx
        .drafts()
        .delete_translation(d.version, key, &lang)
        .await
        .unwrap_err();
    drop(tx);
    let mut tx = locked(h, &d).await;
    let delete_parameter = tx
        .drafts()
        .delete_parameter(d.version, key)
        .await
        .unwrap_err();
    drop(tx);

    for error in [
        &set_value,
        &replace_limits,
        &upsert,
        &delete_translation,
        &delete_parameter,
    ] {
        assert_not_live_draft(error);
    }
    let draft = read_back(h, d.version).await;
    assert_eq!(describe(&draft.parameters), describe(&[original]));
}

async fn every_write_to_a_missing_parameter_gives_a_storage_error(h: &dyn Harness) {
    let d = live_draft(h, 54).await;
    let key = missing_key();
    let mut tx = locked(h, &d).await;

    let set_value = tx
        .drafts()
        .set_value(d.version, key, &StoredValue::from(Value::Null))
        .await
        .unwrap_err();
    let replace_limits = tx
        .drafts()
        .replace_limits(d.version, key, &[])
        .await
        .unwrap_err();
    let upsert = tx
        .drafts()
        .upsert_translation(d.version, key, &translation("fr", "Nom", None))
        .await
        .unwrap_err();
    let delete_translation = tx
        .drafts()
        .delete_translation(d.version, key, &LangCode::parse("fr").unwrap())
        .await
        .unwrap_err();
    let delete_parameter = tx
        .drafts()
        .delete_parameter(d.version, key)
        .await
        .unwrap_err();

    for error in [
        &set_value,
        &replace_limits,
        &upsert,
        &delete_translation,
        &delete_parameter,
    ] {
        assert_missing_row(error);
    }
}

contract_case!(
    delete_translation_gives_the_image_of_the_deleted_row,
    delete_translation_gives_the_image_of_the_deleted_row_memory,
    delete_translation_gives_the_image_of_the_deleted_row_postgres
);
contract_case!(
    delete_translation_of_a_missing_language_gives_a_storage_error,
    delete_translation_of_a_missing_language_gives_a_storage_error_memory,
    delete_translation_of_a_missing_language_gives_a_storage_error_postgres
);
contract_case!(
    delete_parameter_gives_the_image_with_limits_and_translations,
    delete_parameter_gives_the_image_with_limits_and_translations_memory,
    delete_parameter_gives_the_image_with_limits_and_translations_postgres
);
contract_case!(
    every_write_in_an_approved_version_gives_e_rcp_031,
    every_write_in_an_approved_version_gives_e_rcp_031_memory,
    every_write_in_an_approved_version_gives_e_rcp_031_postgres
);
contract_case!(
    every_write_to_a_missing_parameter_gives_a_storage_error,
    every_write_to_a_missing_parameter_gives_a_storage_error_memory,
    every_write_to_a_missing_parameter_gives_a_storage_error_postgres
);
