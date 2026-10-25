//! Port of the reads of the nightly audit chain check (`data-model.md` section 4.9).

use async_trait::async_trait;
use recipe_domain::export::Sha256;

use crate::error::StorageError;

/// The sequence number `chain_seq` of an `audit_log` row.
///
/// The type solves one problem: a bare integer could stand for a row count or a batch size,
/// and the check carries sequence numbers from one batch to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainSeq(i64);

impl ChainSeq {
    /// The sequence number before the first row.
    pub const ZERO: Self = Self(0);

    /// Return the number of the sequence.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl From<i64> for ChainSeq {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

/// A position in the hash chain: a sequence number and the `entry_hash` of that row.
///
/// The head of the chain and the end of a batch both have this shape.
/// An empty table has the position [`ChainPosition::START`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainPosition {
    /// The sequence number of the row.
    pub seq: ChainSeq,
    /// The `entry_hash` of the row.
    /// It is `None` before the first row, or for a head without a hash.
    pub hash: Option<Sha256>,
}

impl ChainPosition {
    /// The position before the first row: sequence 0 and no hash.
    pub const START: Self = Self {
        seq: ChainSeq::ZERO,
        hash: None,
    };
}

/// The lowest and the highest `chain_seq` that `audit_log` holds (ADR-0086 (a)).
///
/// The type solves one problem: a superuser can insert a row outside the range `(0, last_seq]`
/// with the triggers off, and no batch reads it. The check compares these bounds with the head.
/// The values are the raw values that the table holds, so a value of 0 or below is possible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainBounds {
    /// The lowest `chain_seq` of the table.
    pub min: ChainSeq,
    /// The highest `chain_seq` of the table.
    pub max: ChainSeq,
}

/// What step 1 of the check reads in one short transaction (`data-model.md` section 4.9).
///
/// The type solves one problem: the job must apply the rules of ADR-0086 (c) to one consistent
/// read. The adapter returns the values as stored and applies no rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainHead {
    /// The head of the chain: `last_seq` and `last_hash` of `audit_chain_head`.
    pub position: ChainPosition,
    /// The bounds of `audit_log`, or `None` when the table has no row.
    pub bounds: Option<ChainBounds>,
    /// The highest `chain_seq` that a completed check confirmed (`audit.last_check_seq`).
    /// A missing row gives [`ChainSeq::ZERO`].
    pub anchor: ChainSeq,
}

/// The result of one batch of the check.
///
/// The batch stops at the first break. The job needs the position before that break, because
/// it records the larger value of the anchor and `last_clean` (ADR-0086 (e)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchOutcome {
    /// A row of the batch breaks the chain (`E-AUD-002`).
    Broken {
        /// The `chain_seq` of the first row that breaks the chain.
        first_break: ChainSeq,
        /// The `chain_seq` of the last row before the first break.
        /// When the first row of the batch breaks, it is the `chain_seq` of the `after` position.
        last_clean: ChainSeq,
    },
    /// The batch has no break.
    Clean {
        /// The number of rows that the batch checked.
        rows: u64,
        /// The last row of the batch, or `None` when the batch holds no row.
        last: Option<ChainPosition>,
    },
}

/// Port of the reads of the job `audit_chain_check`.
///
/// The port solves one problem: the check must read the chain in short transactions, so it
/// never delays VACUUM. A test must also run the check without a database. Each method is one
/// short `READ ONLY` transaction.
#[async_trait]
pub trait AuditChainReads: Send + Sync {
    /// Read step 1 of the check: the head, the bounds of `audit_log`, and the anchor.
    ///
    /// The adapter returns the bounds as stored, also a value of 0 or below. It returns the head
    /// as stored, also a head with a positive `last_seq` and no hash. That head is a break of the
    /// chain, and the job reports it. The adapter refuses only a head that the constraints of
    /// `audit_chain_head` do not permit, a bad anchor text, and one NULL bound (`E-DB-008`).
    async fn head(&self) -> Result<ChainHead, StorageError>;

    /// Check one batch of rows with `chain_seq` above `after` and up to `head`.
    ///
    /// The batch ends at `batch` rows or at `bytes` of `before_state` and `after_state`.
    /// When the range holds a row, the batch holds at least one row.
    /// A break gives `first_break` and `last_clean`, the `chain_seq` before that break.
    /// A clean batch does not prove that the rows up to `head` exist. After the last batch, the
    /// caller compares the end with the head (`data-model.md` 4.9).
    async fn check_batch(
        &self,
        after: &ChainPosition,
        head: ChainSeq,
        batch: u32,
        bytes: u64,
    ) -> Result<BatchOutcome, StorageError>;
}
