//! Tests of `parameter`, `parameter_limit`, and `parameter_translation` (`data-model.md` sections
//! 3.12 to 3.14, task 7.5).
//!
//! The tests run as `recipe_migrator`, the owner of the tables. Each parameter goes into a live
//! draft (trigger 4.1), and each type belongs to the platform of its machine (trigger 4.2).

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures::{self, Draft, TWIN_CAT};
use crate::support::{
    CHECK_VIOLATION, FOREIGN_KEY_VIOLATION, Role, TestDatabase, UNIQUE_VIOLATION, assert_constraint,
};

/// The constraints and unique indexes that the tests of this module violate.
pub const COVERED: &[&str] = &[
    "parameter_pkey",
    "parameter_unit_id_fkey",
    "parameter_plc_type_id_fkey",
    "parameter_version_platform",
    "parameter_tag_format",
    "parameter_tag_unique",
    "parameter_key_unique",
    "parameter_name_length",
    "parameter_description_length",
    "parameter_s7_address_format",
    "parameter_s7_address_platform",
    "parameter_ordinal_range",
    "parameter_value_size",
    "parameter_limit_pkey",
    "parameter_limit_parameter",
    "parameter_limit_path_format",
    "parameter_limit_min_le_max",
    "parameter_limit_not_empty",
    "parameter_limit_one_kind",
    "parameter_limit_max_length_range",
    "parameter_translation_pkey",
    "parameter_translation_parameter",
    "parameter_translation_lang_format",
    "parameter_translation_name_length",
    "parameter_translation_description_length",
];

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    let database = TestDatabase::create().await;
    database.connect(Role::Migrator).await
}

/// Insert a live draft on one platform. The draft has the unit `U1` and the type `DINT`.
async fn draft_on(conn: &mut PgConnection, platform: &str) -> Draft {
    let user = fixtures::user(conn).await;
    let machine = fixtures::machine(conn, platform).await;
    let unit = fixtures::unit(conn, machine, "U1").await;
    let recipe = fixtures::recipe(conn, machine, platform, user).await;
    let version = fixtures::draft_version(conn, recipe, platform, 1, user).await;
    let dint = fixtures::primitive(conn, "DINT").await;
    Draft {
        user,
        machine,
        unit,
        recipe,
        version,
        dint,
    }
}

// ---- parameter ----

/// The columns of one `parameter` row. A test changes one field of a valid row.
#[derive(Debug, Clone)]
struct ParameterRow {
    platform: &'static str,
    tag: &'static str,
    name: &'static str,
    description: &'static str,
    s7_address: Option<&'static str>,
    ordinal: i32,
    parameter_key: Option<Uuid>,
}

impl ParameterRow {
    /// A parameter of a `TwinCat3` draft with no address, a valid tag, and a valid ordinal.
    fn valid() -> Self {
        Self {
            platform: TWIN_CAT,
            tag: "P_TEMP",
            name: "Temperature",
            description: "A value",
            s7_address: None,
            ordinal: 0,
            parameter_key: None,
        }
    }
}

/// Insert a parameter into the draft. The type is the `DINT` of the draft. Return its id.
async fn insert_parameter(
    conn: &mut PgConnection,
    draft: &Draft,
    row: &ParameterRow,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO parameter \
             (recipe_version_id, plc_platform, parameter_key, unit_id, tag, name_en, \
              description_en, plc_type_id, s7_address, value, ordinal) \
         VALUES ($1, $2, COALESCE($3, gen_random_uuid()), $4, $5, $6, $7, $8, $9, \
                 '0'::jsonb, $10) \
         RETURNING id",
    )
    .bind(draft.version)
    .bind(row.platform)
    .bind(row.parameter_key)
    .bind(draft.unit)
    .bind(row.tag)
    .bind(row.name)
    .bind(row.description)
    .bind(draft.dint)
    .bind(row.s7_address)
    .bind(row.ordinal)
    .fetch_one(conn)
    .await
}

#[tokio::test]
async fn parameter_row_with_valid_values_inserts() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;

    insert_parameter(&mut conn, &draft, &ParameterRow::valid())
        .await
        .unwrap();
}

#[tokio::test]
async fn parameter_pkey_rejects_a_second_row_with_the_same_id() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let id = insert_parameter(&mut conn, &draft, &ParameterRow::valid())
        .await
        .unwrap();

    let result = sqlx::query(
        "INSERT INTO parameter \
             (id, recipe_version_id, plc_platform, unit_id, tag, name_en, description_en, \
              plc_type_id, value) \
         VALUES ($1, $2, 'TwinCat3', $3, 'P_COPY', 'Copy', 'Copy', $4, '0'::jsonb)",
    )
    .bind(id)
    .bind(draft.version)
    .bind(draft.unit)
    .bind(draft.dint)
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "parameter_pkey");
}

/// Trigger 4.3 checks the unit of an inserted parameter first, so an insert cannot reach
/// `parameter_unit_id_fkey`. The owner deletes the unit that a parameter references instead.
#[tokio::test]
async fn parameter_unit_id_fkey_rejects_the_delete_of_a_referenced_unit() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    insert_parameter(&mut conn, &draft, &ParameterRow::valid())
        .await
        .unwrap();

    let result = sqlx::query("DELETE FROM unit WHERE id = $1")
        .bind(draft.unit)
        .execute(&mut conn)
        .await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "parameter_unit_id_fkey");
}

#[tokio::test]
async fn parameter_plc_type_id_fkey_rejects_an_unknown_type() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let unknown = Draft {
        dint: Uuid::nil(),
        ..draft
    };

    let result = insert_parameter(&mut conn, &unknown, &ParameterRow::valid()).await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "parameter_plc_type_id_fkey");
}

#[tokio::test]
async fn parameter_version_platform_rejects_a_platform_that_differs_from_the_version() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let row = ParameterRow {
        platform: "S7_1200",
        ..ParameterRow::valid()
    };

    let result = insert_parameter(&mut conn, &draft, &row).await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "parameter_version_platform");
}

#[tokio::test]
async fn parameter_tag_format_rejects_a_lowercase_tag() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let row = ParameterRow {
        tag: "p_temp",
        ..ParameterRow::valid()
    };

    let result = insert_parameter(&mut conn, &draft, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_tag_format");
}

#[tokio::test]
async fn parameter_tag_unique_rejects_a_second_parameter_with_the_same_tag() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    insert_parameter(&mut conn, &draft, &ParameterRow::valid())
        .await
        .unwrap();

    let result = insert_parameter(&mut conn, &draft, &ParameterRow::valid()).await;

    assert_constraint(result, UNIQUE_VIOLATION, "parameter_tag_unique");
}

#[tokio::test]
async fn parameter_key_unique_rejects_a_second_parameter_with_the_same_key() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let key = Uuid::from_u128(42);
    let first = ParameterRow {
        parameter_key: Some(key),
        ..ParameterRow::valid()
    };
    insert_parameter(&mut conn, &draft, &first).await.unwrap();
    let second = ParameterRow {
        tag: "P_OTHER",
        parameter_key: Some(key),
        ..ParameterRow::valid()
    };

    let result = insert_parameter(&mut conn, &draft, &second).await;

    assert_constraint(result, UNIQUE_VIOLATION, "parameter_key_unique");
}

#[tokio::test]
async fn parameter_name_length_rejects_an_empty_name() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let row = ParameterRow {
        name: "",
        ..ParameterRow::valid()
    };

    let result = insert_parameter(&mut conn, &draft, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_name_length");
}

#[tokio::test]
async fn parameter_description_length_rejects_an_empty_description() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let row = ParameterRow {
        description: "",
        ..ParameterRow::valid()
    };

    let result = insert_parameter(&mut conn, &draft, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_description_length");
}

#[tokio::test]
async fn parameter_ordinal_range_rejects_an_ordinal_above_100000() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let row = ParameterRow {
        ordinal: 100_001,
        ..ParameterRow::valid()
    };

    let result = insert_parameter(&mut conn, &draft, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_ordinal_range");
}

#[tokio::test]
async fn parameter_s7_address_format_rejects_a_db_number_of_zero() {
    let mut conn = connection().await;
    let draft = draft_on(&mut conn, "S7_1200").await;
    let row = ParameterRow {
        platform: "S7_1200",
        s7_address: Some("DB0.DBD0"),
        ..ParameterRow::valid()
    };

    let result = insert_parameter(&mut conn, &draft, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_s7_address_format");
}

#[tokio::test]
async fn parameter_s7_address_platform_requires_an_address_on_s7_300() {
    let mut conn = connection().await;
    let draft = draft_on(&mut conn, "S7_300").await;
    let row = ParameterRow {
        platform: "S7_300",
        s7_address: None,
        ..ParameterRow::valid()
    };

    let result = insert_parameter(&mut conn, &draft, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_s7_address_platform");
}

#[tokio::test]
async fn parameter_s7_address_platform_accepts_an_optional_address_on_s7_1200_and_s7_1500() {
    let mut conn = connection().await;
    let s7_1200 = draft_on(&mut conn, "S7_1200").await;
    let s7_1500 = draft_on(&mut conn, "S7_1500").await;
    for (draft, platform) in [(s7_1200, "S7_1200"), (s7_1500, "S7_1500")] {
        let without = ParameterRow {
            platform,
            ..ParameterRow::valid()
        };
        let with = ParameterRow {
            platform,
            tag: "P_ADDR",
            s7_address: Some("DB1.DBD0"),
            ..ParameterRow::valid()
        };
        insert_parameter(&mut conn, &draft, &without).await.unwrap();
        insert_parameter(&mut conn, &draft, &with).await.unwrap();
    }
}

#[tokio::test]
async fn parameter_s7_address_platform_rejects_an_address_on_twincat3() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let row = ParameterRow {
        s7_address: Some("DB1.DBD0"),
        ..ParameterRow::valid()
    };

    let result = insert_parameter(&mut conn, &draft, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_s7_address_platform");
}

#[tokio::test]
async fn parameter_value_size_rejects_a_value_above_1_mib() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;

    // The MD5 digests of 200 000 numbers make about 6.4 MB of text. The stored value is larger
    // than 1 MiB, even when LZ4 compresses it.
    let result = sqlx::query(
        "INSERT INTO parameter \
             (recipe_version_id, plc_platform, unit_id, tag, name_en, description_en, \
              plc_type_id, value) \
         VALUES ($1, 'TwinCat3', $2, 'P_BIG', 'Big', 'Big value', $3, \
                 (SELECT jsonb_build_object('text', string_agg(md5(g::text), '')) \
                  FROM generate_series(1, 200000) AS g))",
    )
    .bind(draft.version)
    .bind(draft.unit)
    .bind(draft.dint)
    .execute(&mut conn)
    .await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_value_size");
}

// ---- parameter_limit ----

/// The columns of one `parameter_limit` row. A bound is a decimal text, or NULL.
#[derive(Debug, Clone)]
struct LimitRow {
    leaf_path: &'static str,
    min: Option<&'static str>,
    max: Option<&'static str>,
    max_length: Option<i16>,
}

impl LimitRow {
    /// A numeric limit from 0 to 10 on the parameter itself.
    fn valid() -> Self {
        Self {
            leaf_path: "",
            min: Some("0"),
            max: Some("10"),
            max_length: None,
        }
    }
}

/// Insert a limit of a parameter in a version.
async fn insert_limit(
    conn: &mut PgConnection,
    parameter: Uuid,
    version: Uuid,
    row: &LimitRow,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO parameter_limit \
             (parameter_id, recipe_version_id, leaf_path, min_value, max_value, max_length) \
         VALUES ($1, $2, $3, $4::text::numeric, $5::text::numeric, $6)",
    )
    .bind(parameter)
    .bind(version)
    .bind(row.leaf_path)
    .bind(row.min)
    .bind(row.max)
    .bind(row.max_length)
    .execute(conn)
    .await
    .map(|_| ())
}

#[tokio::test]
async fn parameter_limit_row_with_valid_values_inserts() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_LIMIT").await;

    insert_limit(&mut conn, parameter, draft.version, &LimitRow::valid())
        .await
        .unwrap();
}

#[tokio::test]
async fn parameter_limit_pkey_rejects_a_second_limit_of_one_leaf() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_LIMIT").await;
    insert_limit(&mut conn, parameter, draft.version, &LimitRow::valid())
        .await
        .unwrap();

    let result = insert_limit(&mut conn, parameter, draft.version, &LimitRow::valid()).await;

    assert_constraint(result, UNIQUE_VIOLATION, "parameter_limit_pkey");
}

#[tokio::test]
async fn parameter_limit_parameter_rejects_a_limit_of_a_parameter_of_another_version() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_LIMIT").await;
    let other = draft.other_draft(&mut conn).await;

    let result = insert_limit(&mut conn, parameter, other, &LimitRow::valid()).await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "parameter_limit_parameter");
}

#[tokio::test]
async fn parameter_limit_path_format_rejects_a_leaf_path_without_a_dot_or_bracket() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_LIMIT").await;
    let row = LimitRow {
        leaf_path: "bad",
        ..LimitRow::valid()
    };

    let result = insert_limit(&mut conn, parameter, draft.version, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_limit_path_format");
}

#[tokio::test]
async fn parameter_limit_min_le_max_rejects_a_minimum_above_the_maximum() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_LIMIT").await;
    let row = LimitRow {
        min: Some("5"),
        max: Some("1"),
        ..LimitRow::valid()
    };

    let result = insert_limit(&mut conn, parameter, draft.version, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_limit_min_le_max");
}

#[tokio::test]
async fn parameter_limit_not_empty_rejects_a_limit_without_any_bound() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_LIMIT").await;
    let row = LimitRow {
        min: None,
        max: None,
        max_length: None,
        ..LimitRow::valid()
    };

    let result = insert_limit(&mut conn, parameter, draft.version, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_limit_not_empty");
}

#[tokio::test]
async fn parameter_limit_one_kind_rejects_numeric_bounds_with_a_max_length() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_LIMIT").await;
    let row = LimitRow {
        max_length: Some(5),
        ..LimitRow::valid()
    };

    let result = insert_limit(&mut conn, parameter, draft.version, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_limit_one_kind");
}

#[tokio::test]
async fn parameter_limit_max_length_range_rejects_a_max_length_of_zero() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_LIMIT").await;
    let row = LimitRow {
        min: None,
        max: None,
        max_length: Some(0),
        ..LimitRow::valid()
    };

    let result = insert_limit(&mut conn, parameter, draft.version, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_limit_max_length_range");
}

// ---- parameter_translation ----

/// Insert a translation of a parameter in a version.
async fn insert_translation(
    conn: &mut PgConnection,
    parameter: Uuid,
    version: Uuid,
    lang: &str,
    name: &str,
    description: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO parameter_translation \
             (parameter_id, recipe_version_id, lang, name, description) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(parameter)
    .bind(version)
    .bind(lang)
    .bind(name)
    .bind(description)
    .execute(conn)
    .await
    .map(|_| ())
}

#[tokio::test]
async fn parameter_translation_row_with_valid_values_inserts() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_TR").await;

    insert_translation(&mut conn, parameter, draft.version, "pl", "nazwa", "opis")
        .await
        .unwrap();
}

#[tokio::test]
async fn parameter_translation_pkey_rejects_a_second_translation_in_one_language() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_TR").await;
    insert_translation(&mut conn, parameter, draft.version, "pl", "nazwa", "")
        .await
        .unwrap();

    let result = insert_translation(&mut conn, parameter, draft.version, "pl", "druga", "").await;

    assert_constraint(result, UNIQUE_VIOLATION, "parameter_translation_pkey");
}

#[tokio::test]
async fn parameter_translation_parameter_rejects_a_translation_of_another_version() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_TR").await;
    let other = draft.other_draft(&mut conn).await;

    let result = insert_translation(&mut conn, parameter, other, "pl", "nazwa", "").await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "parameter_translation_parameter",
    );
}

#[tokio::test]
async fn parameter_translation_lang_format_rejects_english_as_a_translation() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_TR").await;

    let result = insert_translation(&mut conn, parameter, draft.version, "en", "name", "").await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_translation_lang_format");
}

#[tokio::test]
async fn parameter_translation_lang_format_rejects_an_uppercase_language() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_TR").await;

    let result = insert_translation(&mut conn, parameter, draft.version, "PL", "nazwa", "").await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_translation_lang_format");
}

#[tokio::test]
async fn parameter_translation_name_length_rejects_an_empty_name() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_TR").await;

    let result = insert_translation(&mut conn, parameter, draft.version, "pl", "", "").await;

    assert_constraint(result, CHECK_VIOLATION, "parameter_translation_name_length");
}

#[tokio::test]
async fn parameter_translation_description_length_rejects_a_description_above_4000_characters() {
    let mut conn = connection().await;
    let draft = Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_TR").await;
    let description = "x".repeat(4001);

    let result = insert_translation(
        &mut conn,
        parameter,
        draft.version,
        "pl",
        "nazwa",
        &description,
    )
    .await;

    assert_constraint(
        result,
        CHECK_VIOLATION,
        "parameter_translation_description_length",
    );
}
