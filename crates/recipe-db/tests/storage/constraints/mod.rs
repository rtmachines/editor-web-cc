//! Tests of the constraints and unique indexes of `data-model.md` sections 3.1 to 3.17.
//!
//! Each module exports `COVERED`, the names of the constraints and indexes that its tests violate.
//! The meta-test of this module compares all lists with the named constraints and the unique
//! indexes of section 3. The test also derives the PostgreSQL default names of inline clauses:
//! `<table>_pkey` and `<table>_<column>_fkey`. A name that is in neither a `COVERED` list nor
//! `EXEMPT` fails the test.

mod audit;
mod default_names;
mod identity;
mod parameter;
mod plant;
mod recipe;
mod types;

use sqlx::PgConnection;

use self::default_names::default_constraint_names;
use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{
    FOREIGN_KEY_VIOLATION, Role, TestDatabase, assert_constraint, created_names, document_sql,
};

/// The sections of `data-model.md` that define tables, constraints, and indexes.
const SECTION_3: &[&str] = &[
    "3.0", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7", "3.8", "3.9", "3.10", "3.11", "3.12",
    "3.13", "3.14", "3.15", "3.16", "3.17",
];

/// Composite unique constraints that no test violates on their own.
///
/// Each one is either the target of a foreign key, or it is a unique constraint whose duplicate
/// always violates the primary key `id` first. The tests at the end of this module show that
/// each foreign key target rejects a wrong pair.
const EXEMPT: &[&str] = &[
    // The primary key on `machine.id` rejects a duplicate id first. A foreign key of
    // `recipe_machine_platform` targets this constraint.
    "machine_id_platform_unique",
    // The primary key on `unit.id` rejects a duplicate id first. No foreign key of section 3
    // targets this constraint, because trigger 4.3 does the scope check. No test covers it.
    "unit_machine_id_unique",
    // The primary key on `recipe.id` rejects a duplicate id first. A foreign key of
    // `recipe_version_recipe_platform` targets this constraint.
    "recipe_id_platform_unique",
    // The primary key on `recipe_version.id` rejects a duplicate id first. A foreign key of
    // `recipe_version_based_on_same_recipe` targets this constraint.
    "recipe_version_recipe_id_id_unique",
    // The primary key on `recipe_version.id` rejects a duplicate id first. A foreign key of
    // `parameter_version_platform` targets this constraint.
    "recipe_version_id_platform_unique",
    // The primary key on `parameter.id` rejects a duplicate id first. Foreign keys of
    // `parameter_limit_parameter` and `parameter_translation_parameter` target this constraint.
    "parameter_id_version_unique",
];

/// Return a migrated database connection as the owner of the tables.
async fn migrator() -> PgConnection {
    let database = TestDatabase::create().await;
    database.connect(Role::Migrator).await
}

/// Return true when the catalog holds a constraint or an index of this name.
async fn exists(conn: &mut PgConnection, name: &str) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = $1) \
            OR EXISTS (SELECT 1 FROM pg_class WHERE relname = $1 AND relkind = 'i')",
    )
    .bind(name)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// The names of the unique indexes that normalized SQL creates. A plain index is skipped.
fn unique_index_names(sql: &str) -> Vec<String> {
    sql.split(';')
        .filter(|statement| statement.contains("CREATE UNIQUE INDEX"))
        .flat_map(|statement| created_names(statement).indexes)
        .collect()
}

#[tokio::test]
async fn covered_names_exist_in_the_catalog() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let names: Vec<&str> = [
        plant::COVERED,
        types::COVERED,
        identity::COVERED,
        recipe::COVERED,
        parameter::COVERED,
        audit::COVERED,
    ]
    .concat();

    for name in names {
        assert!(exists(&mut conn, name).await, "missing: {name}");
    }
}

#[test]
fn every_named_constraint_and_unique_index_of_section_3_is_covered_or_exempt() {
    let sql = document_sql(SECTION_3, &[]);
    let named = created_names(&sql);
    let unique = unique_index_names(&sql);
    assert!(
        !named.constraints.is_empty(),
        "section 3 names no constraint"
    );
    assert!(!unique.is_empty(), "section 3 names no unique index");

    let derived = default_constraint_names(&sql);
    assert!(
        derived.iter().any(|name| name.ends_with("_pkey"))
            && derived.iter().any(|name| name.ends_with("_fkey")),
        "section 3 yields no default key name"
    );

    let covered: Vec<&str> = [
        plant::COVERED,
        types::COVERED,
        identity::COVERED,
        recipe::COVERED,
        parameter::COVERED,
        audit::COVERED,
    ]
    .concat();
    let missing: Vec<&str> = named
        .constraints
        .iter()
        .map(String::as_str)
        .chain(unique.iter().map(String::as_str))
        .chain(derived.iter().map(String::as_str))
        .filter(|name| !covered.contains(name) && !EXEMPT.contains(name))
        .collect();

    assert!(
        missing.is_empty(),
        "names of section 3 without a test: {missing:?}"
    );
}

// ---- exempt names: a foreign key that targets the constraint rejects a wrong pair ----

#[tokio::test]
async fn machine_id_platform_target_rejects_a_recipe_with_another_platform() {
    let mut conn = migrator().await;
    let user = fixtures::user(&mut conn).await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = sqlx::query(
        "INSERT INTO recipe (machine_id, plc_platform, reference_number, name, created_by) \
         VALUES ($1, 'S7_1200', 'REF-EXEMPT-1', 'recipe', $2)",
    )
    .bind(machine)
    .bind(user)
    .execute(&mut conn)
    .await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "recipe_machine_platform");
}

#[tokio::test]
async fn recipe_id_platform_target_rejects_a_version_with_another_platform() {
    let mut conn = migrator().await;
    let user = fixtures::user(&mut conn).await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    let recipe = fixtures::recipe(&mut conn, machine, TWIN_CAT, user).await;

    let result = sqlx::query(
        "INSERT INTO recipe_version (recipe_id, plc_platform, number, change_note, created_by) \
         VALUES ($1, 'S7_1200', 1, 'note', $2)",
    )
    .bind(recipe)
    .bind(user)
    .execute(&mut conn)
    .await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "recipe_version_recipe_platform",
    );
}

#[tokio::test]
async fn recipe_version_recipe_id_id_target_rejects_a_base_version_of_another_recipe() {
    let mut conn = migrator().await;
    let user = fixtures::user(&mut conn).await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    let first = fixtures::recipe(&mut conn, machine, TWIN_CAT, user).await;
    let second = fixtures::recipe(&mut conn, machine, TWIN_CAT, user).await;
    let base = fixtures::draft_version(&mut conn, second, TWIN_CAT, 1, user).await;
    let draft = fixtures::draft_version(&mut conn, first, TWIN_CAT, 1, user).await;
    recipe::discard(&mut conn, draft).await;

    let result = sqlx::query(
        "INSERT INTO recipe_version \
             (recipe_id, plc_platform, number, based_on_version_id, change_note, created_by) \
         VALUES ($1, 'TwinCat3', 2, $2, 'note', $3)",
    )
    .bind(first)
    .bind(base)
    .bind(user)
    .execute(&mut conn)
    .await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "recipe_version_based_on_same_recipe",
    );
}

#[tokio::test]
async fn recipe_version_id_platform_target_rejects_a_parameter_with_another_platform() {
    let mut conn = migrator().await;
    let draft = fixtures::Draft::create(&mut conn).await;

    let result = sqlx::query(
        "INSERT INTO parameter \
             (recipe_version_id, plc_platform, unit_id, tag, name_en, description_en, \
              plc_type_id, value) \
         VALUES ($1, 'S7_1200', $2, 'P_EXEMPT', 'name', 'description', $3, '0'::jsonb)",
    )
    .bind(draft.version)
    .bind(draft.unit)
    .bind(draft.dint)
    .execute(&mut conn)
    .await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "parameter_version_platform");
}

#[tokio::test]
async fn parameter_id_version_target_rejects_a_limit_of_another_version() {
    let mut conn = migrator().await;
    let draft = fixtures::Draft::create(&mut conn).await;
    let parameter = draft.parameter(&mut conn, "P_EXEMPT").await;
    let other = draft.other_draft(&mut conn).await;

    let result = sqlx::query(
        "INSERT INTO parameter_limit (parameter_id, recipe_version_id, leaf_path, min_value, \
             max_value) VALUES ($1, $2, '', 0, 10)",
    )
    .bind(parameter)
    .bind(other)
    .execute(&mut conn)
    .await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "parameter_limit_parameter");
}
