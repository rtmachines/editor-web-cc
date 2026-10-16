//! The security events of the read model, aggregated per minute (`data-model.md` section 3.8).

use std::collections::HashMap;
use std::net::IpAddr;

use recipe_domain::audit::{SecurityEvent, SecurityEventKind};
use recipe_domain::identity::UserId;
use recipe_domain::timestamp::Timestamp;

use super::entries::position_id;
use crate::error::StorageError;
use crate::memory::MemoryState;
use crate::memory::sink::RecordedEvent;
use crate::ports::{SecurityEventFilter, SecurityEventId, SecurityEventRow};

/// The seconds of one minute.
const MINUTE_SECONDS: i64 = 60;

/// One row of `security_event`: the first event of the row and the number of its events.
pub(super) struct EventRow {
    /// The id of the first event of the row.
    pub(super) id: SecurityEventId,
    /// The time of the first event of the row.
    pub(super) at: Timestamp,
    /// The first event of the row.
    pub(super) event: SecurityEvent,
    /// The number of events of the row.
    pub(super) count: u32,
}

/// The key of an aggregated row: the client address, the kind, and the minute in UTC.
type MinuteKey = (IpAddr, SecurityEventKind, i64);

/// Return the key of `recorded`, or `None` for an event that does not aggregate.
fn minute_key(recorded: &RecordedEvent) -> Result<Option<MinuteKey>, StorageError> {
    let kind = recorded.event.kind();
    let aggregates = matches!(
        kind,
        SecurityEventKind::LoginFailed
            | SecurityEventKind::LoginThrottled
            | SecurityEventKind::CsrfRejected
    );
    let Some(client) = recorded.event.client_ip().filter(|_client| aggregates) else {
        return Ok(None);
    };
    let minute = recorded
        .at
        .as_offset_date_time()
        .unix_timestamp()
        .checked_div_euclid(MINUTE_SECONDS)
        .ok_or_else(|| StorageError::invariant("an event time has no minute"))?;
    Ok(Some((client, kind, minute)))
}

/// Aggregate the recorded events into rows, in the order of the first event of each row.
///
/// The id of each event is its position plus 1. A row keeps the id, the time, and the fields of
/// its first event, as the upsert of `data-model.md` section 3.8 does.
pub(super) fn aggregate(recorded: Vec<RecordedEvent>) -> Result<Vec<EventRow>, StorageError> {
    let mut rows: Vec<EventRow> = Vec::new();
    let mut open: HashMap<MinuteKey, usize> = HashMap::new();
    for (index, recorded) in recorded.into_iter().enumerate() {
        let key = minute_key(&recorded)?;
        if let Some(row) = key
            .and_then(|key| open.get(&key).copied())
            .and_then(|position| rows.get_mut(position))
        {
            row.count = row
                .count
                .checked_add(1)
                .ok_or_else(|| StorageError::invariant("an event count has no successor"))?;
            continue;
        }
        if let Some(key) = key {
            open.insert(key, rows.len());
        }
        rows.push(EventRow {
            id: SecurityEventId::from(position_id(index)?),
            at: recorded.at,
            event: recorded.event,
            count: 1,
        });
    }
    Ok(rows)
}

/// Return the id of the user with the login of the filter `login`.
///
/// `None` means that no account has the login. The HMAC can still match.
fn subject_user(state: &MemoryState, filter: &SecurityEventFilter) -> Option<UserId> {
    let login = filter.login.as_ref()?.login.as_ref()?;
    state
        .users
        .values()
        .find(|row| &row.login == login)
        .map(|row| row.id)
}

/// Return the rows of `rows` that match `filter`, in the same order.
pub(super) fn filtered(
    state: &MemoryState,
    rows: Vec<EventRow>,
    filter: &SecurityEventFilter,
) -> Vec<EventRow> {
    let user = subject_user(state, filter);
    rows.into_iter()
        .filter(|row| {
            let event = &row.event;
            let subject_matches = filter.login.as_ref().is_none_or(|subject| {
                user.is_some_and(|user| event.user() == Some(user))
                    || event.login_hmac() == Some(&subject.hmac)
            });
            filter.kind.is_none_or(|kind| event.kind() == kind)
                && subject_matches
                && filter
                    .client_ip
                    .is_none_or(|client| event.client_ip() == Some(client))
                && filter.range.contains(row.at)
        })
        .collect()
}

/// Build the list row of `row` with the login of its user. The row holds no `login_hmac`.
pub(super) fn event_row(state: &MemoryState, row: EventRow) -> SecurityEventRow {
    let login = row
        .event
        .user()
        .and_then(|user| state.users.get(&user))
        .map(|user| user.login.clone());
    SecurityEventRow {
        id: row.id,
        occurred_at: row.at,
        kind: row.event.kind(),
        count: row.count,
        client_ip: row.event.client_ip(),
        user_agent: row.event.user_agent().cloned(),
        request_id: row.event.request_id(),
        login,
        detail: row.event.detail().cloned(),
    }
}
