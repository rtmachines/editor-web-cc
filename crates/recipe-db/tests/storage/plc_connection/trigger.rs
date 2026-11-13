//! Trigger tests of the table `plc_connection`: `row_version` and `updated_at` (design D1, task 4.4).

use super::*;

// ---------------------------------------------------------------------------------------------
// Task 4.4: the trigger `plc_connection_a_bump` sets `row_version` and `updated_at`. The role
// `recipe_app` has no `UPDATE` grant on `row_version`.
// ---------------------------------------------------------------------------------------------

/// Create a database with one S7 record. Return the handle, the machine, and a session of the
/// owner of the tables.
async fn database_with_record() -> (TestDatabase, Uuid, PgConnection) {
    let database = TestDatabase::create().await;
    let mut owner = database.connect(Role::Migrator).await;
    let machine = fixtures::machine(&mut owner, S7_1500).await;
    insert_s7(&mut owner, machine, S7_1500, None).await.unwrap();
    (database, machine, owner)
}

/// Return `row_version` of the record of a machine.
async fn row_version_of(conn: &mut PgConnection, machine: Uuid) -> i64 {
    sqlx::query_scalar("SELECT row_version FROM plc_connection WHERE machine_id = $1")
        .bind(machine)
        .fetch_one(conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn plc_connection_trigger_plc_connection_a_bump_is_a_before_update_row_trigger() {
    let (_database, _machine, mut owner) = database_with_record().await;

    let (function, is_row, is_before, is_update): (String, bool, bool, bool) = sqlx::query_as(
        "SELECT p.proname::text, \
                (t.tgtype & 1) = 1, (t.tgtype & 2) = 2, (t.tgtype & 16) = 16 \
         FROM pg_trigger t JOIN pg_proc p ON p.oid = t.tgfoid \
         WHERE t.tgrelid = 'plc_connection'::regclass AND t.tgname = 'plc_connection_a_bump'",
    )
    .fetch_one(&mut owner)
    .await
    .expect("the trigger plc_connection_a_bump exists");

    assert_eq!(function, "bump_row_version");
    assert!(is_row && is_before && is_update);
}

#[tokio::test]
async fn plc_connection_update_as_recipe_app_adds_one_to_row_version_and_sets_updated_at() {
    let (database, machine, mut owner) = database_with_record().await;
    let mut app = database.connect(Role::App).await;

    let changed =
        sqlx::query("UPDATE plc_connection SET host = '192.168.1.21' WHERE machine_id = $1")
            .bind(machine)
            .execute(&mut app)
            .await
            .unwrap();

    assert_eq!(changed.rows_affected(), 1);
    assert_eq!(row_version_of(&mut owner, machine).await, 2);
    let is_touched: bool = sqlx::query_scalar(
        "SELECT updated_at > created_at FROM plc_connection WHERE machine_id = $1",
    )
    .bind(machine)
    .fetch_one(&mut owner)
    .await
    .unwrap();
    assert!(is_touched, "the trigger sets updated_at to now()");
}

#[tokio::test]
async fn plc_connection_update_that_sets_row_version_as_recipe_app_gives_42501() {
    let (database, machine, mut owner) = database_with_record().await;
    let mut app = database.connect(Role::App).await;

    let result = sqlx::query("UPDATE plc_connection SET row_version = 99 WHERE machine_id = $1")
        .bind(machine)
        .execute(&mut app)
        .await;

    assert_privilege_denied(result);
    assert_eq!(row_version_of(&mut owner, machine).await, 1);
}

#[tokio::test]
async fn plc_connection_update_that_sets_row_version_as_the_owner_still_adds_one() {
    let (_database, machine, mut owner) = database_with_record().await;

    sqlx::query("UPDATE plc_connection SET row_version = 99 WHERE machine_id = $1")
        .bind(machine)
        .execute(&mut owner)
        .await
        .unwrap();

    assert_eq!(row_version_of(&mut owner, machine).await, 2);
}
