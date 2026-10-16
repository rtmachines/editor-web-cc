//! The value texts and the limit texts of a grid, and the limit mode of each array level
//! (ADR-0039 (k)).

use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::plant::{ArrayDimensions, LeafPath, LeafSegment, TypeNode, TypeTree};
use recipe_domain::recipe::{LeafText, StoredLimit};

use super::texts::bound_text;
use crate::pages::parameters::fields::SentLimit;

/// The segment that names every element of an array level.
const ALL: &str = "[*]";

/// The text of each value input by the text of its leaf path.
///
/// The type solves one problem: a page shows the stored texts. An answer that renders the form
/// again shows the sent texts. Both come into one map, so the grid has one path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ValueTexts(BTreeMap<String, String>);

impl ValueTexts {
    /// Take the texts of a stored value (`ParameterValue::texts`).
    pub(crate) fn from_stored(texts: &[LeafText]) -> Self {
        Self(
            texts
                .iter()
                .map(|leaf| (leaf.path.to_string(), leaf.text.clone()))
                .collect(),
        )
    }

    /// Take the value fields of a form as the form sent them.
    pub(crate) fn from_sent(values: &[(LeafPath, String)]) -> Self {
        Self(
            values
                .iter()
                .map(|(path, text)| (path.to_string(), text.clone()))
                .collect(),
        )
    }

    /// Return the text of the leaf path `path`, or an empty text.
    pub(super) fn get(&self, path: &str) -> &str {
        self.0.get(path).map_or("", String::as_str)
    }
}

/// The texts of one limit row of a grid: the leaf path and the three bound inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LimitTexts {
    /// The leaf path of the row. It can hold `[*]`.
    pub(crate) path: LeafPath,
    /// The text of the minimum.
    pub(crate) min: String,
    /// The text of the maximum.
    pub(crate) max: String,
    /// The text of the maximum length.
    pub(crate) max_length: String,
}

impl LimitTexts {
    /// Write the texts of stored limits. A time bound shows as its canonical IEC literal.
    pub(crate) fn from_stored(tree: &TypeTree, limits: &[StoredLimit]) -> Vec<Self> {
        limits
            .iter()
            .map(|limit| {
                let primitive = limit
                    .leaf_path
                    .resolve(tree)
                    .map(|target| target.primitive());
                let text = |bound: Option<String>| {
                    bound.map_or_else(String::new, |bound| match primitive {
                        Some(primitive) => bound_text(primitive, tree.platform(), bound),
                        None => bound,
                    })
                };
                Self {
                    path: limit.leaf_path.clone(),
                    min: text(limit.min.map(|bound| bound.to_string())),
                    max: text(limit.max.map(|bound| bound.to_string())),
                    max_length: limit
                        .max_length
                        .map_or_else(String::new, |length| length.to_string()),
                }
            })
            .collect()
    }

    /// Take the limit rows of a form as the form sent them.
    pub(crate) fn from_sent(limits: &[SentLimit]) -> Vec<Self> {
        limits
            .iter()
            .map(|row| Self {
                path: row.leaf_path.clone(),
                min: row.min.clone(),
                max: row.max.clone(),
                max_length: row.max_length.clone(),
            })
            .collect()
    }

    /// Return true when two rows have the same bounds after a trim.
    fn has_bounds_of(&self, other: &Self) -> bool {
        self.min.trim() == other.min.trim()
            && self.max.trim() == other.max.trim()
            && self.max_length.trim() == other.max_length.trim()
    }

    /// Return a copy of the row with the leaf path `path`, or `None` when the tree has no such leaf.
    fn moved_to(&self, tree: &TypeTree, path: &str) -> Option<Self> {
        let path = LeafPath::parse(path).ok()?;
        path.resolve(tree)?;
        Some(Self {
            path,
            ..self.clone()
        })
    }
}

/// The limit rows of a grid by the text of their path, and the mode of the pressed level.
///
/// The type solves one problem: a level shows "a limit per element" when a row holds an index at
/// that level. A switch without a limit has no such row, so the set keeps the mode of the
/// pressed level apart (ADR-0039 (k)).
///
/// A row without a bound stays in the set, because its leaf path names the mode of the form. A
/// forged form can send any path, so the set keeps only the rows that the type tree holds. A
/// switch stops at one row per leaf of the tree.
///
/// A forged form can send more rows than the tree has leaves. The set therefore keeps the levels
/// that hold an index row, so the mode check of each array node is one lookup and no scan.
pub(super) struct LimitSet {
    rows: BTreeMap<String, LimitTexts>,
    /// The text before each index segment of a row path, such as `[*].x` of `[*].x[2].v`.
    index_levels: BTreeSet<String>,
    pressed: Option<(String, bool)>,
    unequal: bool,
    budget: usize,
    /// The count of the prefix checks of `has_index_at`. Only a test build counts.
    #[cfg(test)]
    checks: std::sync::atomic::AtomicUsize,
}

impl LimitSet {
    /// Keep the rows that the tree holds, then switch the mode of the level `toggle`, if any.
    pub(super) fn new(tree: &TypeTree, limits: Vec<LimitTexts>, toggle: Option<&LeafPath>) -> Self {
        let rows: BTreeMap<String, LimitTexts> = limits
            .into_iter()
            .filter(|row| row.path.resolve(tree).is_some())
            .map(|row| (row.path.to_string(), row))
            .collect();
        let mut set = Self {
            index_levels: index_levels(rows.keys()),
            rows,
            pressed: None,
            unequal: false,
            budget: usize::try_from(tree.leaf_count()).unwrap_or(0),
            #[cfg(test)]
            checks: std::sync::atomic::AtomicUsize::new(0),
        };
        if let Some(level) = toggle {
            set.switch(tree, level);
        }
        set
    }

    /// Return the row of the path `path`.
    pub(super) fn get(&self, path: &str) -> Option<&LimitTexts> {
        self.rows.get(path)
    }

    /// Return true when the level `level` (a path that ends in `[*]`) has a limit per element.
    pub(super) fn is_each(&self, level: &str) -> bool {
        match &self.pressed {
            Some((pressed, each)) if pressed == level => *each,
            _other => self.has_index_at(level),
        }
    }

    /// Return true when the switch back of `level` found elements with different limits.
    pub(super) fn is_unequal(&self, level: &str) -> bool {
        self.unequal
            && self
                .pressed
                .as_ref()
                .is_some_and(|(pressed, _)| pressed == level)
    }

    /// Return true when a row holds an index segment right after the prefix of `level`.
    fn has_index_at(&self, level: &str) -> bool {
        let prefix = level.strip_suffix(ALL).unwrap_or(level);
        #[cfg(test)]
        self.checks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.index_levels.contains(prefix)
    }

    /// Return the count of the prefix checks so far.
    #[cfg(test)]
    pub(super) fn checks(&self) -> usize {
        self.checks.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Switch the mode of `level` and move its rows (ADR-0039 (k)).
    fn switch(&mut self, tree: &TypeTree, level: &LeafPath) {
        let level_text = level.to_string();
        let Some(indexes) = level_node(tree, level).map(index_texts) else {
            return;
        };
        let prefix = level_text
            .strip_suffix(ALL)
            .unwrap_or(&level_text)
            .to_owned();
        let each = !self.has_index_at(&level_text);
        if each {
            self.split(tree, &prefix, &indexes);
        } else {
            self.join(tree, &prefix, &indexes);
        }
        self.index_levels = index_levels(self.rows.keys());
        self.pressed = Some((level_text, each));
    }

    /// Copy each row `<prefix>[*]<tail>` to each element `<prefix>[i]<tail>`.
    ///
    /// The set then holds at most one row per leaf of the tree. The copies above that count drop.
    fn split(&mut self, tree: &TypeTree, prefix: &str, indexes: &[String]) {
        let all = format!("{prefix}{ALL}");
        let moved: Vec<(String, LimitTexts)> = self
            .rows
            .iter()
            .filter_map(|(path, row)| Some((path.strip_prefix(&all)?.to_owned(), row.clone())))
            .collect();
        for (tail, _row) in &moved {
            self.rows.remove(&format!("{all}{tail}"));
        }
        let room = self.budget.saturating_sub(self.rows.len());
        let copies: Vec<LimitTexts> = moved
            .iter()
            .flat_map(|(tail, row)| {
                indexes
                    .iter()
                    .map(move |index| (row, format!("{prefix}{index}{tail}")))
            })
            .take(room)
            .filter_map(|(row, path)| row.moved_to(tree, &path))
            .collect();
        for copy in copies {
            self.rows.insert(copy.path.to_string(), copy);
        }
    }

    /// Join the rows `<prefix>[i]<tail>` of each tail into one row `<prefix>[*]<tail>`.
    ///
    /// A tail joins only when every element has a row and all rows have the same bounds.
    /// Otherwise the row `[*]` stays empty, and the set notes the difference.
    fn join(&mut self, tree: &TypeTree, prefix: &str, indexes: &[String]) {
        let found: Vec<(String, String)> = self
            .rows
            .keys()
            .filter_map(|path| Some((path.clone(), element_tail(path.strip_prefix(prefix)?)?)))
            .collect();
        let mut tails: BTreeMap<String, Vec<LimitTexts>> = BTreeMap::new();
        for (path, tail) in found {
            if let Some(row) = self.rows.remove(&path) {
                tails.entry(tail).or_default().push(row);
            }
        }
        for (tail, rows) in tails {
            let first = rows.first();
            let equal = rows.len() == indexes.len()
                && first.is_some_and(|first| rows.iter().all(|row| row.has_bounds_of(first)));
            match first.filter(|_| equal) {
                Some(row) => {
                    if let Some(joined) = row.moved_to(tree, &format!("{prefix}{ALL}{tail}")) {
                        self.rows.insert(joined.path.to_string(), joined);
                    }
                }
                None => self.unequal = true,
            }
        }
    }
}

/// Return the text before each index segment of each path, such as `[*].x` of `[*].x[2].v`.
///
/// A level `<prefix>[*]` has a limit per element when the set holds its `<prefix>`.
fn index_levels<'p>(paths: impl Iterator<Item = &'p String>) -> BTreeSet<String> {
    paths
        .flat_map(|path| {
            path.match_indices('[').filter_map(|(start, _bracket)| {
                let rest = path.get(start..)?;
                element_tail(rest)?;
                path.get(..start)
            })
        })
        .map(str::to_owned)
        .collect()
}

/// Return the tail after the index segment that starts `rest`, or `None` without such a segment.
///
/// The segment `[*]` is no index segment. The tail is empty or starts a new segment.
fn element_tail(rest: &str) -> Option<String> {
    if !rest.starts_with('[') || rest.starts_with(ALL) {
        return None;
    }
    let (_index, tail) = rest.split_once(']')?;
    (tail.is_empty() || tail.starts_with(['.', '['])).then(|| tail.to_owned())
}

/// Return the array node of a level path that ends in `[*]`.
fn level_node<'t>(tree: &'t TypeTree, level: &LeafPath) -> Option<&'t ArrayDimensions> {
    let (_last, before) = level.segments().split_last()?;
    let mut node = tree.root();
    for segment in before {
        node = match (node, segment) {
            (TypeNode::Struct { members, .. }, LeafSegment::Member(name)) => members
                .iter()
                .find(|(member, _)| member == name)
                .map(|(_, child)| child)?,
            (TypeNode::Array { element, .. }, LeafSegment::Index(_) | LeafSegment::All) => element,
            _other => return None,
        };
    }
    match node {
        TypeNode::Array { dimensions, .. } => Some(dimensions),
        _other => None,
    }
}

/// Write the index segment of each element of an array in row-major order, such as `[0,1]`.
pub(super) fn index_texts(dimensions: &ArrayDimensions) -> Vec<String> {
    let tuples = dimensions
        .iter()
        .fold(vec![Vec::new()], |tuples, dimension| {
            tuples
                .iter()
                .flat_map(|prefix: &Vec<i32>| {
                    (dimension.lower()..=dimension.upper()).map(move |index| {
                        let mut tuple = prefix.clone();
                        tuple.push(index);
                        tuple
                    })
                })
                .collect()
        });
    tuples
        .iter()
        .map(|tuple| {
            let parts: Vec<String> = tuple.iter().map(i32::to_string).collect();
            format!("[{}]", parts.join(","))
        })
        .collect()
}
