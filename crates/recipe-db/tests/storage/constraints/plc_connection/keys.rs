//! Tests of the primary key and the row version of `plc_connection`.

use super::*;

/// The same machine and the same id cannot hold two rows. The key `id` stops the second row.
#[tokio::test]
async fn plc_connection_duplicate_id_violates_the_primary_key() {
    let mut conn = connection().await;
    let first = fixtures::machine(&mut conn, S7_1500).await;
    let second = fixtures::machine(&mut conn, S7_1500).await;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, rack, slot, db_number) \
         VALUES ($1, 'S7_1500', '192.168.1.20', 0, 2, 10) RETURNING id",
    )
    .bind(first)
    .fetch_one(&mut conn)
    .await
    .unwrap();

    let result = sqlx::query(
        "INSERT INTO plc_connection (id, machine_id, plc_platform, host, rack, slot, db_number) \
         VALUES ($1, $2, 'S7_1500', '192.168.1.21', 0, 2, 10)",
    )
    .bind(id)
    .bind(second)
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_connection_pkey");
}

/// The trigger `plc_connection_a_bump` sets `row_version` on every update, so an insert is the
/// only statement that can break the check.
#[tokio::test]
async fn plc_connection_row_version_below_1_violates_the_row_version_check() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;

    for row_version in [0_i64, -1] {
        let result = sqlx::query(
            "INSERT INTO plc_connection \
                 (machine_id, plc_platform, host, rack, slot, db_number, row_version) \
             VALUES ($1, 'S7_1500', '192.168.1.20', 0, 2, 10, $2)",
        )
        .bind(machine)
        .bind(row_version)
        .execute(&mut conn)
        .await;

        assert_constraint(
            result,
            CHECK_VIOLATION,
            "plc_connection_row_version_positive",
        );
    }
}
