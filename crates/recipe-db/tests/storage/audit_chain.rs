//! Tests of `AuditChainReads` on PostgreSQL (task 4.4, design D14 row T6, `data-model.md` 4.9).
//!
//! Every test goes through the port of `recipe-app`. The adapter connects as `recipe_app`, so each
//! statement runs with the grants of that role. The tests write and break the chain as the
//! superuser of the harness, because `audit_owner` owns the audit tables. A break comes from a
//! plain `UPDATE` or `DELETE`, or from an insert with `session_replication_role = replica`, which
//! turns the trigger `audit_log_hash_chain` off. Each test has its own database, so no planted
//! break outlives its test.

use std::sync::Arc;

use recipe_app::ports::{AuditChainReads, BatchOutcome, ChainBounds, ChainPosition, ChainSeq};
use recipe_db::Database;
use recipe_db::settings::PoolSettings;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::Sha256;
use sqlx::PgConnection;

use crate::support::{Role, TestDatabase};

/// `audit.check_batch` (`reliability.md` 7.11).
const BATCH: u32 = 10_000;
/// `audit.check_batch_bytes` (`reliability.md` 7.11).
const BYTES: u64 = 8_388_608;
/// The rows of the chain in most tests.
const ROWS: i32 = 5;

/// The adapter as `recipe_app` and a session of the superuser to write and break the chain.
struct Fixture {
    chain: Arc<dyn AuditChainReads>,
    admin: PgConnection,
}

impl Fixture {
    /// A database with `rows` valid entries. The entries come from the trigger of section 4.9.
    async fn with_rows(rows: i32) -> Self {
        let database = TestDatabase::create().await;
        let app = Database::connect(&database.target(Role::App), &PoolSettings::default())
            .await
            .unwrap();
        let mut f = Self {
            chain: app.audit_chain(),
            admin: database.connect(Role::Superuser).await,
        };
        f.append(rows).await;
        f
    }

    /// Append `rows` entries with NULL chain columns, like the application does.
    async fn append(&mut self, rows: i32) {
        sqlx::query(
            "INSERT INTO audit_log \
                 (action, entity_type, entity_id, aggregate_type, aggregate_id, after_state, \
                  request_id) \
             SELECT 'CreateMachine', 'machine', g.id, 'machine', g.id, \
                    pg_catalog.jsonb_build_object('n', g.n), gen_random_uuid() \
             FROM (SELECT n, gen_random_uuid() AS id FROM generate_series(1, $1) AS n) AS g \
             ORDER BY g.n",
        )
        .bind(rows)
        .execute(&mut self.admin)
        .await
        .unwrap();
    }

    /// Plant a break with one statement of the superuser.
    async fn plant(&mut self, statement: &str) {
        sqlx::query(statement)
            .execute(&mut self.admin)
            .await
            .unwrap();
    }

    /// Insert a row with the given `chain_seq` and the hashes of row 1, with the trigger off.
    ///
    /// The statement moves no head. It stands for a superuser who writes outside the trigger.
    async fn plant_row_at(&mut self, seq: i64) {
        self.plant("SET session_replication_role = replica").await;
        sqlx::query(
            "INSERT INTO audit_log \
                 (chain_seq, prev_hash, entry_hash, action, entity_type, entity_id, \
                  aggregate_type, aggregate_id, after_state, request_id) \
             SELECT $1, entry_hash, entry_hash, 'CreateMachine', 'machine', gen_random_uuid(), \
                    'machine', gen_random_uuid(), '{\"n\": 0}'::jsonb, gen_random_uuid() \
             FROM audit_log WHERE chain_seq = 1",
        )
        .bind(seq)
        .execute(&mut self.admin)
        .await
        .unwrap();
    }

    /// Write the state row `audit.last_check_seq` with the given text.
    async fn plant_anchor(&mut self, text: &str) {
        sqlx::query("INSERT INTO app_setting (key, value) VALUES ('audit.last_check_seq', $1)")
            .bind(text)
            .execute(&mut self.admin)
            .await
            .unwrap();
    }

    /// The stored position of the row `seq`.
    async fn position(&mut self, seq: i64) -> ChainPosition {
        let (stored, hash): (i64, Vec<u8>) =
            sqlx::query_as("SELECT chain_seq, entry_hash FROM audit_log WHERE chain_seq = $1")
                .bind(seq)
                .fetch_one(&mut self.admin)
                .await
                .unwrap();
        ChainPosition {
            seq: ChainSeq::from(stored),
            hash: Some(Sha256::from(<[u8; 32]>::try_from(hash).unwrap())),
        }
    }

    /// The stored size of `before_state` and `after_state` of the rows `1..=through`.
    async fn bytes_through(&mut self, through: i64) -> u64 {
        let size: i64 = sqlx::query_scalar(
            "SELECT sum(COALESCE(pg_catalog.pg_column_size(before_state), 0) \
                      + COALESCE(pg_catalog.pg_column_size(after_state), 0))::bigint \
             FROM audit_log WHERE chain_seq <= $1",
        )
        .bind(through)
        .fetch_one(&mut self.admin)
        .await
        .unwrap();
        u64::try_from(size).unwrap()
    }

    /// Check from the start to `head` with the limits of `reliability.md` 7.11.
    async fn check_to(&self, head: i64) -> BatchOutcome {
        self.chain
            .check_batch(&ChainPosition::START, ChainSeq::from(head), BATCH, BYTES)
            .await
            .unwrap()
    }
}

/// The outcome of a batch of `rows` rows without a break that ends at the position `last`.
fn clean(rows: u64, last: ChainPosition) -> BatchOutcome {
    BatchOutcome::Clean {
        rows,
        last: Some(last),
    }
}

/// The row count and the last position of a clean batch, or `None` for any other outcome.
fn clean_parts(outcome: BatchOutcome) -> Option<(u64, ChainPosition)> {
    match outcome {
        BatchOutcome::Clean {
            rows,
            last: Some(last),
        } => Some((rows, last)),
        BatchOutcome::Clean { last: None, .. } | BatchOutcome::Broken { .. } => None,
    }
}

fn bounds(min: i64, max: i64) -> Option<ChainBounds> {
    Some(ChainBounds {
        min: ChainSeq::from(min),
        max: ChainSeq::from(max),
    })
}

/// A batch with a break at `first_break` and the last good row `last_clean` before it.
fn broken(first_break: i64, last_clean: i64) -> BatchOutcome {
    BatchOutcome::Broken {
        first_break: ChainSeq::from(first_break),
        last_clean: ChainSeq::from(last_clean),
    }
}

#[tokio::test]
async fn audit_chain_head_of_an_empty_log_is_the_start_position() {
    let f = Fixture::with_rows(0).await;

    let head = f.chain.head().await.unwrap();

    assert_eq!(head.position, ChainPosition::START);
    assert_eq!(head.bounds, None);
    assert_eq!(head.anchor, ChainSeq::ZERO);
}

#[tokio::test]
async fn audit_chain_check_of_an_empty_log_is_clean_without_a_row() {
    let f = Fixture::with_rows(0).await;

    let outcome = f.check_to(0).await;

    assert_eq!(
        outcome,
        BatchOutcome::Clean {
            rows: 0,
            last: None
        }
    );
}

#[tokio::test]
async fn audit_chain_head_is_the_last_row_of_the_log() {
    let mut f = Fixture::with_rows(ROWS).await;

    let head = f.chain.head().await.unwrap();

    assert_eq!(head.position, f.position(5).await);
    assert_eq!(head.bounds, bounds(1, 5));
    assert_eq!(head.anchor, ChainSeq::ZERO);
}

#[tokio::test]
async fn audit_chain_a_valid_chain_gives_the_end_without_a_break() {
    let f = Fixture::with_rows(ROWS).await;
    let head = f.chain.head().await.unwrap().position;

    let outcome = f.check_to(head.seq.get()).await;

    assert_eq!(outcome, clean(5, head));
}

#[tokio::test]
async fn audit_chain_a_changed_content_breaks_at_its_row() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant("UPDATE audit_log SET after_state = '{\"n\": 99}'::jsonb WHERE chain_seq = 3")
        .await;

    assert_eq!(f.check_to(5).await, broken(3, 2));
}

#[tokio::test]
async fn audit_chain_a_changed_entry_hash_breaks_at_its_row() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant("UPDATE audit_log SET entry_hash = pg_catalog.sha256(entry_hash) WHERE chain_seq = 3")
        .await;

    assert_eq!(f.check_to(5).await, broken(3, 2));
}

#[tokio::test]
async fn audit_chain_the_last_clean_row_is_the_after_value_when_the_first_row_breaks() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant("UPDATE audit_log SET entry_hash = pg_catalog.sha256(entry_hash) WHERE chain_seq = 3")
        .await;
    let after = f.position(2).await;

    let outcome = f
        .chain
        .check_batch(&after, ChainSeq::from(5), BATCH, BYTES)
        .await
        .unwrap();

    assert_eq!(outcome, broken(3, 2));
}

#[tokio::test]
async fn audit_chain_a_gap_with_sound_links_gives_the_row_before_the_gap_as_last_clean() {
    let mut f = Fixture::with_rows(2).await;
    f.plant_row_at(100).await;
    f.plant(
        "UPDATE audit_log SET prev_hash = \
             (SELECT p.entry_hash FROM audit_log AS p WHERE p.chain_seq = 2) \
         WHERE chain_seq = 100",
    )
    .await;
    f.plant(
        "UPDATE audit_log AS a SET entry_hash = pg_catalog.sha256(a.prev_hash \
             || pg_catalog.convert_to(public.audit_canonical(a), 'UTF8')) \
         WHERE a.chain_seq = 100",
    )
    .await;
    f.plant(
        "UPDATE audit_chain_head SET last_seq = 100, \
             last_hash = (SELECT entry_hash FROM audit_log WHERE chain_seq = 100) \
         WHERE singleton",
    )
    .await;
    let head = f.chain.head().await.unwrap().position;

    let outcome = f.check_to(head.seq.get()).await;

    assert_eq!(head, f.position(100).await);
    assert_eq!(outcome, broken(100, 2));
}

#[tokio::test]
async fn audit_chain_a_changed_prev_hash_breaks_at_its_row() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant("UPDATE audit_log SET prev_hash = pg_catalog.sha256(prev_hash) WHERE chain_seq = 4")
        .await;

    assert_eq!(f.check_to(5).await, broken(4, 3));
}

#[tokio::test]
async fn audit_chain_a_deleted_row_breaks_at_the_row_after_the_gap() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant("DELETE FROM audit_log WHERE chain_seq = 3").await;

    assert_eq!(f.check_to(5).await, broken(4, 2));
}

#[tokio::test]
async fn audit_chain_a_gap_with_valid_hash_links_breaks_at_the_row_after_the_gap() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant("DELETE FROM audit_log WHERE chain_seq = 3").await;
    f.plant(
        "UPDATE audit_log SET prev_hash = \
             (SELECT p.entry_hash FROM audit_log AS p WHERE p.chain_seq = 2) \
         WHERE chain_seq = 4",
    )
    .await;
    f.plant(
        "UPDATE audit_log AS a SET entry_hash = pg_catalog.sha256(a.prev_hash \
             || pg_catalog.convert_to(public.audit_canonical(a), 'UTF8')) \
         WHERE a.chain_seq = 4",
    )
    .await;

    assert_eq!(f.check_to(4).await, broken(4, 2));
}

#[tokio::test]
async fn audit_chain_a_row_inserted_with_the_trigger_off_after_a_gap_breaks_at_that_row() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant("SET session_replication_role = replica").await;
    f.plant(
        "INSERT INTO audit_log \
             (chain_seq, prev_hash, entry_hash, action, entity_type, entity_id, aggregate_type, \
              aggregate_id, after_state, request_id) \
         SELECT 7, entry_hash, entry_hash, 'CreateMachine', 'machine', gen_random_uuid(), \
                'machine', gen_random_uuid(), '{\"n\": 7}'::jsonb, gen_random_uuid() \
         FROM audit_log WHERE chain_seq = 5",
    )
    .await;

    assert_eq!(f.check_to(7).await, broken(7, 5));
}

#[tokio::test]
async fn audit_chain_a_deleted_last_row_gives_a_clean_batch_that_ends_before_the_head() {
    let mut f = Fixture::with_rows(ROWS).await;
    let head = f.chain.head().await.unwrap().position;
    f.plant("DELETE FROM audit_log WHERE chain_seq = 5").await;

    let outcome = f.check_to(head.seq.get()).await;

    let fourth = f.position(4).await;
    assert_eq!(outcome, clean(4, fourth));
    assert_ne!(fourth, head, "the job compares the end with the head");
}

#[tokio::test]
async fn audit_chain_a_check_after_a_good_row_finds_a_later_break() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant("UPDATE audit_log SET after_state = '{\"n\": 99}'::jsonb WHERE chain_seq = 4")
        .await;
    let after = f.position(2).await;

    let outcome = f
        .chain
        .check_batch(&after, ChainSeq::from(5), BATCH, BYTES)
        .await
        .unwrap();

    assert_eq!(outcome, broken(4, 3));
}

#[tokio::test]
async fn audit_chain_a_carried_hash_that_is_not_the_previous_hash_breaks_at_the_first_row() {
    let f = Fixture::with_rows(ROWS).await;
    let after = ChainPosition {
        seq: ChainSeq::from(2),
        hash: Some(Sha256::from([0x11_u8; 32])),
    };

    let outcome = f
        .chain
        .check_batch(&after, ChainSeq::from(5), BATCH, BYTES)
        .await
        .unwrap();

    assert_eq!(outcome, broken(3, 2));
}

#[tokio::test]
async fn audit_chain_the_row_limit_ends_a_batch_and_the_next_call_continues_after_it() {
    let mut f = Fixture::with_rows(ROWS).await;
    let mut after = ChainPosition::START;
    let mut seen = Vec::new();

    for _ in 0..3 {
        let outcome = f
            .chain
            .check_batch(&after, ChainSeq::from(5), 2, BYTES)
            .await
            .unwrap();
        let (rows, last) = clean_parts(outcome).unwrap();
        seen.push(rows);
        after = last;
    }

    assert_eq!(seen, vec![2, 2, 1]);
    assert_eq!(after, f.position(5).await);
}

#[tokio::test]
async fn audit_chain_the_byte_limit_ends_a_batch() {
    let mut f = Fixture::with_rows(ROWS).await;
    let two_rows = f.bytes_through(2).await;
    let third = f.position(3).await;

    let exact = f
        .chain
        .check_batch(&ChainPosition::START, ChainSeq::from(5), BATCH, two_rows)
        .await
        .unwrap();
    let one_short = f
        .chain
        .check_batch(
            &ChainPosition::START,
            ChainSeq::from(5),
            BATCH,
            two_rows.checked_sub(1).unwrap(),
        )
        .await
        .unwrap();

    let second = f.position(2).await;
    let first = f.position(1).await;
    assert_eq!((exact, one_short), (clean(2, second), clean(1, first)));
    assert_ne!(second, third);
}

#[tokio::test]
async fn audit_chain_a_batch_holds_one_row_when_the_first_row_is_over_the_byte_limit() {
    let mut f = Fixture::with_rows(ROWS).await;

    let outcome = f
        .chain
        .check_batch(&ChainPosition::START, ChainSeq::from(5), BATCH, 1)
        .await
        .unwrap();

    assert_eq!(outcome, clean(1, f.position(1).await));
}

#[tokio::test]
async fn audit_chain_a_batch_size_of_zero_holds_one_row() {
    let mut f = Fixture::with_rows(ROWS).await;

    let outcome = f
        .chain
        .check_batch(&ChainPosition::START, ChainSeq::from(5), 0, BYTES)
        .await
        .unwrap();

    assert_eq!(outcome, clean(1, f.position(1).await));
}

#[tokio::test]
async fn audit_chain_the_check_stops_at_the_head_of_the_range() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant("UPDATE audit_log SET after_state = '{\"n\": 99}'::jsonb WHERE chain_seq = 4")
        .await;

    let outcome = f.check_to(3).await;

    assert_eq!(outcome, clean(3, f.position(3).await));
}

#[tokio::test]
async fn audit_chain_a_break_in_a_later_batch_is_found_with_the_carried_position() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant("UPDATE audit_log SET after_state = '{\"n\": 99}'::jsonb WHERE chain_seq = 4")
        .await;

    let first = f
        .chain
        .check_batch(&ChainPosition::START, ChainSeq::from(5), 2, BYTES)
        .await
        .unwrap();
    let carried = f.position(2).await;
    let second = f
        .chain
        .check_batch(&carried, ChainSeq::from(5), 2, BYTES)
        .await
        .unwrap();

    assert_eq!((first, second), (clean(2, carried), broken(4, 3)));
}

#[tokio::test]
async fn audit_chain_head_gives_the_bounds_of_a_row_at_zero_without_an_error() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant_row_at(0).await;

    let head = f.chain.head().await.unwrap();

    assert_eq!(head.bounds, bounds(0, 5));
    assert_eq!(head.position, f.position(5).await);
}

#[tokio::test]
async fn audit_chain_head_gives_the_bounds_of_a_row_below_zero_without_an_error() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant_row_at(-3).await;

    let head = f.chain.head().await.unwrap();

    assert_eq!(head.bounds, bounds(-3, 5));
    assert_eq!(head.position, f.position(5).await);
}

#[tokio::test]
async fn audit_chain_head_gives_the_bounds_of_a_row_above_the_head_without_an_error() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant_row_at(9).await;

    let head = f.chain.head().await.unwrap();

    assert_eq!(head.bounds, bounds(1, 9));
    assert_eq!(head.position, f.position(5).await);
}

#[tokio::test]
async fn audit_chain_head_without_a_hash_after_the_start_is_returned_as_read() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant("UPDATE audit_chain_head SET last_hash = NULL WHERE singleton")
        .await;

    let head = f.chain.head().await.unwrap();

    assert_eq!(
        head.position,
        ChainPosition {
            seq: ChainSeq::from(5),
            hash: None
        }
    );
    assert_eq!(head.bounds, bounds(1, 5));
}

#[tokio::test]
async fn audit_chain_head_reads_the_anchor_from_the_state_row() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant_anchor("41").await;

    let head = f.chain.head().await.unwrap();

    assert_eq!(head.anchor, ChainSeq::from(41));
}

#[tokio::test]
async fn audit_chain_head_without_an_anchor_row_gives_the_anchor_zero() {
    let f = Fixture::with_rows(ROWS).await;

    let head = f.chain.head().await.unwrap();

    assert_eq!(head.anchor, ChainSeq::ZERO);
}

#[tokio::test]
async fn audit_chain_head_with_a_bad_anchor_text_is_a_storage_error_without_the_text() {
    let mut f = Fixture::with_rows(ROWS).await;
    f.plant_anchor("9z9").await;

    let error = f.chain.head().await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(!error.to_string().contains("9z9"), "error: {error}");
}
