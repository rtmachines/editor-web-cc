//! Tests of `recipe`, `recipe_version`, and `draft_editor` (`data-model.md` sections 3.9 to 3.11,
//! task 7.4).
//!
//! The tests run as `recipe_migrator`, the owner of the tables. A test that needs a second version
//! of one recipe first discards the live draft, because `recipe_version_one_live_draft` allows one
//! live draft per recipe.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{
    CHECK_VIOLATION, FOREIGN_KEY_VIOLATION, Role, TestDatabase, UNIQUE_VIOLATION, assert_constraint,
};

/// The constraints and unique indexes that the tests of this module violate.
pub const COVERED: &[&str] = &[
    "recipe_pkey",
    "recipe_created_by_fkey",
    "recipe_version_pkey",
    "recipe_version_created_by_fkey",
    "recipe_version_approved_by_fkey",
    "recipe_machine_platform",
    "recipe_reference_format",
    "recipe_reference_unique",
    "recipe_name_length",
    "recipe_version_recipe_platform",
    "recipe_version_number_positive",
    "recipe_version_number_unique",
    "recipe_version_based_on_same_recipe",
    "recipe_version_status_value",
    "recipe_version_change_note_length",
    "recipe_version_row_version_positive",
    "recipe_version_hash_format",
    "recipe_version_status_fields",
    "recipe_version_deleted_only_draft",
    "recipe_version_one_approved",
    "recipe_version_one_live_draft",
    "draft_editor_pkey",
    "draft_editor_recipe_version_id_fkey",
    "draft_editor_user_id_fkey",
];

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    let database = TestDatabase::create().await;
    database.connect(Role::Migrator).await
}

/// The rows that most tests of this module start from.
struct Setup {
    user: Uuid,
    machine: Uuid,
    recipe: Uuid,
}

/// Insert a user, a `TwinCat3` machine, and a recipe of the machine.
async fn setup(conn: &mut PgConnection) -> Setup {
    let user = fixtures::user(conn).await;
    let machine = fixtures::machine(conn, TWIN_CAT).await;
    let recipe = fixtures::recipe(conn, machine, TWIN_CAT, user).await;
    Setup {
        user,
        machine,
        recipe,
    }
}

/// A hash of 64 lowercase hexadecimal characters, which satisfies `recipe_version_hash_format`.
fn valid_hash() -> String {
    "b".repeat(64)
}

/// Insert a recipe with a reference number and a name. Return its id.
async fn insert_recipe(
    conn: &mut PgConnection,
    machine: Uuid,
    platform: &str,
    reference: &str,
    name: &str,
    user: Uuid,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO recipe (machine_id, plc_platform, reference_number, name, created_by) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(reference)
    .bind(name)
    .bind(user)
    .fetch_one(conn)
    .await
}

/// The columns of one `recipe_version` row. A test changes one field of a valid row.
#[derive(Debug, Clone)]
struct VersionRow {
    recipe: Uuid,
    platform: &'static str,
    number: i32,
    status: &'static str,
    row_version: i64,
    based_on: Option<Uuid>,
    change_note: &'static str,
    content_sha256: Option<String>,
    created_by: Uuid,
    approved_by: Option<Uuid>,
    approved_at: bool,
    retired_at: bool,
    deleted_at: bool,
}

impl VersionRow {
    /// A live draft of the recipe of the setup with a valid change note.
    fn draft(setup: &Setup, number: i32) -> Self {
        Self {
            recipe: setup.recipe,
            platform: TWIN_CAT,
            number,
            status: "draft",
            row_version: 1,
            based_on: None,
            change_note: "note",
            content_sha256: None,
            created_by: setup.user,
            approved_by: None,
            approved_at: false,
            retired_at: false,
            deleted_at: false,
        }
    }

    /// An approved version with the approval fields and a valid hash.
    fn approved(setup: &Setup, number: i32) -> Self {
        Self {
            status: "approved",
            approved_by: Some(setup.user),
            approved_at: true,
            content_sha256: Some(valid_hash()),
            ..Self::draft(setup, number)
        }
    }
}

/// Insert a `recipe_version` row from the columns of a row. Return its id.
async fn insert_version(conn: &mut PgConnection, row: &VersionRow) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO recipe_version \
             (recipe_id, plc_platform, number, status, row_version, based_on_version_id, \
              change_note, content_sha256, created_by, approved_by, approved_at, retired_at, \
              deleted_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, \
                 CASE WHEN $11 THEN now() END, CASE WHEN $12 THEN now() END, \
                 CASE WHEN $13 THEN now() END) \
         RETURNING id",
    )
    .bind(row.recipe)
    .bind(row.platform)
    .bind(row.number)
    .bind(row.status)
    .bind(row.row_version)
    .bind(row.based_on)
    .bind(row.change_note)
    .bind(row.content_sha256.clone())
    .bind(row.created_by)
    .bind(row.approved_by)
    .bind(row.approved_at)
    .bind(row.retired_at)
    .bind(row.deleted_at)
    .fetch_one(conn)
    .await
}

/// Discard a live draft. The partial index `recipe_version_one_live_draft` then ignores it.
pub async fn discard(conn: &mut PgConnection, version: Uuid) {
    sqlx::query("UPDATE recipe_version SET deleted_at = now() WHERE id = $1")
        .bind(version)
        .execute(conn)
        .await
        .unwrap();
}

// ---- recipe ----

#[tokio::test]
async fn recipe_row_with_valid_values_inserts() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;

    insert_recipe(&mut conn, s.machine, TWIN_CAT, "REF-1", "Recipe", s.user)
        .await
        .unwrap();
}

#[tokio::test]
async fn recipe_pkey_rejects_a_second_row_with_the_same_id() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;

    let result = sqlx::query(
        "INSERT INTO recipe (id, machine_id, plc_platform, reference_number, name, created_by) \
         VALUES ($1, $2, 'TwinCat3', 'REF-PK', 'Copy', $3)",
    )
    .bind(s.recipe)
    .bind(s.machine)
    .bind(s.user)
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "recipe_pkey");
}

#[tokio::test]
async fn recipe_created_by_fkey_rejects_an_unknown_user() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;

    let result = insert_recipe(
        &mut conn,
        s.machine,
        TWIN_CAT,
        "REF-FK",
        "Recipe",
        Uuid::nil(),
    )
    .await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "recipe_created_by_fkey");
}

#[tokio::test]
async fn recipe_machine_platform_rejects_a_platform_that_differs_from_the_machine() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;

    let result = insert_recipe(&mut conn, s.machine, "S7_1200", "REF-2", "Recipe", s.user).await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "recipe_machine_platform");
}

#[tokio::test]
async fn recipe_reference_format_rejects_a_lowercase_reference() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;

    let result = insert_recipe(&mut conn, s.machine, TWIN_CAT, "ref-3", "Recipe", s.user).await;

    assert_constraint(result, CHECK_VIOLATION, "recipe_reference_format");
}

#[tokio::test]
async fn recipe_reference_unique_rejects_a_second_recipe_with_the_same_reference() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    insert_recipe(&mut conn, s.machine, TWIN_CAT, "REF-4", "Recipe", s.user)
        .await
        .unwrap();

    let result = insert_recipe(&mut conn, s.machine, TWIN_CAT, "REF-4", "Other", s.user).await;

    assert_constraint(result, UNIQUE_VIOLATION, "recipe_reference_unique");
}

#[tokio::test]
async fn recipe_name_length_rejects_an_empty_name() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;

    let result = insert_recipe(&mut conn, s.machine, TWIN_CAT, "REF-5", "", s.user).await;

    assert_constraint(result, CHECK_VIOLATION, "recipe_name_length");
}

// ---- recipe_version ----

#[tokio::test]
async fn recipe_version_row_with_valid_values_inserts() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;

    insert_version(&mut conn, &VersionRow::draft(&s, 1))
        .await
        .unwrap();
}

#[tokio::test]
async fn discarded_draft_frees_the_live_draft_slot() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let first = insert_version(&mut conn, &VersionRow::draft(&s, 1))
        .await
        .unwrap();
    discard(&mut conn, first).await;

    insert_version(&mut conn, &VersionRow::draft(&s, 2))
        .await
        .unwrap();
}

#[tokio::test]
async fn retired_version_does_not_block_a_new_approved_version() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let first = insert_version(&mut conn, &VersionRow::draft(&s, 1))
        .await
        .unwrap();
    fixtures::approve(&mut conn, first, s.user).await;
    fixtures::retire(&mut conn, first).await;
    let second = insert_version(&mut conn, &VersionRow::draft(&s, 2))
        .await
        .unwrap();

    fixtures::approve(&mut conn, second, s.user).await;
}

#[tokio::test]
async fn recipe_version_pkey_rejects_a_second_row_with_the_same_id() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let first = insert_version(&mut conn, &VersionRow::draft(&s, 1))
        .await
        .unwrap();
    discard(&mut conn, first).await;

    let result = sqlx::query(
        "INSERT INTO recipe_version (id, recipe_id, plc_platform, number, change_note, created_by) \
         VALUES ($1, $2, 'TwinCat3', 2, 'note', $3)",
    )
    .bind(first)
    .bind(s.recipe)
    .bind(s.user)
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "recipe_version_pkey");
}

#[tokio::test]
async fn recipe_version_created_by_fkey_rejects_an_unknown_user() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        created_by: Uuid::nil(),
        ..VersionRow::draft(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "recipe_version_created_by_fkey",
    );
}

#[tokio::test]
async fn recipe_version_approved_by_fkey_rejects_an_unknown_approver() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        approved_by: Some(Uuid::nil()),
        ..VersionRow::approved(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "recipe_version_approved_by_fkey",
    );
}

#[tokio::test]
async fn recipe_version_recipe_platform_rejects_a_platform_that_differs_from_the_recipe() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        platform: "S7_1200",
        ..VersionRow::draft(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "recipe_version_recipe_platform",
    );
}

#[tokio::test]
async fn recipe_version_number_positive_rejects_number_zero() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        number: 0,
        ..VersionRow::draft(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "recipe_version_number_positive");
}

#[tokio::test]
async fn recipe_version_number_unique_rejects_a_second_version_with_the_same_number() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let first = insert_version(&mut conn, &VersionRow::draft(&s, 1))
        .await
        .unwrap();
    discard(&mut conn, first).await;

    let result = insert_version(&mut conn, &VersionRow::draft(&s, 1)).await;

    assert_constraint(result, UNIQUE_VIOLATION, "recipe_version_number_unique");
}

#[tokio::test]
async fn recipe_version_based_on_same_recipe_rejects_a_base_version_of_another_recipe() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let other = fixtures::recipe(&mut conn, s.machine, TWIN_CAT, s.user).await;
    let base = fixtures::draft_version(&mut conn, other, TWIN_CAT, 1, s.user).await;
    let first = insert_version(&mut conn, &VersionRow::draft(&s, 1))
        .await
        .unwrap();
    discard(&mut conn, first).await;
    let row = VersionRow {
        based_on: Some(base),
        ..VersionRow::draft(&s, 2)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "recipe_version_based_on_same_recipe",
    );
}

/// `recipe_version_status_value` and `recipe_version_status_fields` both reject an unknown status.
/// PostgreSQL checks the CHECK constraints in name order, and `recipe_version_status_fields` sorts
/// first. This test therefore drops `recipe_version_status_fields` in the disposable database
/// before it inserts the row. The status value CHECK is then the only one that can fail.
#[tokio::test]
async fn recipe_version_status_value_rejects_an_unknown_status() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    // PostgreSQL checks CHECK constraints in name order, and `status_fields` also rejects an
    // unknown status. The owner drops it, so the test database names `status_value` alone.
    sqlx::query("ALTER TABLE recipe_version DROP CONSTRAINT recipe_version_status_fields")
        .execute(&mut conn)
        .await
        .unwrap();
    let row = VersionRow {
        status: "pending",
        ..VersionRow::draft(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "recipe_version_status_value");
}

/// Without the drop of the other CHECK, `recipe_version_status_fields` reports first. The test
/// documents the order of the checks.
#[tokio::test]
async fn an_unknown_status_is_reported_by_status_fields_when_both_checks_exist() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        status: "pending",
        ..VersionRow::draft(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "recipe_version_status_fields");
}

#[tokio::test]
async fn recipe_version_change_note_length_rejects_an_empty_change_note() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        change_note: "",
        ..VersionRow::draft(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "recipe_version_change_note_length");
}

#[tokio::test]
async fn recipe_version_row_version_positive_rejects_a_row_version_of_zero() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        row_version: 0,
        ..VersionRow::draft(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(
        result,
        CHECK_VIOLATION,
        "recipe_version_row_version_positive",
    );
}

#[tokio::test]
async fn recipe_version_hash_format_rejects_a_hash_that_is_not_lowercase_hex() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        content_sha256: Some("Z".repeat(64)),
        ..VersionRow::approved(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "recipe_version_hash_format");
}

#[tokio::test]
async fn recipe_version_status_fields_rejects_a_draft_with_an_approval_time() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        approved_by: Some(s.user),
        approved_at: true,
        ..VersionRow::draft(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "recipe_version_status_fields");
}

#[tokio::test]
async fn recipe_version_status_fields_rejects_an_approved_version_without_a_hash() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        content_sha256: None,
        ..VersionRow::approved(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "recipe_version_status_fields");
}

#[tokio::test]
async fn recipe_version_status_fields_rejects_a_retired_version_without_a_retirement_time() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        status: "retired",
        retired_at: false,
        ..VersionRow::approved(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "recipe_version_status_fields");
}

#[tokio::test]
async fn recipe_version_deleted_only_draft_rejects_a_discarded_approved_version() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let row = VersionRow {
        deleted_at: true,
        ..VersionRow::approved(&s, 1)
    };

    let result = insert_version(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "recipe_version_deleted_only_draft");
}

#[tokio::test]
async fn recipe_version_one_approved_rejects_a_second_approved_version() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let first = insert_version(&mut conn, &VersionRow::draft(&s, 1))
        .await
        .unwrap();
    fixtures::approve(&mut conn, first, s.user).await;
    let second = insert_version(&mut conn, &VersionRow::draft(&s, 2))
        .await
        .unwrap();

    let result = sqlx::query(
        "UPDATE recipe_version SET status = 'approved', approved_by = $2, approved_at = now(), \
             content_sha256 = repeat('c', 64) WHERE id = $1",
    )
    .bind(second)
    .bind(s.user)
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "recipe_version_one_approved");
}

#[tokio::test]
async fn recipe_version_one_live_draft_rejects_a_second_live_draft() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    insert_version(&mut conn, &VersionRow::draft(&s, 1))
        .await
        .unwrap();

    let result = insert_version(&mut conn, &VersionRow::draft(&s, 2)).await;

    assert_constraint(result, UNIQUE_VIOLATION, "recipe_version_one_live_draft");
}

// ---- draft_editor ----

#[tokio::test]
async fn draft_editor_row_with_valid_values_inserts() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let version = insert_version(&mut conn, &VersionRow::draft(&s, 1))
        .await
        .unwrap();

    sqlx::query("INSERT INTO draft_editor (recipe_version_id, user_id) VALUES ($1, $2)")
        .bind(version)
        .bind(s.user)
        .execute(&mut conn)
        .await
        .unwrap();
}

#[tokio::test]
async fn draft_editor_pkey_rejects_a_second_row_of_one_user_and_version() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let version = insert_version(&mut conn, &VersionRow::draft(&s, 1))
        .await
        .unwrap();
    sqlx::query("INSERT INTO draft_editor (recipe_version_id, user_id) VALUES ($1, $2)")
        .bind(version)
        .bind(s.user)
        .execute(&mut conn)
        .await
        .unwrap();

    let result =
        sqlx::query("INSERT INTO draft_editor (recipe_version_id, user_id) VALUES ($1, $2)")
            .bind(version)
            .bind(s.user)
            .execute(&mut conn)
            .await;

    assert_constraint(result, UNIQUE_VIOLATION, "draft_editor_pkey");
}

#[tokio::test]
async fn draft_editor_recipe_version_id_fkey_rejects_an_unknown_version() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;

    let result =
        sqlx::query("INSERT INTO draft_editor (recipe_version_id, user_id) VALUES ($1, $2)")
            .bind(Uuid::nil())
            .bind(s.user)
            .execute(&mut conn)
            .await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "draft_editor_recipe_version_id_fkey",
    );
}

#[tokio::test]
async fn draft_editor_user_id_fkey_rejects_an_unknown_user() {
    let mut conn = connection().await;
    let s = setup(&mut conn).await;
    let version = insert_version(&mut conn, &VersionRow::draft(&s, 1))
        .await
        .unwrap();

    let result =
        sqlx::query("INSERT INTO draft_editor (recipe_version_id, user_id) VALUES ($1, $2)")
            .bind(version)
            .bind(Uuid::nil())
            .execute(&mut conn)
            .await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "draft_editor_user_id_fkey");
}
