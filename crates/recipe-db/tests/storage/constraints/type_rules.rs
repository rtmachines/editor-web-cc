//! Tests of the identifier rules in `plc_type` and `plc_type_member` (ADR-0143, ADR-0145).
//!
//! Migration `0011` replaces `dims_valid` and builds the two case-insensitive unique indexes.
//! `types.rs` has the tests of the older constraints. This module holds the tests of `0011`.

use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;

use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{CHECK_VIOLATION, Role, TestDatabase, UNIQUE_VIOLATION, assert_constraint};

/// The constraints and unique indexes that the tests of this module violate.
pub const COVERED: &[&str] = &[
    "plc_type_struct_name_ci_unique",
    "plc_type_member_name_ci_unique",
];

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    TestDatabase::create()
        .await
        .into_session(Role::Migrator)
        .await
}

/// Return the result of `dims_valid` for two array literals of SQL.
async fn dims_valid(conn: &mut PgConnection, lower: &str, upper: &str) -> bool {
    sqlx::query_scalar(&format!(
        "SELECT dims_valid('{lower}'::integer[], '{upper}'::integer[])"
    ))
    .fetch_one(conn)
    .await
    .unwrap()
}

#[tokio::test]
async fn dims_valid_rejects_a_bound_array_that_starts_at_another_subscript_than_1() {
    let mut conn = connection().await;
    let cases = [
        // The task cases of ADR-0145 (a).
        ("[0:0]={5}", "{0}", false),
        ("[0:0]={0}", "{65536}", false),
        ("{1}", "[0:0]={8}", false),
        // Bounds that are valid in every other respect. Only the start subscript differs.
        ("[0:0]={1}", "{5}", false),
        ("[2:2]={1}", "{5}", false),
        ("[0:1]={1,1}", "{5,5}", false),
        ("{1,1}", "[0:1]={5,5}", false),
        ("{1}", "{8}", true),
        ("{1,1,1}", "{4,5,6}", true),
    ];

    for (lower, upper, expected) in cases {
        assert_eq!(
            dims_valid(&mut conn, lower, upper).await,
            expected,
            "lower {lower}, upper {upper}"
        );
    }
}

#[tokio::test]
async fn plc_type_array_shape_rejects_bounds_that_start_at_the_subscript_0() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    let int = fixtures::primitive(&mut conn, "INT").await;

    // The first pair is the pair of ADR-0145 (a). The other pairs are valid but for the subscript.
    for (lower, upper) in [
        ("[0:0]={5}", "{0}"),
        ("[0:0]={1}", "{5}"),
        ("{1}", "[0:0]={8}"),
    ] {
        let result = sqlx::query(&format!(
            "INSERT INTO plc_type (kind, platforms, element_type_id, dim_lower, dim_upper, \
                 machine_id) \
             VALUES ('array', ARRAY['TwinCat3'], $1, '{lower}'::integer[], \
                 '{upper}'::integer[], $2)"
        ))
        .bind(int)
        .bind(machine)
        .execute(&mut conn)
        .await;

        assert_constraint(result, CHECK_VIOLATION, "plc_type_array_shape");
    }
}

/// Insert a struct revision with its members in one transaction, as the application does.
async fn insert_struct_with_members(
    app: &mut PgConnection,
    machine: Uuid,
    name: &str,
    revision: i32,
    members: &[(&str, Uuid)],
) -> Result<(), sqlx::Error> {
    let mut tx = app.begin().await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO plc_type (kind, name, platforms, machine_id, revision) \
         VALUES ('struct', $1, ARRAY['TwinCat3'], $2, $3) RETURNING id",
    )
    .bind(name)
    .bind(machine)
    .bind(revision)
    .fetch_one(&mut *tx)
    .await?;
    for (ordinal, (member_name, member_type)) in (1_i16..).zip(members) {
        sqlx::query(
            "INSERT INTO plc_type_member (struct_type_id, ordinal, member_name, member_type_id) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(ordinal)
        .bind(member_name)
        .bind(member_type)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

/// Insert a struct revision with the one member `speed`.
async fn insert_struct(
    app: &mut PgConnection,
    machine: Uuid,
    name: &str,
    revision: i32,
    member_type: Uuid,
) -> Result<(), sqlx::Error> {
    insert_struct_with_members(app, machine, name, revision, &[("speed", member_type)]).await
}

#[tokio::test]
async fn plc_type_struct_name_ci_unique_rejects_a_struct_name_that_differs_only_in_case() {
    let database = TestDatabase::create().await;
    let mut owner = database.connect(Role::Migrator).await;
    let machine = fixtures::machine(&mut owner, TWIN_CAT).await;
    let other_machine = fixtures::machine(&mut owner, TWIN_CAT).await;
    let int = fixtures::primitive(&mut owner, "INT").await;
    let mut app = database.connect(Role::App).await;
    insert_struct(&mut app, machine, "ST_Zone", 1, int)
        .await
        .unwrap();

    let result = insert_struct(&mut app, machine, "st_zone", 1, int).await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_type_struct_name_ci_unique");
    // Another machine and another revision are other scopes of the same name.
    insert_struct(&mut app, other_machine, "st_zone", 1, int)
        .await
        .unwrap();
    insert_struct(&mut app, machine, "st_zone", 2, int)
        .await
        .unwrap();
}

#[tokio::test]
async fn plc_type_member_name_ci_unique_rejects_a_member_name_that_differs_only_in_case() {
    let database = TestDatabase::create().await;
    let mut owner = database.connect(Role::Migrator).await;
    let machine = fixtures::machine(&mut owner, TWIN_CAT).await;
    let int = fixtures::primitive(&mut owner, "INT").await;
    let mut app = database.connect(Role::App).await;

    let result = insert_struct_with_members(
        &mut app,
        machine,
        "ST_Pair",
        1,
        &[("speed", int), ("Speed", int)],
    )
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_type_member_name_ci_unique");
    // The same two names are valid in two structs.
    insert_struct_with_members(&mut app, machine, "ST_One", 1, &[("speed", int)])
        .await
        .unwrap();
    insert_struct_with_members(&mut app, machine, "ST_Two", 1, &[("Speed", int)])
        .await
        .unwrap();
}
