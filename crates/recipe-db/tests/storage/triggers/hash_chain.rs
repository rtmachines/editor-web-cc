//! Tests of the section 4.9: `audit_hash_chain` and the batch statement of the nightly check.
//!
//! The tests run as the superuser of the harness, because `audit_owner` owns the audit tables.
//! The tests send `chain_seq`, `prev_hash`, and `entry_hash` as NULL, like the application does.

use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::support::{Role, TestDatabase};

/// The batch statement of `data-model.md` section 4.9 (step 2 of the nightly check).
/// `$1` is `$after_seq`, `$2` is `$head_seq`, `$3` is `$batch`, `$4` is `$batch_bytes`,
/// and `$5` is `$after_hash`.
const CHECK_BATCH: &str = "\
WITH sized AS (
    SELECT a.chain_seq,
           sum(COALESCE(pg_catalog.pg_column_size(a.before_state), 0)
               + COALESCE(pg_catalog.pg_column_size(a.after_state), 0))
               OVER (ORDER BY a.chain_seq) AS running_bytes
    FROM public.audit_log AS a
    WHERE a.chain_seq > $1::bigint AND a.chain_seq <= $2::bigint
    ORDER BY a.chain_seq
    LIMIT $3::bigint
), bounds AS (
    SELECT COALESCE(max(s.chain_seq) FILTER (WHERE s.running_bytes <= $4::bigint),
                    min(s.chain_seq)) AS last_in_batch
    FROM sized AS s
), batch AS (
    SELECT a.chain_seq, a.prev_hash, a.entry_hash,
           pg_catalog.sha256(COALESCE(a.prev_hash, '\\x'::bytea)
               || pg_catalog.convert_to(public.audit_canonical(a), 'UTF8')) AS recomputed
    FROM public.audit_log AS a CROSS JOIN bounds AS b
    WHERE a.chain_seq > $1::bigint AND a.chain_seq <= b.last_in_batch
), chain AS (
    SELECT b.*,
           lag(b.chain_seq, 1, $1::bigint)          OVER w AS previous_seq,
           lag(b.entry_hash, 1, $5::bytea) OVER w AS previous_hash
    FROM batch AS b
    WINDOW w AS (ORDER BY b.chain_seq)
)
SELECT min(chain_seq) FILTER (
           WHERE chain_seq IS DISTINCT FROM previous_seq + 1
              OR prev_hash IS DISTINCT FROM previous_hash
              OR entry_hash IS DISTINCT FROM recomputed)   AS first_break,
       count(*)                                            AS row_count,
       max(chain_seq)                                      AS last_seq,
       (array_agg(entry_hash ORDER BY chain_seq DESC))[1]  AS last_hash
FROM chain";

const BATCH_ROWS: i64 = 1000;
const BATCH_BYTES: i64 = 1_000_000;
const MANY_ROWS: usize = 60;

/// The result of the batch statement: first break, row count, last sequence, and last hash.
pub type CheckResult = (Option<i64>, i64, Option<i64>, Option<Vec<u8>>);

async fn connection() -> PgConnection {
    // After 0003, `audit_owner` owns the audit tables and `recipe_migrator` holds no privilege on
    // them. The superuser of the harness writes the audit rows (design D12).
    TestDatabase::create().await.connect(Role::Superuser).await
}

/// Insert one valid entry with NULL chain columns and return its `chain_seq`.
async fn insert_entry(conn: &mut PgConnection) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO audit_log \
             (action, entity_type, entity_id, aggregate_type, aggregate_id, after_state, request_id) \
         VALUES ('CreateMachine', 'machine', $1, 'machine', $1, '{\"a\":1}'::jsonb, $2) \
         RETURNING chain_seq",
    )
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .fetch_one(conn)
    .await
    .unwrap()
}

/// The pair `(prev_hash, entry_hash)` of a row.
async fn hashes(conn: &mut PgConnection, seq: i64) -> (Option<Vec<u8>>, Vec<u8>) {
    sqlx::query_as("SELECT prev_hash, entry_hash FROM audit_log WHERE chain_seq = $1")
        .bind(seq)
        .fetch_one(conn)
        .await
        .unwrap()
}

async fn head(conn: &mut PgConnection) -> (i64, Option<Vec<u8>>) {
    sqlx::query_as("SELECT last_seq, last_hash FROM audit_chain_head WHERE singleton")
        .fetch_one(conn)
        .await
        .unwrap()
}

/// Run the check statement over the whole chain in one batch.
pub async fn check(conn: &mut PgConnection) -> CheckResult {
    let (head_seq, _) = head(conn).await;
    sqlx::query_as(CHECK_BATCH)
        .bind(0_i64)
        .bind(head_seq)
        .bind(BATCH_ROWS)
        .bind(BATCH_BYTES)
        .bind(None::<Vec<u8>>)
        .fetch_one(conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn the_first_row_gets_sequence_1_and_a_null_prev_hash() {
    let mut conn = connection().await;

    let seq = insert_entry(&mut conn).await;

    let (prev, entry) = hashes(&mut conn, seq).await;
    assert_eq!((seq, prev, entry.len()), (1, None, 32));
}

#[tokio::test]
async fn a_later_row_gets_the_next_sequence_and_the_hash_of_the_head() {
    let mut conn = connection().await;
    insert_entry(&mut conn).await;
    let (_, first_hash) = hashes(&mut conn, 1).await;

    let seq = insert_entry(&mut conn).await;

    let (prev, entry) = hashes(&mut conn, seq).await;
    assert_eq!((seq, prev, entry.len()), (2, Some(first_hash), 32));
}

#[tokio::test]
async fn the_head_moves_to_the_new_row() {
    let mut conn = connection().await;
    insert_entry(&mut conn).await;
    let seq = insert_entry(&mut conn).await;

    let (_, entry) = hashes(&mut conn, seq).await;

    assert_eq!(head(&mut conn).await, (seq, Some(entry)));
}

#[tokio::test]
async fn a_multi_row_insert_gets_consecutive_sequences_in_the_order_of_its_rows() {
    let mut conn = connection().await;
    let ids: Vec<Uuid> = (0..MANY_ROWS).map(|_| Uuid::new_v4()).collect();

    sqlx::query(
        "INSERT INTO audit_log \
             (action, entity_type, entity_id, aggregate_type, aggregate_id, after_state, request_id) \
         SELECT 'CreateMachine', 'machine', t.id, 'machine', t.id, '{\"a\":1}'::jsonb, $2 \
         FROM unnest($1::uuid[]) WITH ORDINALITY AS t(id, ordinality) \
         ORDER BY t.ordinality",
    )
    .bind(&ids)
    .bind(Uuid::new_v4())
    .execute(&mut conn)
    .await
    .unwrap();

    let stored: Vec<(i64, Uuid)> =
        sqlx::query_as("SELECT chain_seq, entity_id FROM audit_log ORDER BY chain_seq")
            .fetch_all(&mut conn)
            .await
            .unwrap();
    let seqs: Vec<i64> = stored.iter().map(|(seq, _)| *seq).collect();
    let stored_ids: Vec<Uuid> = stored.iter().map(|(_, id)| *id).collect();
    let expected: Vec<i64> = (1..=i64::try_from(MANY_ROWS).unwrap()).collect();
    assert_eq!((seqs, stored_ids), (expected, ids));
    assert_eq!(check(&mut conn).await.0, None);
}

#[tokio::test]
async fn the_check_statement_finds_no_break_in_a_valid_chain() {
    let mut conn = connection().await;
    for _ in 0..5 {
        insert_entry(&mut conn).await;
    }

    let (first_break, row_count, last_seq, last_hash) = check(&mut conn).await;

    let (_, entry) = hashes(&mut conn, 5).await;
    assert_eq!(
        (first_break, row_count, last_seq, last_hash),
        (None, 5, Some(5), Some(entry))
    );
}

#[tokio::test]
async fn the_check_statement_finds_a_changed_after_state_at_its_sequence() {
    let mut conn = connection().await;
    for _ in 0..5 {
        insert_entry(&mut conn).await;
    }
    sqlx::query("UPDATE audit_log SET after_state = '{\"a\":2}'::jsonb WHERE chain_seq = 3")
        .execute(&mut conn)
        .await
        .unwrap();

    let (first_break, ..) = check(&mut conn).await;

    assert_eq!(first_break, Some(3));
}

/// `recipe_app` holds `INSERT` on all columns of `audit_log`. Trigger 4.9 must replace the three
/// chain columns, so a forged value never reaches the table (review Q8).
#[tokio::test]
async fn recipe_app_cannot_forge_the_chain_columns_of_an_insert() {
    const FORGED_SEQ: i64 = 99;
    let forged_prev = vec![0x11_u8; 32];
    let forged_hash = vec![0x22_u8; 32];
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;
    insert_entry(&mut admin).await;
    let (_, first_hash) = hashes(&mut admin, 1).await;
    let mut app = database.connect(Role::App).await;

    let seq: i64 = sqlx::query_scalar(
        "INSERT INTO audit_log \
             (chain_seq, prev_hash, entry_hash, action, entity_type, entity_id, aggregate_type, \
              aggregate_id, after_state, request_id) \
         VALUES ($1, $2, $3, 'CreateMachine', 'machine', $4, 'machine', $4, \
                 '{\"a\":1}'::jsonb, $4) \
         RETURNING chain_seq",
    )
    .bind(FORGED_SEQ)
    .bind(&forged_prev)
    .bind(&forged_hash)
    .bind(Uuid::new_v4())
    .fetch_one(&mut app)
    .await
    .unwrap();

    assert_eq!(seq, 2);
    let (prev, entry) = hashes(&mut admin, 2).await;
    assert_eq!(prev, Some(first_hash));
    assert_ne!(entry, forged_hash);
    let forged_rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE chain_seq = $1")
            .bind(FORGED_SEQ)
            .fetch_one(&mut admin)
            .await
            .unwrap();
    assert_eq!(forged_rows, 0);
    assert_eq!(head(&mut admin).await, (2, Some(entry)));
    assert_eq!(check(&mut admin).await.0, None);
}

#[tokio::test]
async fn a_rollback_after_an_audit_insert_leaves_no_gap() {
    let mut conn = connection().await;
    insert_entry(&mut conn).await;
    let mut tx = conn.begin().await.unwrap();
    assert_eq!(insert_entry(&mut tx).await, 2);
    tx.rollback().await.unwrap();

    let seq = insert_entry(&mut conn).await;

    let (prev, _) = hashes(&mut conn, seq).await;
    let (_, first_hash) = hashes(&mut conn, 1).await;
    assert_eq!((seq, prev), (2, Some(first_hash)));
    assert_eq!(check(&mut conn).await.0, None);
}
