//! The value texts and the limit texts of a grid, and the limit mode of each array level
//! (ADR-0039 (k)).

use std::collections::btree_map::Entry;
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
    /// Return true when the row holds a minimum, a maximum, or a maximum length.
    ///
    /// A blank row only keeps the place of a limit in the form. It is no limit.
    fn has_bound(&self) -> bool {
        [&self.min, &self.max, &self.max_length]
            .iter()
            .any(|text| !text.trim().is_empty())
    }

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

/// Return a row of the leaf path `path` without a bound, or `None` when the tree has no such leaf.
fn blank_row_at(tree: &TypeTree, path: &str) -> Option<LimitTexts> {
    let path = LeafPath::parse(path).ok()?;
    path.resolve(tree)?;
    Some(LimitTexts {
        path,
        min: String::new(),
        max: String::new(),
        max_length: String::new(),
    })
}

/// The limit rows of a grid by the text of their path, and the mode of the pressed type level.
///
/// The type solves one problem: an array level shows "a limit per element" when a row holds an
/// index at that level. A switch without a limit has no such row, so the set keeps the mode of the
/// pressed type level apart (ADR-0039 (k), ADR-0223 (a)).
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
    /// The type levels that hold an index row at one of their array levels (ADR-0223 (c)).
    each_levels: BTreeSet<String>,
    /// The pressed type level and the mode of all its array levels after the switch.
    pressed: Option<(String, bool)>,
    /// The count of the array levels of the pressed type level with unequal limits.
    unequal: usize,
    /// The type levels that had mixed modes, so the set copied their `[*]` limits (ADR-0223 (f)).
    mixed: BTreeSet<String>,
    budget: usize,
    /// The count of the prefix checks of `has_index_at`. Only a test build counts.
    #[cfg(test)]
    checks: std::sync::atomic::AtomicUsize,
    /// The count of the row visits of the switch. Only a test build counts.
    #[cfg(test)]
    visits: usize,
}

impl LimitSet {
    /// Keep the rows that the tree holds, then switch the mode of the type level `toggle`, if any.
    pub(super) fn new(tree: &TypeTree, limits: Vec<LimitTexts>, toggle: Option<&LeafPath>) -> Self {
        let rows: BTreeMap<String, LimitTexts> = limits
            .into_iter()
            .filter(|row| row.path.resolve(tree).is_some())
            .map(|row| (row.path.to_string(), row))
            .collect();
        let index_levels = index_levels(rows.keys());
        let mut set = Self {
            each_levels: each_type_levels(&index_levels),
            index_levels,
            rows,
            pressed: None,
            unequal: 0,
            mixed: BTreeSet::new(),
            budget: usize::try_from(tree.leaf_count()).unwrap_or(0),
            #[cfg(test)]
            checks: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(test)]
            visits: 0,
        };
        set.copy_mixed_levels(tree, toggle);
        if let Some(level) = toggle {
            set.switch(tree, level);
        }
        set
    }

    /// Copy the `[*]` limits of each type level with mixed modes to the element rows.
    ///
    /// The type level then shows a limit per element, as the switch of ADR-0039 (k) does. The
    /// pressed type level gets no mixed notice, because its switch follows at once.
    fn copy_mixed_levels(&mut self, tree: &TypeTree, pressed: Option<&LeafPath>) {
        let levels = self.mixed_type_levels();
        if levels.is_empty() {
            return;
        }
        for level in &levels {
            self.copy_to_elements(tree, level);
        }
        self.refresh_levels();
        let pressed = pressed.map(ToString::to_string);
        self.mixed = levels
            .into_iter()
            .filter(|level| Some(level) != pressed.as_ref())
            .collect();
    }

    /// Return the type levels with a real `[*]` limit at one array level and an index limit at
    /// another.
    ///
    /// The domain permits such limits, because `[*]` and an index must not name one array level
    /// (`domain-model.md` section 2.1).
    fn mixed_type_levels(&self) -> BTreeSet<String> {
        if self.each_levels.is_empty() {
            return BTreeSet::new();
        }
        // `collect` buffers every item before it removes the duplicates. `extend` inserts
        // one item at a time, so only the distinct type levels stay in memory.
        let mut each = BTreeSet::new();
        each.extend(
            self.rows
                .iter()
                .filter(|(_path, row)| row.has_bound())
                .flat_map(|(path, _row)| index_prefixes(path))
                .map(each_type_level),
        );
        let all: BTreeSet<String> = self
            .rows
            .iter()
            .filter(|(_path, row)| row.has_bound())
            .flat_map(|(path, _row)| all_type_levels(path))
            .collect();
        all.intersection(&each).cloned().collect()
    }

    /// Copy each row `[*]` of the type level `level` to the element rows of its array level.
    fn copy_to_elements(&mut self, tree: &TypeTree, level: &str) {
        let Ok(path) = LeafPath::parse(level) else {
            return;
        };
        let shape = segment_texts(level);
        let Some((dimensions, _first)) = self.first_array_level(tree, &path, &shape) else {
            return;
        };
        let indexes = index_texts(dimensions);
        self.split(tree, &shape, &indexes, true);
    }

    /// Return true when the set copied the `[*]` limits of the type level `level`.
    pub(super) fn is_mixed(&self, level: &str) -> bool {
        self.mixed.contains(level)
    }

    /// Return the row of the path `path`.
    pub(super) fn get(&self, path: &str) -> Option<&LimitTexts> {
        self.rows.get(path)
    }

    /// Return true when the array level `level` (a path that ends in `[*]`) has a limit per
    /// element.
    pub(super) fn is_each(&self, level: &str) -> bool {
        match &self.pressed {
            Some((pressed, each)) if *pressed == type_level(level) => *each,
            _other => self.has_index_at(level),
        }
    }

    /// Return true when the type level of `level` shows a limit per element.
    ///
    /// The type level shows it when an index row sits at any of its array levels, not only at the
    /// first one (ADR-0223 (c)). The pressed type level shows the mode of its switch.
    pub(super) fn is_each_type_level(&self, level: &str) -> bool {
        self.is_each_of_type_level(&type_level(level))
    }

    /// Return true when the type level `type_level`, already computed, shows a limit per element.
    pub(super) fn is_each_of_type_level(&self, type_level: &str) -> bool {
        match &self.pressed {
            Some((pressed, each)) if pressed == type_level => *each,
            _other => self.each_levels.contains(type_level),
        }
    }

    /// Recompute the index levels and the per-element type levels after the rows changed.
    fn refresh_levels(&mut self) {
        self.index_levels = index_levels(self.rows.keys());
        self.each_levels = each_type_levels(&self.index_levels);
    }

    /// Return the count of the array levels of the type level `level` with unequal limits.
    ///
    /// The count is not zero only when the switch back of `level` found such array levels.
    pub(super) fn unequal_count(&self, level: &str) -> usize {
        match &self.pressed {
            Some((pressed, _each)) if pressed == level => self.unequal,
            _other => 0,
        }
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

    /// Return the count of the row visits of the switch so far.
    #[cfg(test)]
    pub(super) fn visits(&self) -> usize {
        self.visits
    }

    /// Add `count` row visits. A normal build has no counter.
    fn count_visits(&mut self, count: usize) {
        #[cfg(test)]
        {
            self.visits = self.visits.saturating_add(count);
        }
        #[cfg(not(test))]
        let _ = count;
    }

    /// Switch the mode of the type level `level` and move the rows of each of its array levels.
    ///
    /// The mode of the type level tells the direction, because the mode row shows that mode. The
    /// type level has a limit per element when an index row sits at any of its array levels. The
    /// pressed switch overrides this mode. A level with an index segment is no type level, and the
    /// switch skips it. The reader refuses such a `limit_mode` (ADR-0223 (b)).
    fn switch(&mut self, tree: &TypeTree, level: &LeafPath) {
        let level_text = level.to_string();
        if level
            .segments()
            .iter()
            .any(|part| matches!(part, LeafSegment::Index(_)))
        {
            return;
        }
        let shape = segment_texts(&level_text);
        let Some((dimensions, first)) = self.first_array_level(tree, level, &shape) else {
            return;
        };
        let indexes = index_texts(dimensions);
        let each = !self.is_each_type_level(&first);
        if each {
            self.split(tree, &shape, &indexes, false);
        } else {
            self.join(tree, &shape, indexes.len());
        }
        self.count_visits(self.rows.len());
        self.refresh_levels();
        self.pressed = Some((level_text, each));
    }

    /// Return the dimensions of the type level `level` and the text of its first array level.
    ///
    /// The walk meets the first array level first. Each outer `[*]` of the type level gives its
    /// first index when that array level has a limit per element, and `[*]` otherwise.
    fn first_array_level<'t>(
        &self,
        tree: &'t TypeTree,
        level: &LeafPath,
        shape: &[&str],
    ) -> Option<(&'t ArrayDimensions, String)> {
        let mut node = tree.root();
        let mut text = String::new();
        let mut parts = shape.iter().zip(level.segments()).peekable();
        while let Some((token, segment)) = parts.next() {
            match (node, segment) {
                (TypeNode::Struct { members, .. }, LeafSegment::Member(name)) => {
                    node = members
                        .iter()
                        .find(|(member, _)| member == name)
                        .map(|(_, child)| child)?;
                    text.push_str(token);
                }
                (
                    TypeNode::Array {
                        dimensions,
                        element,
                        ..
                    },
                    LeafSegment::All,
                ) => {
                    if parts.peek().is_none() {
                        text.push_str(token);
                        return Some((dimensions, text));
                    }
                    let own = format!("{text}{ALL}");
                    if self.has_index_at(&own) {
                        text.push_str(&first_index_text(dimensions));
                    } else {
                        text.push_str(ALL);
                    }
                    node = element;
                }
                _other => return None,
            }
        }
        None
    }

    /// Copy each row `<array level>[*]<tail>` to each element `<array level>[i]<tail>`.
    ///
    /// The set then holds at most one row per leaf of the tree. The copies above that count drop.
    /// One pass over the rows finds the rows of every array level of the type level.
    /// The copy of mixed modes sets `keep_blank`: a blank row is no limit and stays one row.
    fn split(&mut self, tree: &TypeTree, shape: &[&str], indexes: &[String], keep_blank: bool) {
        let mut moved: Vec<(String, String, LimitTexts)> = Vec::new();
        let mut visits = 0_usize;
        self.rows.retain(|path, row| {
            visits = visits.saturating_add(1);
            match split_at_level(path, shape) {
                Some(cut) if cut.element == ALL && !(keep_blank && !row.has_bound()) => {
                    moved.push((cut.prefix.to_owned(), cut.tail.to_owned(), row.clone()));
                    false
                }
                _other => true,
            }
        });
        self.count_visits(visits);
        let room = self.budget.saturating_sub(self.rows.len());
        let copies: Vec<LimitTexts> = moved
            .iter()
            .flat_map(|(prefix, tail, row)| {
                indexes
                    .iter()
                    .map(move |index| (row, format!("{prefix}{index}{tail}")))
            })
            .filter(|(_row, path)| !self.rows.contains_key(path))
            .take(room)
            .filter_map(|(row, path)| row.moved_to(tree, &path))
            .collect();
        for copy in copies {
            self.rows.entry(copy.path.to_string()).or_insert(copy);
        }
    }

    /// Join the rows `<array level>[i]<tail>` of each array level and tail into one row
    /// `<array level>[*]<tail>`.
    ///
    /// A tail joins only when every element has a row and all rows have the same bounds.
    /// Otherwise the row `[*]` of that array level stays empty. The set counts each array level
    /// with such a tail once. One pass over the rows finds the rows of every array level.
    fn join(&mut self, tree: &TypeTree, shape: &[&str], elements: usize) {
        let mut groups: BTreeMap<(String, String), JoinGroup> = BTreeMap::new();
        let mut visits = 0_usize;
        self.rows.retain(|path, row| {
            visits = visits.saturating_add(1);
            let Some(cut) = split_at_level(path, shape).filter(|cut| cut.element != ALL) else {
                return true;
            };
            let key = (cut.prefix.to_owned(), cut.tail.to_owned());
            match groups.entry(key) {
                Entry::Vacant(slot) => {
                    slot.insert(JoinGroup {
                        first: row.clone(),
                        count: 1,
                        equal: true,
                    });
                }
                Entry::Occupied(mut slot) => {
                    let group = slot.get_mut();
                    match group.count.checked_add(1) {
                        Some(count) => group.count = count,
                        None => group.equal = false,
                    }
                    group.equal = group.equal && group.first.has_bounds_of(row);
                }
            }
            false
        });
        self.count_visits(visits);
        let mut unequal_levels = BTreeSet::new();
        for ((prefix, tail), group) in groups {
            if group.equal && group.count == elements {
                let path = format!("{prefix}{ALL}{tail}");
                if let Some(joined) = group.first.moved_to(tree, &path) {
                    self.rows.insert(joined.path.to_string(), joined);
                }
            } else {
                // An empty `[*]` row keeps the outer index of the group (ADR-0223 (d)).
                let path = format!("{prefix}{ALL}{tail}");
                if let Some(empty) = blank_row_at(tree, &path) {
                    self.rows.insert(empty.path.to_string(), empty);
                }
                unequal_levels.insert(prefix);
            }
        }
        self.unequal = unequal_levels.len();
    }
}

/// The rows `<array level>[i]<tail>` of one array level and one tail, while a join runs.
struct JoinGroup {
    /// The first row of the group. The joined row takes its bounds.
    first: LimitTexts,
    /// The count of the rows so far.
    count: usize,
    /// True while every row so far has the bounds of `first`.
    equal: bool,
}

/// A row path cut at the last segment of a type level.
struct LevelCut<'p> {
    /// The text before the segment: the array level without its last `[*]`.
    prefix: &'p str,
    /// The segment itself: `[*]` or an index segment.
    element: &'p str,
    /// The text after the segment. It is empty or starts a new segment.
    tail: &'p str,
}

/// Return the type level of the array level `level`: each index segment becomes `[*]`.
///
/// The type solves one problem: the grid shows one mode row for one type level, and the type
/// level `[*].arr[*]` covers `[1].arr[*]`, `[2].arr[*]`, and each other array level of that
/// array (ADR-0223). A segment such as `[1,2]` of a two-dimensional array becomes one `[*]`.
pub(super) fn type_level(level: &str) -> String {
    let mut text = String::with_capacity(level.len());
    let mut inside = false;
    for ch in level.chars() {
        match (ch, inside) {
            ('[', false) => {
                inside = true;
                text.push_str(ALL);
            }
            (']', true) => inside = false,
            (_, true) => {}
            (_, false) => text.push(ch),
        }
    }
    text
}

/// Cut the first segment off a path text: a member `.name` or an index segment `[1,2]`.
fn split_segment(text: &str) -> Option<(&str, &str)> {
    let end = match text.chars().next()? {
        '[' => text.find(']')?.checked_add(1)?,
        '.' => match text.get(1..)?.find(['.', '[']) {
            Some(at) => at.checked_add(1)?,
            None => text.len(),
        },
        _other => return None,
    };
    text.split_at_checked(end)
}

/// Return the segment texts of a path text, such as `[".x", "[*]", ".v"]` of `.x[*].v`.
fn segment_texts(path: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut rest = path;
    while let Some((segment, after)) = split_segment(rest) {
        segments.push(segment);
        rest = after;
    }
    segments
}

/// Cut `path` at the last segment of the type level `shape`, or return `None` when it differs.
///
/// A `[*]` of the type level before its last segment matches `[*]` or an index segment. The
/// last segment of the path must be an array segment.
fn split_at_level<'p>(path: &'p str, shape: &[&str]) -> Option<LevelCut<'p>> {
    let (_last, outer) = shape.split_last()?;
    let mut rest = path;
    for want in outer {
        let (segment, after) = split_segment(rest)?;
        let same = if *want == ALL {
            segment.starts_with('[')
        } else {
            segment == *want
        };
        if !same {
            return None;
        }
        rest = after;
    }
    let (element, tail) = split_segment(rest)?;
    if !element.starts_with('[') {
        return None;
    }
    let prefix = path.get(..path.len().checked_sub(rest.len())?)?;
    Some(LevelCut {
        prefix,
        element,
        tail,
    })
}

/// Return the text before each index segment of each path, such as `[*].x` of `[*].x[2].v`.
///
/// A level `<prefix>[*]` has a limit per element when the set holds its `<prefix>`.
fn index_levels<'p>(paths: impl Iterator<Item = &'p String>) -> BTreeSet<String> {
    paths
        .flat_map(|path| index_prefixes(path))
        .map(str::to_owned)
        .collect()
}

/// Return the borrowed text before each index segment of one path.
///
/// This avoids one owned string per prefix when the caller maps each prefix at once.
fn index_prefixes(path: &str) -> impl Iterator<Item = &str> {
    path.match_indices('[').filter_map(|(start, _bracket)| {
        let rest = path.get(start..)?;
        element_tail(rest)?;
        path.get(..start)
    })
}

/// Return the type level of the array level `<prefix>[*]`.
fn each_type_level(prefix: &str) -> String {
    type_level(&format!("{prefix}{ALL}"))
}

/// Return the type level of each array level that follows one of the index level prefixes.
fn each_type_levels(index_levels: &BTreeSet<String>) -> BTreeSet<String> {
    index_levels
        .iter()
        .map(|prefix| each_type_level(prefix))
        .collect()
}

/// Return the type level of each `[*]` segment of a path, such as `[*].a[*]` of `[1].a[*]`.
fn all_type_levels(path: &str) -> impl Iterator<Item = String> + '_ {
    path.match_indices(ALL)
        .filter_map(|(start, segment)| path.get(..start.checked_add(segment.len())?))
        .map(type_level)
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

/// Write the index segment of the first element of an array, such as `[1,1]`.
fn first_index_text(dimensions: &ArrayDimensions) -> String {
    let parts: Vec<String> = dimensions
        .iter()
        .map(|dimension| dimension.lower().to_string())
        .collect();
    format!("[{}]", parts.join(","))
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
