//! The audit entries of the read model: ids in commit order and commit times (ADR-0051 (a)).

use std::num::NonZeroU64;

use recipe_domain::audit::AuditEntry;
use recipe_domain::identity::UserId;
use recipe_domain::timestamp::Timestamp;

use super::labels::label;
use crate::error::StorageError;
use crate::memory::MemoryState;
use crate::ports::{
    AggregateId, AuditEntityType, AuditEntryId, AuditFilter, AuditRecord, AuditRow, AuditScope,
    EntityId,
};

/// One committed entry with its id and its commit time.
pub(super) struct Numbered {
    /// The position of the entry in commit order plus 1.
    pub(super) id: AuditEntryId,
    /// The commit time of the entry.
    pub(super) at: Timestamp,
    /// The entry.
    pub(super) entry: AuditEntry,
}

/// Return the id of the item at `index` of a list: the position plus 1.
pub(super) fn position_id(index: usize) -> Result<NonZeroU64, StorageError> {
    u64::try_from(index)
        .ok()
        .and_then(|position| position.checked_add(1))
        .and_then(NonZeroU64::new)
        .ok_or_else(|| StorageError::invariant("the position of a row has no id"))
}

/// Give each committed entry its id, in commit order.
pub(super) fn numbered(
    committed: Vec<(Timestamp, AuditEntry)>,
) -> Result<Vec<Numbered>, StorageError> {
    committed
        .into_iter()
        .enumerate()
        .map(|(index, (at, entry))| {
            Ok(Numbered {
                id: AuditEntryId::from(position_id(index)?),
                at,
                entry,
            })
        })
        .collect()
}

/// Return the number of rows of a window: one row more than the page.
pub(super) fn window(limit: u32) -> Result<usize, StorageError> {
    usize::try_from(limit)
        .ok()
        .and_then(|size| size.checked_add(1))
        .ok_or_else(|| StorageError::invariant("the page size has no successor"))
}

/// Return the id of the user with the login of `filter.actor`.
///
/// The outer `None` means that the filter names no user, so no entry matches (A42).
pub(super) fn actor_of(state: &MemoryState, filter: &AuditFilter) -> Option<Option<UserId>> {
    match &filter.actor {
        None => Some(None),
        Some(login) => state
            .users
            .values()
            .find(|row| &row.login == login)
            .map(|row| Some(row.id)),
    }
}

/// Return whether `numbered` is inside `scope` and matches `filter` and the resolved `actor`.
pub(super) fn matches(
    numbered: &Numbered,
    filter: &AuditFilter,
    scope: AuditScope,
    actor: Option<UserId>,
) -> bool {
    let entry = &numbered.entry;
    let target = entry.target();
    let entity_matches = filter.entity.is_none_or(|entity| {
        entity.entity_type.as_sql() == target.entity_type()
            && entity
                .entity_id
                .is_none_or(|id| id.as_uuid() == target.entity_id())
    });
    scope.admits(target.aggregate_type())
        && filter
            .aggregate
            .is_none_or(|aggregate| aggregate.as_uuid() == target.aggregate_id())
        && entity_matches
        && actor.is_none_or(|user| entry.actor() == Some(user))
        && filter.action.is_none_or(|action| entry.action() == action)
        && filter.range.contains(numbered.at)
}

/// Build the list row of `numbered` with the actor login and the label of the current rows.
pub(super) fn row(state: &MemoryState, numbered: &Numbered) -> Result<AuditRow, StorageError> {
    let entry = &numbered.entry;
    let target = entry.target();
    let entity_type = AuditEntityType::from_sql(target.entity_type())
        .ok_or_else(|| StorageError::invariant("an audit entry has an unknown entity type"))?;
    let entity_id = EntityId::from(*target.entity_id());
    Ok(AuditRow {
        id: numbered.id,
        occurred_at: numbered.at,
        actor: entry
            .actor()
            .and_then(|user| state.users.get(&user))
            .map(|user| user.login.clone()),
        action: entry.action(),
        entity_type,
        entity_id,
        aggregate_type: target.aggregate_type(),
        aggregate_id: AggregateId::from(*target.aggregate_id()),
        label: label(state, entity_type, entity_id),
    })
}

/// Build the record of `numbered` with its states and its trace fields.
///
/// The chain position equals the id, because the in-memory log has no gaps.
pub(super) fn record(
    state: &MemoryState,
    numbered: &Numbered,
) -> Result<AuditRecord, StorageError> {
    let entry = &numbered.entry;
    let chain_seq = i64::try_from(numbered.id.get())
        .map_err(|_overflow| StorageError::invariant("an audit id is above i64::MAX"))?;
    Ok(AuditRecord {
        row: row(state, numbered)?,
        request_id: entry.request_id(),
        before: entry.before().cloned(),
        after: entry.after().cloned(),
        chain_seq,
        client_ip: entry.client_ip(),
        session_ref: entry.session_ref().cloned(),
    })
}
