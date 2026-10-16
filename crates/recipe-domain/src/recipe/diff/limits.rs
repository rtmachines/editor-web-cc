//! The diff of the limit rows of one parameter (ADR-0022, ADR-0050 (c)).

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;

use super::rows::{DiffBounds, DiffPart};
use crate::plant::{LeafPath, TypeTree, TypeTrees};
use crate::recipe::limit::bound_text;
use crate::recipe::{Parameter, StoredLimit};

/// The limit rows of one side with the tree that sets the form of a time bound.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct LimitSide<'a> {
    limits: &'a [StoredLimit],
    tree: Option<&'a TypeTree>,
}

impl<'a> LimitSide<'a> {
    /// Return the limit rows of `parameter` with the tree of its type.
    pub(super) fn of(parameter: &'a Parameter, trees: &'a TypeTrees) -> Self {
        Self {
            limits: parameter.limits(),
            tree: trees.tree(parameter.plc_type()),
        }
    }

    /// Return the tree of the side, or `None` when the trees do not hold the type.
    pub(super) const fn tree(&self) -> Option<&'a TypeTree> {
        self.tree
    }
}

/// Write the bounds of a stored limit row as text.
///
/// The leaf of the row in `tree` sets the form of a time bound. Without a tree, or when the
/// tree has no such leaf, every bound is canonical decimal text.
pub(super) fn bounds(limit: &StoredLimit, tree: Option<&TypeTree>) -> DiffBounds {
    let primitive = tree.and_then(|tree| {
        let target = limit.leaf_path.resolve(tree)?;
        Some((target.primitive(), tree.platform()))
    });
    let text = |bound: Decimal| match primitive {
        Some((primitive, platform)) => bound_text(primitive, platform, bound.normalize()),
        None => bound.normalize().to_string(),
    };
    DiffBounds {
        min: limit.min.map(text),
        max: limit.max.map(text),
        max_length: limit.max_length.map(|length| length.to_string()),
    }
}

/// Return one row for each limit row that differs, matched by the stored leaf path.
///
/// A `[*]` row is one row. A change of the limit mode gives rows with an empty other side. The
/// rows compare by `min`, `max`, and `max_length`. With `every_row`, each row of both sides
/// gives a row, also an equal row. A type change sets `every_row` (ADR-0050 (c)).
///
/// The rows come in the preorder of the leaves (ADR-0050 (d)). The sort key of a row is the
/// preorder position of the first leaf that the row covers. A row of the new side takes its
/// position in the new tree. A row of the old side only takes its position in the old tree.
/// At the same position, the row that covers more leaves comes first, so `[*]` comes before an
/// element (ADR-0039 (k)). The leaf path decides the rest. A row without a position comes last.
///
/// The function numbers the leaves of a tree only for a side with a row in the result. It does
/// not sort a result with fewer than two rows.
pub(super) fn limit_rows(old: LimitSide<'_>, new: LimitSide<'_>, every_row: bool) -> Vec<DiffPart> {
    let rows = unsorted_rows(old, new, every_row);
    if rows.len() < 2 {
        return rows.into_iter().map(|(_, _, part)| part).collect();
    }
    let has_side = |wanted: Side| rows.iter().any(|(side, _, _)| *side == wanted);
    let new_order = has_side(Side::New).then(|| Preorder::of(new.tree));
    let old_order = has_side(Side::Old).then(|| Preorder::of(old.tree));
    let mut keyed: Vec<(RowOrder, DiffPart)> = rows
        .into_iter()
        .map(|(side, path, part)| {
            let order = match side {
                Side::New => new_order.as_ref(),
                Side::Old => old_order.as_ref(),
            };
            let key = order.map_or_else(|| RowOrder::new(None, path), |order| order.key(path));
            (key, part)
        })
        .collect();
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    keyed.into_iter().map(|(_, part)| part).collect()
}

/// The side whose tree gives the position of a limit row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// A row of the old side only.
    Old,
    /// A row of the new side, with its old match or none.
    New,
}

/// Return the limit rows of [`limit_rows`] in input order, each with its side and its path.
///
/// The rows of the new side come first. The rows of the old side only follow.
fn unsorted_rows<'a>(
    old: LimitSide<'a>,
    new: LimitSide<'a>,
    every_row: bool,
) -> Vec<(Side, &'a LeafPath, DiffPart)> {
    let old_by_path: HashMap<&LeafPath, &StoredLimit> = old
        .limits
        .iter()
        .map(|limit| (&limit.leaf_path, limit))
        .collect();
    let new_paths: HashSet<&LeafPath> = new.limits.iter().map(|limit| &limit.leaf_path).collect();
    let changed = new.limits.iter().filter_map(|limit| {
        let before = old_by_path.get(&limit.leaf_path).copied();
        (every_row || before != Some(limit)).then(|| {
            let part = DiffPart::Limit {
                path: limit.leaf_path.clone(),
                old: before.map(|before| bounds(before, old.tree)),
                new: Some(bounds(limit, new.tree)),
            };
            (Side::New, &limit.leaf_path, part)
        })
    });
    let only_old = old
        .limits
        .iter()
        .filter(|limit| !new_paths.contains(&limit.leaf_path))
        .map(|limit| {
            let part = DiffPart::Limit {
                path: limit.leaf_path.clone(),
                old: Some(bounds(limit, old.tree)),
                new: None,
            };
            (Side::Old, &limit.leaf_path, part)
        });
    changed.chain(only_old).collect()
}

/// The sort key of one limit row. The derived order compares the fields from top to bottom.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RowOrder {
    /// `true` when the tree of the side has no leaf for the row.
    is_unplaced: bool,
    /// The preorder position of the first leaf that the row covers.
    position: usize,
    /// The number of covered leaves, reversed so that a wider row comes first.
    width: Reverse<usize>,
    /// The stored leaf path.
    path: LeafPath,
}

impl RowOrder {
    /// Build the key from the position and the width of a row, or from `None` for no position.
    fn new(place: Option<(usize, usize)>, path: &LeafPath) -> Self {
        let (position, width) = place.unwrap_or_default();
        Self {
            is_unplaced: place.is_none(),
            position,
            width: Reverse(width),
            path: path.clone(),
        }
    }
}

/// The preorder position of each concrete leaf of one tree.
struct Preorder<'a> {
    tree: Option<&'a TypeTree>,
    positions: HashMap<LeafPath, usize>,
}

impl<'a> Preorder<'a> {
    /// Number the leaves of `tree` in preorder. Without a tree the map is empty.
    fn of(tree: Option<&'a TypeTree>) -> Self {
        let positions = tree
            .map(|tree| {
                tree.leaves()
                    .iter()
                    .enumerate()
                    .map(|(position, leaf)| (leaf.path().clone(), position))
                    .collect()
            })
            .unwrap_or_default();
        Self { tree, positions }
    }

    /// Return the sort key of the limit row at `path`.
    fn key(&self, path: &LeafPath) -> RowOrder {
        let place = self.tree.and_then(|tree| {
            let covered = path.concrete_paths(tree)?;
            let position = *self.positions.get(covered.first()?)?;
            Some((position, covered.len()))
        });
        RowOrder::new(place, path)
    }
}

/// Return the limit row of `side` that covers each leaf, directly or through `[*]`.
///
/// A leaf that two rows cover keeps the first row. Without a tree the map is empty.
pub(super) fn effective_limits<'a>(side: LimitSide<'a>) -> HashMap<LeafPath, &'a StoredLimit> {
    let mut covered = HashMap::new();
    let Some(tree) = side.tree else {
        return covered;
    };
    for limit in side.limits {
        for path in limit.leaf_path.concrete_paths(tree).unwrap_or_default() {
            covered.entry(path).or_insert(limit);
        }
    }
    covered
}
