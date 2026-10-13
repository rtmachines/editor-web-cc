//! List check of process limit rows (design D9, ADR-0007).

use std::collections::BTreeSet;
use std::fmt;

use crate::plant::{LeafPath, LeafSegment};

/// One row of a limit request. The bound texts stay unparsed until a type tree is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitInput {
    /// The leaf that the row limits.
    pub leaf_path: LeafPath,
    /// The lower bound as text, or `None` when the row has no lower bound.
    pub min: Option<String>,
    /// The upper bound as text, or `None` when the row has no upper bound.
    pub max: Option<String>,
    /// The largest length of a string leaf, or `None` when the row has no length limit.
    pub max_length: Option<u16>,
}

/// The rows of one limit request that pass the list check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitList(Vec<LimitInput>);

/// Reason why a list of limit rows fails the list check.
///
/// The enum has no code. `recipe-web` wraps it into `E-WEB-008` (ADR-0025).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitListReason {
    /// Two rows name the same leaf path.
    DuplicatePath,
    /// A row with `[*]` and a row with an index name one array level.
    StarAndIndex,
    /// A row has no bound and no length limit.
    EmptyRow,
}

impl fmt::Display for LimitListReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePath => f.write_str("two limit rows name the same leaf path"),
            Self::StarAndIndex => f.write_str(
                "a limit row with all elements and a row with an index name one array level",
            ),
            Self::EmptyRow => f.write_str("a limit row has no bound and no length limit"),
        }
    }
}

impl LimitList {
    /// Return the rows, in the order of the request.
    #[must_use]
    pub fn rows(&self) -> &[LimitInput] {
        &self.0
    }

    /// Consume the list and return its rows.
    #[must_use]
    pub fn into_rows(self) -> Vec<LimitInput> {
        self.0
    }

    /// Check the rows of one limit request and wrap them into a `LimitList`.
    ///
    /// The checks run in this order: empty row, duplicate path, `[*]` with an index.
    ///
    /// # Errors
    ///
    /// Returns [`LimitListReason::EmptyRow`] when a row has no bound and no length limit.
    /// Returns [`LimitListReason::DuplicatePath`] when two rows name one leaf path.
    /// Returns [`LimitListReason::StarAndIndex`] when `[*]` and an index name one array level.
    pub fn parse(rows: Vec<LimitInput>) -> Result<Self, LimitListReason> {
        if rows.iter().any(is_empty_row) {
            return Err(LimitListReason::EmptyRow);
        }
        check_duplicate_paths(&rows)?;
        check_star_and_index(&rows)?;
        Ok(Self(rows))
    }
}

/// Return `true` when a row has no bound and no length limit.
fn is_empty_row(row: &LimitInput) -> bool {
    row.min.is_none() && row.max.is_none() && row.max_length.is_none()
}

/// Reject a list where two rows name one leaf path.
fn check_duplicate_paths(rows: &[LimitInput]) -> Result<(), LimitListReason> {
    let mut seen = BTreeSet::new();
    if rows.iter().all(|row| seen.insert(&row.leaf_path)) {
        Ok(())
    } else {
        Err(LimitListReason::DuplicatePath)
    }
}

/// Reject a list where `[*]` and an index name one array level (ADR-0007).
///
/// The prefixes before every `[*]` go into one set. A row with an index is a conflict
/// when its prefix before that index is in the set. The prefix length gives the position.
fn check_star_and_index(rows: &[LimitInput]) -> Result<(), LimitListReason> {
    let star_prefixes: BTreeSet<Vec<LeafSegment>> = rows
        .iter()
        .flat_map(|row| prefixes_at(row.leaf_path.segments(), is_all))
        .collect();
    let conflict = rows.iter().any(|row| {
        prefixes_at(row.leaf_path.segments(), is_index)
            .any(|prefix| star_prefixes.contains(&prefix))
    });
    if conflict {
        Err(LimitListReason::StarAndIndex)
    } else {
        Ok(())
    }
}

/// Return the prefix before each segment that `is_at` accepts.
fn prefixes_at(
    segments: &[LeafSegment],
    is_at: fn(&LeafSegment) -> bool,
) -> impl Iterator<Item = Vec<LeafSegment>> + '_ {
    segments
        .iter()
        .enumerate()
        .filter(move |(_, segment)| is_at(segment))
        .map(move |(position, _)| segments.iter().take(position).cloned().collect())
}

/// Return `true` for the segment `[*]`.
fn is_all(segment: &LeafSegment) -> bool {
    matches!(segment, LeafSegment::All)
}

/// Return `true` for an index segment.
fn is_index(segment: &LeafSegment) -> bool {
    matches!(segment, LeafSegment::Index(_))
}

#[cfg(test)]
mod tests {
    use super::{LimitInput, LimitList, LimitListReason};
    use crate::plant::LeafPath;

    fn row(path: &str) -> LimitInput {
        LimitInput {
            leaf_path: LeafPath::parse(path).unwrap(),
            min: None,
            max: Some("10".to_owned()),
            max_length: None,
        }
    }

    fn empty_row(path: &str) -> LimitInput {
        LimitInput {
            leaf_path: LeafPath::parse(path).unwrap(),
            min: None,
            max: None,
            max_length: None,
        }
    }

    #[test]
    fn two_rows_for_one_leaf_path_are_a_duplicate() {
        let rows = vec![row(".setpoint"), row(".setpoint")];
        assert_eq!(LimitList::parse(rows), Err(LimitListReason::DuplicatePath));
    }

    #[test]
    fn all_elements_and_one_element_of_one_level_are_rejected() {
        let rows = vec![row(".ramps[*]"), row(".ramps[2]")];
        assert_eq!(LimitList::parse(rows), Err(LimitListReason::StarAndIndex));
    }

    #[test]
    fn index_row_before_all_elements_row_is_rejected() {
        let rows = vec![row(".ramps[2]"), row(".ramps[*]")];
        assert_eq!(LimitList::parse(rows), Err(LimitListReason::StarAndIndex));
    }

    #[test]
    fn all_elements_and_index_below_a_member_are_rejected() {
        let rows = vec![row(".zones[*].a"), row(".zones[2].b")];
        assert_eq!(LimitList::parse(rows), Err(LimitListReason::StarAndIndex));
    }

    #[test]
    fn different_prefixes_with_all_and_index_are_allowed() {
        let rows = vec![row(".a[*]"), row(".b[2]")];
        assert!(LimitList::parse(rows).is_ok());
    }

    #[test]
    fn two_all_elements_rows_at_one_level_are_allowed() {
        let rows = vec![row(".zones[*].a"), row(".zones[*].b")];
        assert!(LimitList::parse(rows).is_ok());
    }

    #[test]
    fn row_without_a_field_is_empty() {
        let rows = vec![empty_row(".setpoint")];
        assert_eq!(LimitList::parse(rows), Err(LimitListReason::EmptyRow));
    }

    #[test]
    fn empty_row_is_checked_before_duplicate_path() {
        let rows = vec![row(".setpoint"), row(".setpoint"), empty_row(".other")];
        assert_eq!(LimitList::parse(rows), Err(LimitListReason::EmptyRow));
    }

    #[test]
    fn empty_list_is_valid() {
        let list = LimitList::parse(Vec::new()).unwrap();
        assert!(list.rows().is_empty());
    }

    #[test]
    fn all_elements_at_root_with_index_at_root_is_rejected() {
        let rows = vec![row("[*]"), row("[1,2]")];
        assert_eq!(LimitList::parse(rows), Err(LimitListReason::StarAndIndex));
    }

    #[test]
    fn valid_list_keeps_its_rows_in_order() {
        let rows = vec![row(".a"), row(".b")];
        let list: LimitList = LimitList::parse(rows.clone()).unwrap();
        assert_eq!(list.rows(), rows.as_slice());
        assert_eq!(list.into_rows(), rows);
    }
}
