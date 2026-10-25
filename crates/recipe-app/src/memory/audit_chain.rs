//! In-memory `AuditChainReads` (task 3.10, design D6).
//!
//! The adapter keeps scripted rows with fake hashes. A row knows the hash that a correct
//! recomputation gives, so a test can break one row and see the first break.
//!
//! The adapter gives the results of the SQL adapter (ADR-0086 (g)). It reads the rows in
//! `chain_seq` order. It refuses a second row with a taken `chain_seq`. It treats a gap in
//! `chain_seq` as a break. It permits a head with a `last_seq` above 0 and no hash.

use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::export::Sha256;
use recipe_domain::setting::SettingKey;

use super::call_faults::CallFaults;
use super::{Fault, MemoryState, MemoryStore};
use crate::error::StorageError;
use crate::ports::{
    AuditChainReads, BatchOutcome, ChainBounds, ChainHead, ChainPosition, ChainSeq, Isolation,
    StateValue, parse_chain_seq_text,
};

/// The last byte of the hash that a correct recomputation gives.
const SOUND_SALT: u8 = 0;
/// The last byte of the hash of a broken row.
const BROKEN_SALT: u8 = 1;

/// A method of [`AuditChainReads`]. A fault and a call counter belong to one method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuditChainMethod {
    /// The method `head`.
    Head,
    /// The method `check_batch`.
    CheckBatch,
}

impl AuditChainMethod {
    /// Every method of the port.
    pub const ALL: [Self; 2] = [Self::Head, Self::CheckBatch];
}

/// The way that a test breaks one row of the chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainBreak {
    /// The stored `entry_hash` differs from the hash that the recomputation gives.
    WrongEntryHash,
    /// The stored `previous_hash` differs from the `entry_hash` of the row before.
    WrongPreviousHash,
}

/// One row of `audit_log` as the check sees it.
///
/// The type solves one problem: the check compares the stored hashes with the recomputed
/// hashes, and the adapter has no real entries to hash. The row holds both sides.
#[derive(Clone, Debug)]
pub(crate) struct ChainRow {
    /// The `chain_seq` of the row.
    seq: ChainSeq,
    /// The stored `previous_hash`, or `None` for the first row.
    previous_hash: Option<Sha256>,
    /// The stored `entry_hash`.
    entry_hash: Sha256,
    /// The `entry_hash` that a correct recomputation gives.
    sound_hash: Sha256,
    /// The bytes of `before_state` and `after_state` together.
    bytes: u64,
    /// True for a row that a test planted outside the chain. It never counts as the last row.
    planted: bool,
}

impl ChainRow {
    /// Return the position of this row in the chain.
    fn position(&self) -> ChainPosition {
        ChainPosition {
            seq: self.seq,
            hash: Some(self.entry_hash),
        }
    }

    /// Return true when the row follows `previous` without a gap and both stored hashes are right.
    ///
    /// `previous_seq` is the `chain_seq` of the row before, or the `after` value for the first row.
    /// A `chain_seq` that does not equal `previous_seq` + 1 is a break, as in the batch statement.
    fn is_intact(&self, previous: Option<Sha256>, previous_seq: ChainSeq) -> bool {
        let next_seq = previous_seq.get().checked_add(1);
        next_seq == Some(self.seq.get())
            && self.previous_hash == previous
            && self.entry_hash == self.sound_hash
    }
}

/// Build a fake hash from a sequence number. Another `salt` gives another hash.
fn fake_hash(seq: ChainSeq, salt: u8) -> Sha256 {
    let mut bytes = [0_u8; 32];
    for (slot, byte) in bytes.iter_mut().zip(seq.get().to_be_bytes()) {
        *slot = byte;
    }
    if let Some(last) = bytes.last_mut() {
        *last = salt;
    }
    Sha256::from(bytes)
}

/// In-memory adapter of [`AuditChainReads`] on a [`MemoryStore`].
///
/// The type solves one problem: the tests of the job `audit_chain_check` need a chain that they
/// can extend, break, and read in batches, without a database. Each call reads in one `READ ONLY`
/// transaction. A fault fires before the read. A clone shares the faults and the store.
#[derive(Clone, Debug)]
pub struct MemoryAuditChain {
    /// The store that holds the rows.
    store: MemoryStore,
    /// The planned faults and the call counters of the methods.
    faults: Arc<Mutex<CallFaults<AuditChainMethod, Fault>>>,
}

impl MemoryAuditChain {
    /// Build an adapter that reads the chain of `store`. The chain starts empty.
    #[must_use]
    pub fn new(store: MemoryStore) -> Self {
        Self {
            store,
            faults: Arc::default(),
        }
    }

    /// Append one correct row of `bytes` bytes and return its position.
    ///
    /// The first row has `chain_seq` 1 and no `previous_hash`.
    ///
    /// A planted row can hold the next `chain_seq`. The unique index refuses a second row with
    /// that number, so the call adds no row. It returns the position that the row would have had,
    /// and the head does not move.
    pub fn append_row(&self, bytes: u64) -> ChainPosition {
        self.store.update_state(|state, _now| {
            let last = last_chain_row(&state.chain);
            let seq = ChainSeq::from(last.map_or(0, |row| row.seq.get()).saturating_add(1));
            let entry_hash = fake_hash(seq, SOUND_SALT);
            let row = ChainRow {
                seq,
                previous_hash: last.map(|row| row.entry_hash),
                entry_hash,
                sound_hash: entry_hash,
                bytes,
                planted: false,
            };
            let position = row.position();
            if state.chain.iter().all(|existing| existing.seq != seq) {
                state.chain.push(row);
            }
            position
        })
    }

    /// Plant a row with `chain_seq` `seq` outside the chain. The head does not move.
    ///
    /// The row stands for a superuser who inserts a row with the triggers off (ADR-0086 (g)).
    /// The row has no `previous_hash`. The unique index of `chain_seq` refuses a second row with
    /// the same number, so the call returns false and adds no row in that case.
    pub fn plant_row(&self, seq: ChainSeq, bytes: u64) -> bool {
        self.plant(seq, bytes, false)
    }

    /// Plant a row like [`plant_row`](Self::plant_row), with one difference.
    ///
    /// The `previous_hash` is the `entry_hash` of the row with the highest `chain_seq` below
    /// `seq`. Only a gap in `chain_seq` can then break the row.
    pub fn plant_linked_row(&self, seq: ChainSeq, bytes: u64) -> bool {
        self.plant(seq, bytes, true)
    }

    /// Add the planted row. Return false when a row with `chain_seq` `seq` exists.
    fn plant(&self, seq: ChainSeq, bytes: u64, linked: bool) -> bool {
        self.store.update_state(|state, _now| {
            if state.chain.iter().any(|row| row.seq == seq) {
                return false;
            }
            let previous_hash = linked
                .then(|| {
                    state
                        .chain
                        .iter()
                        .filter(|row| row.seq < seq)
                        .max_by_key(|row| row.seq)
                        .map(|row| row.entry_hash)
                })
                .flatten();
            let entry_hash = fake_hash(seq, SOUND_SALT);
            state.chain.push(ChainRow {
                seq,
                previous_hash,
                entry_hash,
                sound_hash: entry_hash,
                bytes,
                planted: true,
            });
            true
        })
    }

    /// Break the row with `chain_seq` `seq` in the way of `kind`.
    ///
    /// Returns false when no such row exists.
    pub fn break_row(&self, seq: ChainSeq, kind: ChainBreak) -> bool {
        self.store.update_state(|state, _now| {
            let Some(row) = state.chain.iter_mut().find(|row| row.seq == seq) else {
                return false;
            };
            match kind {
                ChainBreak::WrongEntryHash => row.entry_hash = fake_hash(seq, BROKEN_SALT),
                ChainBreak::WrongPreviousHash => {
                    row.previous_hash = Some(fake_hash(seq, BROKEN_SALT));
                }
            }
            true
        })
    }

    /// Plant a head that differs from the last row, or restore the last row with `None`.
    pub fn set_head(&self, head: Option<ChainPosition>) {
        self.store
            .update_state(|state, _now| state.chain_head = head);
    }

    /// Make call number `call` of `method` fail with `fault`. Later calls succeed.
    pub fn fail_on_call(&self, method: AuditChainMethod, call: u64, fault: Fault) {
        self.faults.lock().fail_on_call(method, call, fault);
    }

    /// Make call number `call` of `method` and every later call fail with `fault`.
    pub fn fail_from_call(&self, method: AuditChainMethod, call: u64, fault: Fault) {
        self.faults.lock().fail_from_call(method, call, fault);
    }

    /// Stop every planned fault. The call counters keep their values.
    pub fn clear_faults(&self) {
        self.faults.lock().clear();
    }

    /// Return the number of calls that `method` received.
    #[must_use]
    pub fn calls(&self, method: AuditChainMethod) -> u64 {
        self.faults.lock().calls(method)
    }

    /// Count one call of `method` and return its planned fault as an error.
    fn begin_call(&self, method: AuditChainMethod) -> Result<(), StorageError> {
        match self.faults.lock().next(method) {
            Some(fault) => Err(fault.into()),
            None => Ok(()),
        }
    }
}

/// Return the last row of the chain. A planted row is not part of the chain.
fn last_chain_row(chain: &[ChainRow]) -> Option<&ChainRow> {
    chain.iter().rev().find(|row| !row.planted)
}

/// Return the lowest and the highest `chain_seq` of all rows, or `None` for an empty table.
fn bounds_of(chain: &[ChainRow]) -> Option<ChainBounds> {
    let min = chain.iter().map(|row| row.seq).min()?;
    let max = chain.iter().map(|row| row.seq).max()?;
    Some(ChainBounds { min, max })
}

/// Read the anchor from the state key `audit.last_check_seq`. A missing row gives zero.
///
/// A text that breaks the form of ADR-0073 (i) gives a `StorageError`, as the adapter of
/// `recipe-db` does. The error holds no part of the text.
fn read_anchor(state: &MemoryState) -> Result<ChainSeq, StorageError> {
    let key = SettingKey::parse(StateValue::AuditLastCheckSeq(ChainSeq::ZERO).key())
        .map_err(|_| StorageError::invariant("a state key is not a setting key"))?;
    state.settings.get(&key).map_or(Ok(ChainSeq::ZERO), |row| {
        parse_chain_seq_text(row.value.as_str()).ok_or_else(|| {
            StorageError::invariant("a stored state value is not in the text form of ADR-0073 (i)")
        })
    })
}

/// Read step 1 of the check from the state.
fn read_head(state: &MemoryState) -> Result<ChainHead, StorageError> {
    let position = state
        .chain_head
        .or_else(|| last_chain_row(&state.chain).map(ChainRow::position))
        .unwrap_or(ChainPosition::START);
    Ok(ChainHead {
        position,
        bounds: bounds_of(&state.chain),
        anchor: read_anchor(state)?,
    })
}

/// The limits of one batch.
struct BatchLimits {
    /// The most rows of the batch. A value of 0 still allows one row.
    rows: u64,
    /// The most bytes of the batch. The first row may exceed it.
    bytes: u64,
}

/// Check the rows after `after` up to `head` and stop at the limits.
fn check_rows(
    chain: &[ChainRow],
    after: &ChainPosition,
    head: ChainSeq,
    limits: &BatchLimits,
) -> BatchOutcome {
    let max_rows = limits.rows.max(1);
    let mut previous = after.hash;
    let mut previous_seq = after.seq;
    let mut rows = 0_u64;
    let mut used_bytes = 0_u64;
    let mut last = None;
    let mut range: Vec<&ChainRow> = chain
        .iter()
        .filter(|row| row.seq > after.seq && row.seq <= head)
        .collect();
    range.sort_by_key(|row| row.seq);
    for row in range {
        let next_bytes = used_bytes.saturating_add(row.bytes);
        if rows >= max_rows || (rows > 0 && next_bytes > limits.bytes) {
            break;
        }
        if !row.is_intact(previous, previous_seq) {
            return BatchOutcome::Broken {
                first_break: row.seq,
                last_clean: previous_seq,
            };
        }
        used_bytes = next_bytes;
        rows = rows.saturating_add(1);
        previous = Some(row.entry_hash);
        previous_seq = row.seq;
        last = Some(row.position());
    }
    BatchOutcome::Clean { rows, last }
}

#[async_trait]
impl AuditChainReads for MemoryAuditChain {
    async fn head(&self) -> Result<ChainHead, StorageError> {
        self.begin_call(AuditChainMethod::Head)?;
        self.store
            .pool_transaction(Isolation::ReadOnly, |state, _now, _journal| {
                read_head(state)
            })?
    }

    async fn check_batch(
        &self,
        after: &ChainPosition,
        head: ChainSeq,
        batch: u32,
        bytes: u64,
    ) -> Result<BatchOutcome, StorageError> {
        self.begin_call(AuditChainMethod::CheckBatch)?;
        let limits = BatchLimits {
            rows: u64::from(batch),
            bytes,
        };
        self.store
            .pool_transaction(Isolation::ReadOnly, |state, _now, _journal| {
                check_rows(&state.chain, after, head, &limits)
            })
    }
}

#[cfg(test)]
mod tests;
