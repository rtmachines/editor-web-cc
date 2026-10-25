//! `AuditChainReads` on PostgreSQL (task 4.4, ADR-0022, ADR-0060, `data-model.md` section 4.9).
//!
//! The role `recipe_app` runs every statement of this file with the grants of `grants.expected`:
//! `SELECT` on `audit_log`, on `audit_chain_head`, and on `app_setting`. The function `audit_canonical` has the
//! default `EXECUTE` for every role, so the check needs no other grant.

use std::time::Duration;

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{
    AuditChainReads, BatchOutcome, ChainBounds, ChainHead, ChainPosition, ChainSeq,
    parse_chain_seq_text,
};
use recipe_domain::export::Sha256;
use sqlx::{PgPool, Postgres, Transaction};

use crate::error::{corrupt, map_error, missing_row};

/// The table name in a defect of the anchor row.
const SETTING_TABLE: &str = "app_setting";

/// The table name in a defect of a row of the audit log.
const LOG_TABLE: &str = "audit_log";

/// The table name in a defect of the head row.
const HEAD_TABLE: &str = "audit_chain_head";

/// `audit.check_statement_timeout` (`reliability.md` section 7.11). It bounds one batch.
const BATCH_STATEMENT_TIMEOUT: Duration = Duration::from_secs(60);

/// The start of every transaction of this file. Each method has one batch and no retry.
const BEGIN: &str = "BEGIN ISOLATION LEVEL READ COMMITTED READ ONLY";

/// The settings of one transaction. `SET LOCAL` takes no bind, so `set_config` with the third
/// argument `true` sets the same local values. `$1` is the statement timeout as text.
const LOCAL_SETTINGS: &str = "SELECT \
     set_config('TimeZone', 'UTC', true), \
     set_config('statement_timeout', $1, true)";

/// Step 1 of `data-model.md` section 4.9 and ADR-0086 (a): the end of the range of this run, the
/// bounds of `audit_log`, and the anchor.
///
/// The unique index `audit_log_chain_seq_unique` serves `min` and `max`. An empty table gives
/// NULL bounds. A missing anchor row gives a NULL anchor.
const READ_HEAD: &str = "\
SELECT h.last_seq, h.last_hash, \
       (SELECT min(a.chain_seq) FROM public.audit_log AS a) AS min_seq, \
       (SELECT max(a.chain_seq) FROM public.audit_log AS a) AS max_seq, \
       (SELECT s.value FROM public.app_setting AS s \
        WHERE s.key = 'audit.last_check_seq') AS anchor \
FROM public.audit_chain_head AS h WHERE h.singleton";

/// Step 2 of `data-model.md` section 4.9: one batch.
///
/// `$1` is `$after_seq`, `$2` is `$head_seq`, `$3` is `$batch`, `$4` is `$batch_bytes`, and `$5` is
/// `$after_hash`. The unique index `audit_log_chain_seq_unique` serves the range scan.
const CHECK_BATCH: &str = "\
WITH sized AS ( \
    SELECT a.chain_seq, \
           sum(COALESCE(pg_catalog.pg_column_size(a.before_state), 0) \
               + COALESCE(pg_catalog.pg_column_size(a.after_state), 0)) \
               OVER (ORDER BY a.chain_seq) AS running_bytes \
    FROM public.audit_log AS a \
    WHERE a.chain_seq > $1::bigint AND a.chain_seq <= $2::bigint \
    ORDER BY a.chain_seq \
    LIMIT $3::bigint \
), bounds AS ( \
    SELECT COALESCE(max(s.chain_seq) FILTER (WHERE s.running_bytes <= $4::bigint), \
                    min(s.chain_seq)) AS last_in_batch \
    FROM sized AS s \
), batch AS ( \
    SELECT a.chain_seq, a.prev_hash, a.entry_hash, \
           pg_catalog.sha256(COALESCE(a.prev_hash, '\\x'::bytea) \
               || pg_catalog.convert_to(public.audit_canonical(a), 'UTF8')) AS recomputed \
    FROM public.audit_log AS a CROSS JOIN bounds AS b \
    WHERE a.chain_seq > $1::bigint AND a.chain_seq <= b.last_in_batch \
), chain AS ( \
    SELECT b.*, \
           lag(b.chain_seq, 1, $1::bigint) OVER w AS previous_seq, \
           lag(b.entry_hash, 1, $5::bytea) OVER w AS previous_hash \
    FROM batch AS b \
    WINDOW w AS (ORDER BY b.chain_seq) \
), checked AS ( \
    SELECT c.chain_seq, c.previous_seq, c.entry_hash, \
           (c.chain_seq IS DISTINCT FROM c.previous_seq + 1 \
            OR c.prev_hash IS DISTINCT FROM c.previous_hash \
            OR c.entry_hash IS DISTINCT FROM c.recomputed) AS broken \
    FROM chain AS c \
) \
SELECT min(chain_seq) FILTER (WHERE broken) AS first_break, \
       min(previous_seq) FILTER (WHERE broken) AS last_clean, \
       count(*) AS row_count, \
       max(chain_seq) AS last_seq, \
       (array_agg(entry_hash ORDER BY chain_seq DESC))[1] AS last_hash \
FROM checked";

/// The columns of the head row: `last_seq`, `last_hash`, `min_seq`, `max_seq`, and `anchor`.
type HeadRow = (
    i64,
    Option<Vec<u8>>,
    Option<i64>,
    Option<i64>,
    Option<String>,
);

/// The columns of the batch row: `first_break`, `last_clean`, `row_count`, `last_seq`, and
/// `last_hash`.
type BatchRow = (Option<i64>, Option<i64>, i64, Option<i64>, Option<Vec<u8>>);

/// The reads of the audit chain check on `recipe-db`.
///
/// The type solves one problem: the check reads a table that grows without limit, and a long
/// snapshot would hold back VACUUM. Each method therefore runs one statement in its own short
/// `READ ONLY` transaction, and the job carries the position from one batch to the next.
pub(crate) struct PgAuditChainReads {
    pool: PgPool,
}

impl PgAuditChainReads {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Start a `READ ONLY` transaction and set the local timeout and time zone.
    async fn begin_local(&self) -> Result<Transaction<'static, Postgres>, StorageError> {
        let timeout = format!("{}ms", BATCH_STATEMENT_TIMEOUT.as_millis());
        let mut tx = self.pool.begin_with(BEGIN).await.map_err(map_error)?;
        sqlx::query(LOCAL_SETTINGS)
            .bind(timeout)
            .execute(&mut *tx)
            .await
            .map_err(map_error)?;
        Ok(tx)
    }

    /// Run the batch statement in its own `READ ONLY` transaction and return its row.
    async fn run_batch(
        &self,
        after: &ChainPosition,
        head: ChainSeq,
        batch: u32,
        bytes: u64,
    ) -> Result<BatchRow, StorageError> {
        // A byte budget above the largest `bigint` cannot be reached by any table.
        let bytes = i64::try_from(bytes).unwrap_or(i64::MAX);
        let after_hash = after.hash.as_ref().map(|hash| hash.as_bytes().as_slice());
        let mut tx = self.begin_local().await?;
        let row = sqlx::query_as::<_, BatchRow>(CHECK_BATCH)
            .bind(after.seq.get())
            .bind(head.get())
            // A batch holds at least one row (`data-model.md` 4.9), so 0 means 1, not `LIMIT 0`.
            .bind(i64::from(batch.max(1)))
            .bind(bytes)
            .bind(after_hash)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_error)?;
        tx.commit().await.map_err(map_error)?;
        Ok(row)
    }
}

/// Build a chain position as the columns hold it.
///
/// The constraints of `audit_chain_head` and `audit_log` permit a non-negative `chain_seq` and a
/// hash of 32 bytes or none. Any other shape is a defect of the stored data (`E-DB-008`).
/// This function does not require a hash after the start. The callers decide that.
fn position(
    table: &'static str,
    seq: i64,
    hash: Option<Vec<u8>>,
) -> Result<ChainPosition, StorageError> {
    if seq < 0 {
        return Err(corrupt(table, "the chain_seq is negative"));
    }
    let hash = hash
        .map(|bytes| <[u8; 32]>::try_from(bytes).map(Sha256::from))
        .transpose()
        .map_err(|_bytes| corrupt(table, "the hash is not 32 bytes long"))?;
    Ok(ChainPosition {
        seq: ChainSeq::from(seq),
        hash,
    })
}

/// Build the position of a stored `audit_log` row.
///
/// `entry_hash` is `NOT NULL`, so a row after the start without a hash is a defect (`E-DB-008`).
fn row_position(seq: i64, hash: Option<Vec<u8>>) -> Result<ChainPosition, StorageError> {
    let position = position(LOG_TABLE, seq, hash)?;
    if position.seq > ChainSeq::ZERO && position.hash.is_none() {
        return Err(corrupt(LOG_TABLE, "a row after the start has no hash"));
    }
    Ok(position)
}

/// Build the bounds from the two columns. Both NULL give `None`. One NULL is a defect.
///
/// A value of 0 or below passes through. The job names it as a break (ADR-0086 (c)).
fn bounds(min: Option<i64>, max: Option<i64>) -> Result<Option<ChainBounds>, StorageError> {
    match (min, max) {
        (Some(min), Some(max)) => Ok(Some(ChainBounds {
            min: ChainSeq::from(min),
            max: ChainSeq::from(max),
        })),
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => {
            Err(corrupt(LOG_TABLE, "only one of the two bounds is NULL"))
        }
    }
}

/// Read the anchor text. A missing row gives zero. A bad form is a defect (`E-DB-008`).
///
/// The error names the table and not the text.
fn anchor(text: Option<String>) -> Result<ChainSeq, StorageError> {
    text.map_or(Ok(ChainSeq::ZERO), |text| {
        parse_chain_seq_text(&text).ok_or_else(|| {
            corrupt(
                SETTING_TABLE,
                "the anchor is not in the form of a chain_seq",
            )
        })
    })
}

/// Turn the head row into the head of the port.
fn head_of(row: HeadRow) -> Result<ChainHead, StorageError> {
    let (seq, hash, min_seq, max_seq, anchor_text) = row;
    Ok(ChainHead {
        position: position(HEAD_TABLE, seq, hash)?,
        bounds: bounds(min_seq, max_seq)?,
        anchor: anchor(anchor_text)?,
    })
}

/// Turn the batch row into the outcome of the port.
fn outcome_of(row: BatchRow) -> Result<BatchOutcome, StorageError> {
    let (first_break, last_clean, row_count, last_seq, last_hash) = row;
    match (first_break, last_clean) {
        (Some(first_break), Some(last_clean)) => {
            return Ok(BatchOutcome::Broken {
                first_break: ChainSeq::from(first_break),
                last_clean: ChainSeq::from(last_clean),
            });
        }
        (Some(_), None) => return Err(corrupt(LOG_TABLE, "a break has no last clean row")),
        (None, _) => {}
    }
    let rows = u64::try_from(row_count).map_err(|error| corrupt(LOG_TABLE, error))?;
    let last = match last_seq {
        Some(seq) => Some(row_position(seq, last_hash)?),
        None if rows == 0 => None,
        None => return Err(corrupt(LOG_TABLE, "a batch with rows has no last row")),
    };
    Ok(BatchOutcome::Clean { rows, last })
}

#[async_trait]
impl AuditChainReads for PgAuditChainReads {
    async fn head(&self) -> Result<ChainHead, StorageError> {
        let mut tx = self.begin_local().await?;
        let row = sqlx::query_as::<_, HeadRow>(READ_HEAD)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_error)?;
        tx.commit().await.map_err(map_error)?;
        head_of(row.ok_or_else(|| missing_row(HEAD_TABLE))?)
    }

    async fn check_batch(
        &self,
        after: &ChainPosition,
        head: ChainSeq,
        batch: u32,
        bytes: u64,
    ) -> Result<BatchOutcome, StorageError> {
        outcome_of(self.run_batch(after, head, batch, bytes).await?)
    }
}

#[cfg(test)]
mod tests;
