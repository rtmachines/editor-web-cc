//! The pure rules of the audit chain check (ADR-0086 (c) to (e)).
//!
//! The functions read values and return a [`Verdict`]. They do no I/O, so the job stays short.

use crate::error::StorageError;
use crate::ports::{ChainHead, ChainPosition, ChainSeq};

/// The `chain_seq` of the first row of a chain.
fn first() -> ChainSeq {
    ChainSeq::from(1)
}

/// A break of the chain: the `chain_seq` that `E-AUD-002` names and the anchor to record.
///
/// The type solves one problem: every kind of break names one `chain_seq` and records one anchor
/// by a fixed rule (ADR-0086 (c) and (e)). The job reports both from one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Verdict {
    /// The `chain_seq` that the error names.
    pub(super) chain_seq: ChainSeq,
    /// The value of `audit.last_check_seq` after the check. It is never below the old anchor.
    pub(super) anchor: ChainSeq,
}

/// Return the sequence number after `seq`.
///
/// # Errors
///
/// Returns a `StorageError` when the number overflows. A real chain never reaches that number.
pub(super) fn next_seq(seq: ChainSeq) -> Result<ChainSeq, StorageError> {
    seq.get()
        .checked_add(1)
        .map(ChainSeq::from)
        .ok_or_else(|| StorageError::invariant("a chain_seq has no successor"))
}

/// Return the sequence number before `seq`.
fn previous_seq(seq: ChainSeq) -> Result<ChainSeq, StorageError> {
    seq.get()
        .checked_sub(1)
        .map(ChainSeq::from)
        .ok_or_else(|| StorageError::invariant("a chain_seq has no predecessor"))
}

/// Apply the five rules of step 1 in order (ADR-0086 (c)).
///
/// The first rule that applies is the break. A break in step 1 leaves the anchor unchanged.
///
/// # Errors
///
/// Returns a `StorageError` when a name of a break overflows.
pub(super) fn step_one(head: &ChainHead) -> Result<Option<Verdict>, StorageError> {
    let anchor = head.anchor;
    Ok(first_rule(head)?.map(|chain_seq| Verdict { chain_seq, anchor }))
}

/// Return the `chain_seq` that the first rule of step 1 names, or `None`.
fn first_rule(head: &ChainHead) -> Result<Option<ChainSeq>, StorageError> {
    let last = head.position.seq;
    let first = first();
    if let Some(bounds) = head.bounds {
        if bounds.min < first {
            return Ok(Some(bounds.min));
        }
        if bounds.max > last {
            return Ok(Some(bounds.max));
        }
    }
    if last < head.anchor {
        return next_seq(last).map(Some);
    }
    if last > ChainSeq::ZERO && head.bounds.is_none_or(|bounds| bounds.min > first) {
        return Ok(Some(first));
    }
    match head.bounds {
        Some(bounds) if bounds.max < last => next_seq(bounds.max).map(Some),
        _ => Ok(None),
    }
}

/// Return the verdict for a break inside a batch (ADR-0086 (d) and (e)).
///
/// The anchor is the larger value of the old anchor and `last_clean` of the batch.
pub(super) fn batch_break(
    first_break: ChainSeq,
    last_clean: ChainSeq,
    anchor: ChainSeq,
) -> Verdict {
    Verdict {
        chain_seq: first_break,
        anchor: anchor.max(last_clean),
    }
}

/// Compare the end of the check with the head of step 1 (ADR-0086 (d)).
///
/// `end` is the last position that the batches checked, or [`ChainPosition::START`].
/// The caller guarantees that `end` is not above the head.
///
/// # Errors
///
/// Returns a `StorageError` when a name of a break overflows.
pub(super) fn end_check(
    end: &ChainPosition,
    head: &ChainPosition,
    anchor: ChainSeq,
) -> Result<Option<Verdict>, StorageError> {
    if end.seq < head.seq {
        return Ok(Some(Verdict {
            chain_seq: next_seq(end.seq)?,
            anchor: anchor.max(end.seq),
        }));
    }
    if end.hash != head.hash {
        return Ok(Some(Verdict {
            chain_seq: head.seq,
            anchor: anchor.max(previous_seq(head.seq)?),
        }));
    }
    Ok(None)
}
