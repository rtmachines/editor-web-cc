//! Tests of the section 4.6: `plc_type_immutable` and `plc_type_struct_has_member`.
//!
//! The tests run as the table owner `recipe_migrator`. A struct needs a member in the transaction
//! that creates it.

use recipe_domain::plant::{PlcType, Primitive};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::contract::harness::{begin, commit};
use crate::contract::postgres_harness::PgHarness;
use crate::contract::seed::{insert, machine, seed_type, struct_record, type_id};
use crate::support::fixtures::{self, Draft, TWIN_CAT};
use crate::support::{Role, TestDatabase, assert_trigger};

async fn plant() -> (PgConnection, Draft) {
    let mut conn = TestDatabase::create()
        .await
        .into_session(Role::Migrator)
        .await;
    let draft = Draft::create(&mut conn).await;
    (conn, draft)
}

/// A plant with one committed struct that has one member.
async fn plant_with_struct() -> (PgConnection, Draft, Uuid) {
    let (mut conn, draft) = plant().await;
    let id =
        fixtures::complete_struct(&mut conn, draft.machine, TWIN_CAT, "Pair", draft.dint).await;
    (conn, draft, id)
}

async fn count(conn: &mut PgConnection, query: &'static str, id: Uuid) -> i64 {
    sqlx::query_scalar(query)
        .bind(id)
        .fetch_one(conn)
        .await
        .unwrap()
}

const TYPE_ROWS: &str = "SELECT count(*) FROM plc_type WHERE id = $1";
const MEMBER_ROWS: &str = "SELECT count(*) FROM plc_type_member WHERE struct_type_id = $1";

// ---- E-PLT-036: a struct has a member at commit ----

#[tokio::test]
async fn e_plt_036_a_struct_with_members_in_its_creating_transaction_commits() {
    let (mut conn, draft) = plant().await;
    let mut tx = conn.begin().await.unwrap();
    let id = fixtures::struct_type(&mut tx, draft.machine, TWIN_CAT, "Pair").await;
    fixtures::member(&mut tx, id, 1, draft.dint).await;
    fixtures::member(&mut tx, id, 2, draft.dint).await;

    tx.commit().await.unwrap();

    assert_eq!(count(&mut conn, TYPE_ROWS, id).await, 1);
    assert_eq!(count(&mut conn, MEMBER_ROWS, id).await, 2);
}

#[tokio::test]
async fn e_plt_036_a_struct_without_a_member_is_rejected_at_commit() {
    let (mut conn, draft) = plant().await;
    let mut tx = conn.begin().await.unwrap();
    let id = fixtures::struct_type(&mut tx, draft.machine, TWIN_CAT, "Empty").await;

    let commit = tx.commit().await;

    assert_trigger(commit, "E-PLT-036");
    assert_eq!(count(&mut conn, TYPE_ROWS, id).await, 0);
}

#[tokio::test]
async fn e_plt_036_a_primitive_needs_no_member() {
    let (mut conn, _draft) = plant().await;

    let id = fixtures::primitive_on(&mut conn, "LONELY", &[TWIN_CAT]).await;

    assert_eq!(count(&mut conn, TYPE_ROWS, id).await, 1);
}

// ---- E-PLT-033: rows are immutable ----

#[tokio::test]
async fn e_plt_033_an_insert_of_a_type_and_of_a_member_in_one_transaction_passes() {
    let (mut conn, _draft, id) = plant_with_struct().await;

    assert_eq!(count(&mut conn, TYPE_ROWS, id).await, 1);
    assert_eq!(count(&mut conn, MEMBER_ROWS, id).await, 1);
}

#[tokio::test]
async fn e_plt_033_a_type_row_cannot_be_updated_or_deleted() {
    let (mut conn, _draft, id) = plant_with_struct().await;

    let update = sqlx::query("UPDATE plc_type SET revision = 2 WHERE id = $1")
        .bind(id)
        .execute(&mut conn)
        .await;
    let delete = sqlx::query("DELETE FROM plc_type WHERE id = $1")
        .bind(id)
        .execute(&mut conn)
        .await;

    assert_trigger(update, "E-PLT-033");
    assert_trigger(delete, "E-PLT-033");
    assert_eq!(count(&mut conn, TYPE_ROWS, id).await, 1);
}

#[tokio::test]
async fn e_plt_033_a_primitive_row_cannot_be_updated_or_deleted() {
    let (mut conn, draft) = plant().await;

    let update = sqlx::query("UPDATE plc_type SET size_bytes = 8 WHERE id = $1")
        .bind(draft.dint)
        .execute(&mut conn)
        .await;
    let delete = sqlx::query("DELETE FROM plc_type WHERE id = $1")
        .bind(draft.dint)
        .execute(&mut conn)
        .await;

    assert_trigger(update, "E-PLT-033");
    assert_trigger(delete, "E-PLT-033");
}

#[tokio::test]
async fn e_plt_033_a_member_row_cannot_be_updated_or_deleted() {
    let (mut conn, _draft, id) = plant_with_struct().await;

    let update =
        sqlx::query("UPDATE plc_type_member SET member_name = 'other' WHERE struct_type_id = $1")
            .bind(id)
            .execute(&mut conn)
            .await;
    let delete = sqlx::query("DELETE FROM plc_type_member WHERE struct_type_id = $1")
        .bind(id)
        .execute(&mut conn)
        .await;

    assert_trigger(update, "E-PLT-033");
    assert_trigger(delete, "E-PLT-033");
    assert_eq!(count(&mut conn, MEMBER_ROWS, id).await, 1);
}

// ---- E-PLT-034: members of a struct close with its transaction ----

#[tokio::test]
async fn e_plt_034_a_second_member_in_the_creating_transaction_passes() {
    let (mut conn, draft) = plant().await;
    let mut tx = conn.begin().await.unwrap();
    let id = fixtures::struct_type(&mut tx, draft.machine, TWIN_CAT, "Pair").await;
    fixtures::member(&mut tx, id, 1, draft.dint).await;

    let second = sqlx::query(
        "INSERT INTO plc_type_member (struct_type_id, ordinal, member_name, member_type_id) \
         VALUES ($1, 2, 'second', $2)",
    )
    .bind(id)
    .bind(draft.dint)
    .execute(&mut *tx)
    .await;
    tx.commit().await.unwrap();

    assert_eq!(second.unwrap().rows_affected(), 1);
    assert_eq!(count(&mut conn, MEMBER_ROWS, id).await, 2);
}

#[tokio::test]
async fn e_plt_034_a_member_in_a_later_transaction_is_rejected() {
    let (mut conn, draft, id) = plant_with_struct().await;

    let late = sqlx::query(
        "INSERT INTO plc_type_member (struct_type_id, ordinal, member_name, member_type_id) \
         VALUES ($1, 2, 'late', $2)",
    )
    .bind(id)
    .bind(draft.dint)
    .execute(&mut conn)
    .await;

    assert_trigger(late, "E-PLT-034");
    assert_eq!(count(&mut conn, MEMBER_ROWS, id).await, 1);
}

/// A member insert with the values of `second_member_insert`, ordinal 2.
const SECOND_MEMBER: &str = "INSERT INTO plc_type_member \
     (struct_type_id, ordinal, member_name, member_type_id) VALUES ($1, 2, 'forged', $2)";

#[tokio::test]
async fn member_insert_with_forged_created_at_gives_e_plt_034() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let mut admin = database.connect(Role::Superuser).await;
    let draft = Draft::create(&mut conn).await;
    let id =
        fixtures::complete_struct(&mut conn, draft.machine, TWIN_CAT, "Pair", draft.dint).await;
    // Transaction B starts after the struct is committed. The forged time is its start time.
    let mut tx = conn.begin().await.unwrap();
    let started: String = sqlx::query_scalar("SELECT now()::text")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SET session_replication_role = replica")
        .execute(&mut admin)
        .await
        .unwrap();
    sqlx::query("UPDATE plc_type SET created_at = $1::timestamptz WHERE id = $2")
        .bind(&started)
        .bind(id)
        .execute(&mut admin)
        .await
        .unwrap();

    let forged = sqlx::query(SECOND_MEMBER)
        .bind(id)
        .bind(draft.dint)
        .execute(&mut *tx)
        .await;

    assert_trigger(forged, "E-PLT-034");
    database.keep_until_thread_ends();
}

/// The `xmin` clause passes here: the struct row version belongs to the current transaction.
/// Only the `created_at` clause refuses the member (ADR-0212).
#[tokio::test]
async fn member_insert_with_own_xmin_and_other_created_at_gives_e_plt_034() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let mut admin = database.connect(Role::Superuser).await;
    let draft = Draft::create(&mut conn).await;
    let mut tx = admin.begin().await.unwrap();
    let id = fixtures::struct_type(&mut tx, draft.machine, TWIN_CAT, "Pair").await;
    // The update writes a new row version. Its `xmin` is the current transaction.
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE plc_type SET created_at = created_at - interval '1 day' WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SET LOCAL session_replication_role = origin")
        .execute(&mut *tx)
        .await
        .unwrap();

    let forged = sqlx::query(SECOND_MEMBER)
        .bind(id)
        .bind(draft.dint)
        .execute(&mut *tx)
        .await;

    assert_trigger(forged, "E-PLT-034");
    database.keep_until_thread_ends();
}

#[tokio::test]
async fn member_insert_without_a_visible_struct_gives_e_plt_035() {
    let (mut conn, draft) = plant().await;
    // A deferred foreign key lets the insert reach the trigger without a visible parent row.
    sqlx::query(
        "ALTER TABLE plc_type_member ALTER CONSTRAINT plc_type_member_struct_type_id_fkey \
         DEFERRABLE INITIALLY DEFERRED",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    let mut tx = conn.begin().await.unwrap();

    let orphan = sqlx::query(SECOND_MEMBER)
        .bind(Uuid::new_v4())
        .bind(draft.dint)
        .execute(&mut *tx)
        .await;

    assert_trigger(orphan, "E-PLT-035");
}

#[tokio::test]
async fn a_struct_and_its_members_through_the_adapter_pass_both_tests() {
    let harness = PgHarness::create().await;
    let built = machine(160, "M160");
    insert(&harness, &built).await;
    let dint = seed_type(&harness, built.id(), Primitive::Dint).await;
    let boolean = seed_type(&harness, built.id(), Primitive::Bool).await;
    let record = struct_record(
        &built,
        type_id(60),
        "Pump",
        1,
        None,
        &[("speed", dint), ("running", boolean)],
    );
    let row = PlcType::try_from(record).unwrap();
    let mut tx = begin(&harness).await;

    tx.types().insert_struct(&row).await.unwrap();
    commit(&harness, tx, built.id()).await;

    let mut conn = harness.database.connect(Role::Migrator).await;
    let members = count(&mut conn, MEMBER_ROWS, *type_id(60).as_uuid()).await;
    assert_eq!(members, 2);
}

// ---- E-PLT-035: only a struct has members ----

#[tokio::test]
async fn e_plt_035_a_member_of_a_struct_passes() {
    let (mut conn, _draft, id) = plant_with_struct().await;

    assert_eq!(count(&mut conn, MEMBER_ROWS, id).await, 1);
}

#[tokio::test]
async fn e_plt_035_a_member_of_a_primitive_is_rejected() {
    let (mut conn, draft) = plant().await;

    let member = sqlx::query(
        "INSERT INTO plc_type_member (struct_type_id, ordinal, member_name, member_type_id) \
         VALUES ($1, 1, 'm1', $2)",
    )
    .bind(draft.dint)
    .bind(draft.dint)
    .execute(&mut conn)
    .await;

    assert_trigger(member, "E-PLT-035");
}
