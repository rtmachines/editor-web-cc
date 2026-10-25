//! Time helpers of the memory adapters.
//!
//! The text form of the state keys lives in the port module `ports::state_key` (ADR-0073 (i)).

use std::time::Duration;

use recipe_domain::timestamp::Timestamp;

/// Return `instant - span`. The result stops at the smallest instant on overflow.
pub(crate) fn earlier(instant: Timestamp, span: Duration) -> Timestamp {
    let span = time::Duration::try_from(span).unwrap_or(time::Duration::MAX);
    Timestamp::from(instant.as_offset_date_time().saturating_sub(span))
}

#[cfg(test)]
mod tests;
