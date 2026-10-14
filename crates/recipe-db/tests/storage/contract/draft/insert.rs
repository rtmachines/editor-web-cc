//! Contract cases of `insert_parameter` and `update_definition`.

use recipe_app::error::{PARAMETER_TAG_UNIQUE, StorageError};
use recipe_domain::plant::Primitive;
use recipe_domain::recipe::{Ordinal, ParameterTag, StoredParameter, StoredValue};
use recipe_domain::text::{LongText, Text};
use serde_json::{Value, json};

use super::fixtures::{
    assert_missing_row, assert_not_live_draft, commit_draft, domain, full_parameter, keys,
    live_draft, missing_key, number, parameter_shape, read_back,
};
use crate::contract::harness::{Harness, begin};
use crate::contract::images::{Kind, assert_shape, field, text};
use crate::contract::recipe_seed::{approve, describe, parameter, range, seed, version_id};
use crate::contract::seed::seed_type;

async fn insert_parameter_gives_the_image_with_the_limits_and_no_translations(h: &dyn Harness) {
    let d = live_draft(h, 10).await;
    // A new parameter has no translations (`Parameter::from_new`), and the port writes none.
    let mut stored = full_parameter(&d.plant, 1, "FULL");
    stored.translations = Vec::new();
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    tx.drafts().touch_draft(d.version, h.user()).await.unwrap();

    let image = tx
        .drafts()
        .insert_parameter(d.version, &domain(&stored))
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;

    assert_shape(&image, &parameter_shape(Kind::String));
    assert_eq!(text(&image, "tag"), "FULL");
    assert_eq!(text(&image, "recipe_version_id"), d.version.to_string());
    assert_eq!(text(&image, "plc_platform"), "TwinCat3");
    assert_eq!(text(&image, "parameter_key"), stored.key.to_string());
    let limits = field(&image, "limits").as_array().unwrap();
    assert_eq!(limits.len(), 2);
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
    assert_eq!(limits[0]["leaf_path"], json!(""));
    assert_eq!(limits[0]["min_value"], number("0"));
    assert_eq!(limits[0]["max_value"], number("10.25"));
    assert_eq!(limits[0]["max_length"], Value::Null);
    assert_eq!(limits[1]["leaf_path"], json!(".B"));
    assert_eq!(limits[1]["max_length"], json!(12));
    assert_eq!(limits[1]["min_value"], Value::Null);
    assert!(field(&image, "translations").as_array().unwrap().is_empty());
    let draft = read_back(h, d.version).await;
    assert_eq!(describe(&draft.parameters), describe(&[stored]));
}

async fn insert_parameter_keeps_the_canonical_decimal_text(h: &dyn Harness) {
    let d = live_draft(h, 11).await;
    let digits28 = "1234567890123456789012345678";
    let mut stored = Vec::new();
    for (n, tag, value) in [
        (1, "INT_150", "150"),
        (2, "TENTH", "0.1"),
        (3, "DIGITS_28", digits28),
        (4, "DECIMAL_28", "0.1234567890123456789012345678"),
    ] {
        let mut p = parameter(&d.plant, n, tag);
        p.value = StoredValue::from(json!(value));
        p.limits = vec![range("", "-0.1234567890123456789012345678", digits28)];
        stored.push(p);
    }
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    for p in &stored {
        tx.drafts()
            .insert_parameter(d.version, &domain(p))
            .await
            .unwrap();
    }
    commit_draft(h, &d, tx).await;

    let draft = read_back(h, d.version).await;

    stored.sort_by(|a, b| a.tag.cmp(&b.tag));
    assert_eq!(describe(&draft.parameters), describe(&stored));
    let values: Vec<&Value> = draft.parameters.iter().map(|p| p.value.as_json()).collect();
    assert!(values.iter().all(|value| value.is_string()), "{values:?}");
}

async fn a_known_tag_gives_duplicate_with_the_tag_constraint(h: &dyn Harness) {
    let d = live_draft(h, 12).await;
    seed(h, d.version, &parameter(&d.plant, 1, "SAME")).await;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    let error = tx
        .drafts()
        .insert_parameter(d.version, &domain(&parameter(&d.plant, 2, "SAME")))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == PARAMETER_TAG_UNIQUE),
        "{error:?}"
    );
}

async fn an_insert_into_an_approved_version_gives_e_rcp_031(h: &dyn Harness) {
    let d = live_draft(h, 13).await;
    approve(h, d.version).await;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    let error = tx
        .drafts()
        .insert_parameter(d.version, &domain(&parameter(&d.plant, 1, "LATE")))
        .await
        .unwrap_err();

    assert_not_live_draft(&error);
}

async fn an_insert_into_a_missing_version_gives_the_version_platform_key(h: &dyn Harness) {
    let d = live_draft(h, 14).await;
    let mut tx = begin(h).await;

    let error = tx
        .drafts()
        .insert_parameter(version_id(98), &domain(&parameter(&d.plant, 1, "ORPHAN")))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::MissingReference { constraint }
            if constraint == "parameter_version_platform"),
        "{error:?}"
    );
}

/// The stored parameter after a definition change: the definition of `changed`, and the value,
/// the limits, and the translations of `original`.
fn with_definition_of(original: &StoredParameter, changed: &StoredParameter) -> StoredParameter {
    StoredParameter {
        tag: changed.tag.clone(),
        name_en: changed.name_en.clone(),
        description_en: changed.description_en.clone(),
        plc_type: changed.plc_type,
        ordinal: changed.ordinal,
        ..original.clone()
    }
}

async fn update_definition_keeps_the_value_the_limits_and_the_translations(h: &dyn Harness) {
    let d = live_draft(h, 15).await;
    let original = full_parameter(&d.plant, 1, "OLD");
    seed(h, d.version, &original).await;
    let int = seed_type(h, d.plant.machine, Primitive::Int).await;
    let mut changed = original.clone();
    changed.tag = ParameterTag::parse("NEW").unwrap();
    changed.name_en = Text::parse("Renamed").unwrap();
    changed.description_en = LongText::parse("Other").unwrap();
    changed.plc_type = int;
    changed.ordinal = Ordinal::parse(7).unwrap();
    changed.value = StoredValue::from(json!("999"));
    changed.limits = Vec::new();
    changed.translations = Vec::new();
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    tx.drafts().touch_draft(d.version, h.user()).await.unwrap();

    tx.drafts()
        .update_definition(d.version, &domain(&changed))
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;

    let draft = read_back(h, d.version).await;
    let expected = with_definition_of(&original, &changed);
    assert_eq!(describe(&draft.parameters), describe(&[expected]));
}

async fn update_definition_with_a_known_tag_gives_duplicate(h: &dyn Harness) {
    let d = live_draft(h, 16).await;
    seed(h, d.version, &parameter(&d.plant, 1, "FIRST")).await;
    let second = parameter(&d.plant, 2, "SECOND");
    seed(h, d.version, &second).await;
    let mut renamed = second;
    renamed.tag = ParameterTag::parse("FIRST").unwrap();
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    let error = tx
        .drafts()
        .update_definition(d.version, &domain(&renamed))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == PARAMETER_TAG_UNIQUE),
        "{error:?}"
    );
}

async fn update_definition_in_an_approved_version_gives_e_rcp_031(h: &dyn Harness) {
    let d = live_draft(h, 17).await;
    let stored = parameter(&d.plant, 1, "FIXED");
    seed(h, d.version, &stored).await;
    approve(h, d.version).await;
    let mut renamed = stored;
    renamed.name_en = Text::parse("Late").unwrap();
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    let error = tx
        .drafts()
        .update_definition(d.version, &domain(&renamed))
        .await
        .unwrap_err();

    assert_not_live_draft(&error);
}

async fn update_definition_of_a_missing_parameter_gives_a_storage_error(h: &dyn Harness) {
    let d = live_draft(h, 18).await;
    let mut ghost = parameter(&d.plant, 1, "GHOST");
    ghost.key = missing_key();
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    let error = tx
        .drafts()
        .update_definition(d.version, &domain(&ghost))
        .await
        .unwrap_err();

    assert_missing_row(&error);
}

contract_case!(
    insert_parameter_gives_the_image_with_the_limits_and_no_translations,
    insert_parameter_gives_the_image_with_the_limits_and_no_translations_memory,
    insert_parameter_gives_the_image_with_the_limits_and_no_translations_postgres
);
contract_case!(
    insert_parameter_keeps_the_canonical_decimal_text,
    insert_parameter_keeps_the_canonical_decimal_text_memory,
    insert_parameter_keeps_the_canonical_decimal_text_postgres
);
contract_case!(
    a_known_tag_gives_duplicate_with_the_tag_constraint,
    a_known_tag_gives_duplicate_with_the_tag_constraint_memory,
    a_known_tag_gives_duplicate_with_the_tag_constraint_postgres
);
contract_case!(
    an_insert_into_an_approved_version_gives_e_rcp_031,
    an_insert_into_an_approved_version_gives_e_rcp_031_memory,
    an_insert_into_an_approved_version_gives_e_rcp_031_postgres
);
contract_case!(
    an_insert_into_a_missing_version_gives_the_version_platform_key,
    an_insert_into_a_missing_version_gives_the_version_platform_key_memory,
    an_insert_into_a_missing_version_gives_the_version_platform_key_postgres
);
contract_case!(
    update_definition_keeps_the_value_the_limits_and_the_translations,
    update_definition_keeps_the_value_the_limits_and_the_translations_memory,
    update_definition_keeps_the_value_the_limits_and_the_translations_postgres
);
contract_case!(
    update_definition_with_a_known_tag_gives_duplicate,
    update_definition_with_a_known_tag_gives_duplicate_memory,
    update_definition_with_a_known_tag_gives_duplicate_postgres
);
contract_case!(
    update_definition_in_an_approved_version_gives_e_rcp_031,
    update_definition_in_an_approved_version_gives_e_rcp_031_memory,
    update_definition_in_an_approved_version_gives_e_rcp_031_postgres
);
contract_case!(
    update_definition_of_a_missing_parameter_gives_a_storage_error,
    update_definition_of_a_missing_parameter_gives_a_storage_error_memory,
    update_definition_of_a_missing_parameter_gives_a_storage_error_postgres
);
