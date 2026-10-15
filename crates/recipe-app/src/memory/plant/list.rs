//! The list of machines of the in-memory `PlantRepository` (design D4).
//!
//! The statement of `recipe-db` is the model. The filter folds case with `to_lowercase` on both
//! sides. The rows come in the same order as the `ORDER BY` of that statement.

use std::cmp::Ordering;

use recipe_domain::timestamp::Timestamp;

use crate::memory::MemoryState;
use crate::memory::rows::MachineRow;
use crate::ports::{MachineFilter, MachineSummary};

/// Return one page of the machines that match `filter`.
///
/// A row matches when its code or its name contains the search text.
/// Both sides fold to lower case first.
pub(super) fn list(
    state: &MemoryState,
    filter: &MachineFilter,
    offset: u64,
    limit: u32,
) -> Vec<MachineSummary> {
    let needle = filter
        .text
        .as_ref()
        .map(|text| text.as_str().to_lowercase());
    let mut rows: Vec<&MachineRow> = state
        .machines
        .values()
        .filter(|row| filter.include_archived || row.archived_at.is_none())
        .filter(|row| needle.as_deref().is_none_or(|text| contains(row, text)))
        .collect();
    rows.sort_by(|left, right| order(left, right));
    // A count that does not fit `usize` is larger than any list, so it keeps its meaning.
    let skip = usize::try_from(offset).unwrap_or(usize::MAX);
    let take = usize::try_from(limit).unwrap_or(usize::MAX);
    rows.into_iter()
        .skip(skip)
        .take(take)
        .map(|row| MachineSummary {
            id: row.id,
            code: row.code,
            name: row.name.clone(),
            platform: row.platform,
            archived_at: row.archived_at,
        })
        .collect()
}

/// Tell whether the code or the name of the row contains `needle`, which is in lower case.
fn contains(row: &MachineRow, needle: &str) -> bool {
    row.code.to_string().to_lowercase().contains(needle)
        || row.name.as_str().to_lowercase().contains(needle)
}

/// The order of the list: the number of the code, then `archived_at DESC NULLS FIRST`, then `id`.
fn order(left: &MachineRow, right: &MachineRow) -> Ordering {
    left.code
        .number()
        .cmp(&right.code.number())
        .then_with(|| archived_order(left.archived_at, right.archived_at))
        .then_with(|| left.id.cmp(&right.id))
}

/// Put the live machine first and then the archived machines, the latest archiving first.
fn archived_order(left: Option<Timestamp>, right: Option<Timestamp>) -> Ordering {
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(left), Some(right)) => right.cmp(&left),
    }
}
