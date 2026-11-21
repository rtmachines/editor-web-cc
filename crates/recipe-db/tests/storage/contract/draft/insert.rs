//! Contract cases of `insert_parameter` and `update_definition`.

use recipe_app::error::{PARAMETER_TAG_UNIQUE, StorageError};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{
    DataBlockNumber, Machine, PlcPlatform, Primitive, S7Address, StringEncoding, StringKind,
};
use recipe_domain::recipe::{
    LIMIT_ROWS_PER_VERSION, Ordinal, PARAMETERS_PER_VERSION, ParameterTag, StoredLimit,
    StoredParameter, StoredValue,
};
use recipe_domain::text::{LongText, Text};
use serde_json::{Map, Value, json};

use super::fixtures::{
    Draft, assert_missing_row, assert_not_live_draft, commit_draft, domain, full_parameter, keys,
    live_draft, missing_key, number, parameter_shape, read_back,
};
use crate::contract::harness::{Harness, begin};
use crate::contract::images::{Kind, assert_shape, field, text};
use crate::contract::recipe_seed::{
    Plant, approve, create_recipe, describe, length, parameter, plant, range, seed, version_id,
};
use crate::contract::seed::{
    code, ensure, insert, machine, machine_id, name, s7_machine, seed_type, string_record, type_id,
    unit_id,
};

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

    assert_shape(&image, &parameter_shape(Kind::Number));
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
    // The case tests the precision of the JSONB number.
    // Storage does not check the value against the type.
    // The `DINT` type only holds the values.
    let mut stored = Vec::new();
    for (n, tag, value) in [
        (1, "INT_150", "150"),
        (2, "TENTH", "0.1"),
        (3, "DIGITS_28", digits28),
        (4, "DECIMAL_28", "0.1234567890123456789012345678"),
    ] {
        let mut p = parameter(&d.plant, n, tag);
        p.value = StoredValue::from(number(value));
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
    assert!(values.iter().all(|value| value.is_number()), "{values:?}");
}

/// The five 64-bit extreme leaves of a value, each with the canonical text that the image must keep.
const EXTREMES: [(&str, &str); 5] = [
    ("ulint", "18446744073709551615"),
    ("lword", "18446744073709551615"),
    ("lint", "-9223372036854775808"),
    ("ltime", "-9223372036854775808"),
    ("ldt", "9223372036854775807"),
];

async fn insert_parameter_keeps_the_64_bit_extremes_in_the_image(h: &dyn Harness) {
    let d = live_draft(h, 19).await;
    // Storage does not check the value against the type, so the `DINT` fixture holds the object.
    let written: Map<String, Value> = EXTREMES
        .iter()
        .map(|(key, text)| ((*key).to_owned(), number(text)))
        .collect();
    let mut stored = parameter(&d.plant, 1, "EXTREME");
    stored.value = StoredValue::from(Value::Object(written.clone()));
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    tx.drafts().touch_draft(d.version, h.user()).await.unwrap();

    let image = tx
        .drafts()
        .insert_parameter(d.version, &domain(&stored))
        .await
        .unwrap();
    commit_draft(h, &d, tx).await;

    let read = field(&image, "value");
    assert_eq!(read, &Value::Object(written.clone()));
    assert_eq!(read.to_string(), Value::Object(written).to_string());
    for (key, text) in EXTREMES {
        let leaf = read.get(key).unwrap();
        assert_eq!(leaf.to_string(), text, "{key}");
    }
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
    changed.value = StoredValue::from(json!(999));
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

/// Assert that an error is the trigger error or the `CHECK` error with the code `expected`.
fn assert_constraint_code(error: &StorageError, expected: ErrorCode) {
    assert!(
        matches!(error, StorageError::Constraint { code, .. } if *code == expected),
        "{error:?}"
    );
}

/// Assert the message text of a constraint error. Both adapters give the text of `reliability.md` 1.2.
fn assert_constraint_message(error: &StorageError, expected: &str) {
    assert!(
        matches!(error, StorageError::Constraint { message, .. } if message == expected),
        "{error:?}"
    );
}

/// A recipe with a live draft on a `S7_300` machine. The platform needs an S7 address.
async fn s7_300_draft(h: &dyn Harness, n: u128) -> Draft {
    let built = Machine::create(
        machine_id(n),
        code(&format!("M{n}")),
        name("Press"),
        PlcPlatform::S7_300,
        Some(DataBlockNumber::parse("10").unwrap()),
        StringEncoding::Latin1,
        (unit_id(n.checked_mul(10).unwrap()), name("Main unit")),
    )
    .unwrap();
    insert(h, &built).await;
    let unit = built.units().next().unwrap().id();
    let plant = Plant {
        machine: built.id(),
        unit,
        dint: seed_type(h, built.id(), Primitive::Dint).await,
    };
    let created = create_recipe(h, &plant, n, &format!("REF-{n}")).await;
    Draft {
        plant,
        version: created.versions()[0].id(),
    }
}

/// Insert `stored` into the draft in one transaction and give the error of the port.
async fn insert_error(h: &dyn Harness, d: &Draft, stored: &StoredParameter) -> StorageError {
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    tx.drafts()
        .insert_parameter(d.version, &domain(stored))
        .await
        .unwrap_err()
}

/// `count` limit rows with the leaf paths `[0]`, `[1]`, and so on.
fn limit_rows(count: u64) -> Vec<StoredLimit> {
    (0..count).map(|i| length(&format!("[{i}]"), 1)).collect()
}

async fn a_unit_of_another_machine_gives_e_rcp_032(h: &dyn Harness) {
    let d = live_draft(h, 20).await;
    let other = plant(h, 21, "M21").await;
    let mut foreign = parameter(&d.plant, 1, "FOREIGN_UNIT");
    foreign.unit = other.unit;

    let error = insert_error(h, &d, &foreign).await;

    assert_constraint_code(&error, ErrorCode::RCP_032);
}

async fn a_type_of_another_machine_gives_e_plt_032(h: &dyn Harness) {
    let d = live_draft(h, 22).await;
    let other = machine(23, "M23");
    insert(h, &other).await;
    let record = string_record(&other, type_id(1), StringKind::String, 10);
    ensure(h, other.id(), record).await;
    let mut foreign = parameter(&d.plant, 1, "FOREIGN_TYPE");
    foreign.plc_type = type_id(1);

    let error = insert_error(h, &d, &foreign).await;

    assert_constraint_code(&error, ErrorCode::PLT_032);
}

async fn a_type_of_another_platform_gives_e_plt_031(h: &dyn Harness) {
    let d = live_draft(h, 24).await;
    let other = s7_machine(25, "M25", "37");
    insert(h, &other).await;
    let record = string_record(&other, type_id(2), StringKind::String, 10);
    ensure(h, other.id(), record).await;
    let mut foreign = parameter(&d.plant, 1, "FOREIGN_PLATFORM");
    foreign.plc_type = type_id(2);

    let error = insert_error(h, &d, &foreign).await;

    assert_constraint_code(&error, ErrorCode::PLT_031);
}

async fn an_s7_address_on_a_twincat3_version_breaks_the_platform_check(h: &dyn Harness) {
    let d = live_draft(h, 26).await;
    let mut addressed = parameter(&d.plant, 1, "ADDRESSED");
    addressed.s7_address = Some(S7Address::parse("DB1.DBD0").unwrap());

    let error = insert_error(h, &d, &addressed).await;

    assert_constraint_code(&error, ErrorCode::DB_003);
}

async fn no_s7_address_on_an_s7_300_version_breaks_the_platform_check(h: &dyn Harness) {
    let d = s7_300_draft(h, 27).await;
    let bare = parameter(&d.plant, 1, "BARE");

    let error = insert_error(h, &d, &bare).await;

    assert_constraint_code(&error, ErrorCode::DB_003);
}

async fn the_parameter_above_the_budget_gives_e_rcp_035(h: &dyn Harness) {
    let d = live_draft(h, 28).await;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    for n in 1..=u128::from(PARAMETERS_PER_VERSION) {
        let stored = parameter(&d.plant, n, &format!("P{n}"));
        tx.drafts()
            .insert_parameter(d.version, &domain(&stored))
            .await
            .unwrap();
    }
    let one_too_many = parameter(&d.plant, u128::from(PARAMETERS_PER_VERSION) + 1, "ONE_MORE");

    let error = tx
        .drafts()
        .insert_parameter(d.version, &domain(&one_too_many))
        .await
        .unwrap_err();

    assert_constraint_code(&error, ErrorCode::RCP_035);
    assert_constraint_message(&error, "The version has more than 4 000 parameters.");
}

async fn an_existing_key_at_the_full_budget_gives_the_unique_violation(h: &dyn Harness) {
    let d = live_draft(h, 30).await;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    let mut first_key = None;
    for n in 1..=u128::from(PARAMETERS_PER_VERSION) {
        let stored = parameter(&d.plant, n, &format!("P{n}"));
        first_key.get_or_insert(stored.key);
        tx.drafts()
            .insert_parameter(d.version, &domain(&stored))
            .await
            .unwrap();
    }
    // PostgreSQL checks the unique constraint of the row before the statement trigger of the budget.
    let mut same_key = parameter(&d.plant, u128::from(PARAMETERS_PER_VERSION) + 1, "ONE_MORE");
    same_key.key = first_key.unwrap();

    let error = tx
        .drafts()
        .insert_parameter(d.version, &domain(&same_key))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == "parameter_key_unique"),
        "{error:?}"
    );
}

async fn the_limit_rows_above_the_budget_give_e_rcp_036(h: &dyn Harness) {
    let d = live_draft(h, 29).await;
    let part = LIMIT_ROWS_PER_VERSION / 4;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    for n in 1..=4 {
        let mut stored = parameter(&d.plant, n, &format!("P{n}"));
        stored.limits = limit_rows(part);
        tx.drafts()
            .insert_parameter(d.version, &domain(&stored))
            .await
            .unwrap();
    }
    let mut one_more = parameter(&d.plant, 5, "ONE_MORE");
    one_more.limits = limit_rows(1);

    let error = tx
        .drafts()
        .insert_parameter(d.version, &domain(&one_more))
        .await
        .unwrap_err();

    assert_constraint_code(&error, ErrorCode::RCP_036);
    assert_constraint_message(&error, "The version has more than 32 768 limit rows.");
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
    insert_parameter_keeps_the_64_bit_extremes_in_the_image,
    insert_parameter_keeps_the_64_bit_extremes_in_the_image_memory,
    insert_parameter_keeps_the_64_bit_extremes_in_the_image_postgres
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
contract_case!(
    a_unit_of_another_machine_gives_e_rcp_032,
    a_unit_of_another_machine_gives_e_rcp_032_memory,
    a_unit_of_another_machine_gives_e_rcp_032_postgres
);
contract_case!(
    a_type_of_another_machine_gives_e_plt_032,
    a_type_of_another_machine_gives_e_plt_032_memory,
    a_type_of_another_machine_gives_e_plt_032_postgres
);
contract_case!(
    a_type_of_another_platform_gives_e_plt_031,
    a_type_of_another_platform_gives_e_plt_031_memory,
    a_type_of_another_platform_gives_e_plt_031_postgres
);
contract_case!(
    an_s7_address_on_a_twincat3_version_breaks_the_platform_check,
    an_s7_address_on_a_twincat3_version_breaks_the_platform_check_memory,
    an_s7_address_on_a_twincat3_version_breaks_the_platform_check_postgres
);
contract_case!(
    no_s7_address_on_an_s7_300_version_breaks_the_platform_check,
    no_s7_address_on_an_s7_300_version_breaks_the_platform_check_memory,
    no_s7_address_on_an_s7_300_version_breaks_the_platform_check_postgres
);
contract_case!(
    the_parameter_above_the_budget_gives_e_rcp_035,
    the_parameter_above_the_budget_gives_e_rcp_035_memory,
    the_parameter_above_the_budget_gives_e_rcp_035_postgres
);
contract_case!(
    the_limit_rows_above_the_budget_give_e_rcp_036,
    the_limit_rows_above_the_budget_give_e_rcp_036_memory,
    the_limit_rows_above_the_budget_give_e_rcp_036_postgres
);
contract_case!(
    an_existing_key_at_the_full_budget_gives_the_unique_violation,
    an_existing_key_at_the_full_budget_gives_the_unique_violation_memory,
    an_existing_key_at_the_full_budget_gives_the_unique_violation_postgres
);
