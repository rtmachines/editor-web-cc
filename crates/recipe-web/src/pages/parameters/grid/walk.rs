//! The walk over a type tree that builds the rows of a grid (design D8).
//!
//! The walk uses an explicit stack, as every walk over a type tree does (`reliability.md`
//! section 2.2). A step is a node, a mode row, the start of a group, or the end of a group.

use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::plant::{ArrayDimensions, LeafClass, Primitive, TypeNode, TypeTree};

use super::limits::{LimitSet, ValueTexts, index_texts, type_level};
use super::texts::{hint_key, message, type_text};
use super::{
    AllCell, GridBody, GridHint, GridRow, GridSource, GridView, HiddenField, InputCell, LeafIds,
    LeafInput, LeafRow, ModeRow, ToggleRow, ValueCell, limit_input_id,
};
use crate::pages::Texts;

/// The segment that names every element of an array level.
const ALL: &str = "[*]";

/// One step of the walk.
enum Step<'t> {
    /// A node. `value` is its concrete path, or `None` inside the rows `[*]` of a level. `limit`
    /// is the path of its limit row. `range` is the index range of the innermost row `[*]`.
    Node {
        node: &'t TypeNode,
        value: Option<String>,
        limit: String,
        range: String,
    },
    /// The mode row of the array level `level`. The row stands for its type level (ADR-0223).
    Mode { level: String },
    /// The toggle row and the start of a group of the level `level` with `count` elements.
    Open {
        level: String,
        range: String,
        count: usize,
        open: bool,
    },
    /// The end of a group.
    Close,
}

/// One leaf of the walk with the paths of its value and its limit.
struct LeafSpot<'s> {
    primitive: Primitive,
    string_length: Option<u16>,
    value: Option<&'s str>,
    limit: &'s str,
    range: &'s str,
}

/// The state of the walk: the input and the rows so far.
struct Walk<'a> {
    t: Texts<'a>,
    values: ValueTexts,
    limits: LimitSet,
    chunk: usize,
    root_label: String,
    bodies: Vec<GridBody>,
    rows: Vec<GridRow>,
    group: Option<(String, bool)>,
    counts: Counts,
    hints: Vec<GridHint>,
    hint_ids: BTreeSet<String>,
    ids: BTreeMap<String, LeafIds>,
    /// The type levels that have a mode row already.
    mode_levels: BTreeSet<String>,
}

/// The counters of the `id` values of a grid.
#[derive(Default)]
struct Counts {
    rows: usize,
    values: usize,
    limits: usize,
    modes: usize,
    groups: usize,
}

/// Return the counter value and add one to the counter.
fn next(counter: &mut usize) -> usize {
    let value = *counter;
    *counter = counter.saturating_add(1);
    value
}

/// Build the grid of `source` (design D8).
pub(super) fn build(source: GridSource<'_>, t: Texts<'_>) -> GridView {
    let tree: &TypeTree = source.tree;
    let mut walk = Walk {
        t,
        values: source.values,
        limits: LimitSet::new(tree, source.limits, source.toggle),
        chunk: source.chunk,
        root_label: source.root_label,
        bodies: Vec::new(),
        rows: Vec::new(),
        group: None,
        counts: Counts::default(),
        hints: Vec::new(),
        hint_ids: BTreeSet::new(),
        ids: BTreeMap::new(),
        mode_levels: BTreeSet::new(),
    };
    let mut stack = vec![Step::Node {
        node: tree.root(),
        value: Some(String::new()),
        limit: String::new(),
        range: String::new(),
    }];
    while let Some(step) = stack.pop() {
        let mut ahead = walk.step(step);
        ahead.reverse();
        stack.extend(ahead);
    }
    walk.flush();
    GridView {
        hints: walk.hints,
        bodies: walk.bodies,
        ids: walk.ids,
    }
}

impl<'a> Walk<'a> {
    /// Run one step and return the steps that follow it, in their order.
    fn step<'t>(&mut self, step: Step<'t>) -> Vec<Step<'t>> {
        match step {
            Step::Node {
                node,
                value,
                limit,
                range,
            } => self.node(node, value, limit, range),
            Step::Mode { level } => {
                self.mode_row(&level);
                Vec::new()
            }
            Step::Open {
                level,
                range,
                count,
                open,
            } => {
                self.open_group(&level, (&range, count), open);
                Vec::new()
            }
            Step::Close => {
                self.flush();
                self.group = None;
                Vec::new()
            }
        }
    }

    /// Write the row of a leaf, or return the steps of the children of a struct or an array.
    fn node<'t>(
        &mut self,
        node: &'t TypeNode,
        value: Option<String>,
        limit: String,
        range: String,
    ) -> Vec<Step<'t>> {
        match node {
            TypeNode::Leaf {
                primitive,
                string_length,
                ..
            } => {
                self.leaf_row(&LeafSpot {
                    primitive: *primitive,
                    string_length: *string_length,
                    value: value.as_deref(),
                    limit: &limit,
                    range: &range,
                });
                Vec::new()
            }
            TypeNode::Struct { members, .. } => members
                .iter()
                .map(|(name, child)| Step::Node {
                    node: child,
                    value: value.as_ref().map(|path| format!("{path}.{name}")),
                    limit: format!("{limit}.{name}"),
                    range: range.clone(),
                })
                .collect(),
            TypeNode::Array {
                dimensions,
                element,
                ..
            } => self.array(dimensions, element, value, (limit, range)),
        }
    }

    /// Return the steps of an array: its rows `[*]`, its mode row, and its elements.
    ///
    /// The rows `[*]` and the mode step of a level stand once. They stand where the value path of
    /// the array equals its limit path, or inside the rows `[*]` of an outer level. The walk
    /// writes the mode row only at the first array level of each type level.
    fn array<'t>(
        &self,
        dimensions: &ArrayDimensions,
        element: &'t TypeNode,
        value: Option<String>,
        (limit, range): (String, String),
    ) -> Vec<Step<'t>> {
        let level = format!("{limit}{ALL}");
        let each = self.limits.is_each(&level);
        let owns_level = value.as_deref().is_none_or(|path| path == limit);
        let own_range = format!("[{dimensions}]");
        let mut steps = Vec::new();
        if owns_level && !each {
            steps.push(Step::Node {
                node: element,
                value: None,
                limit: level.clone(),
                range: own_range.clone(),
            });
        }
        if owns_level {
            steps.push(Step::Mode {
                level: level.clone(),
            });
        }
        if value.is_none() && !each {
            return steps;
        }
        let elements: Vec<(String, Step<'t>)> = index_texts(dimensions)
            .into_iter()
            .map(|index| {
                let step = Step::Node {
                    node: element,
                    value: value.as_ref().map(|path| format!("{path}{index}")),
                    limit: if each {
                        format!("{limit}{index}")
                    } else {
                        level.clone()
                    },
                    range: range.clone(),
                };
                (index, step)
            })
            .collect();
        steps.extend(self.grouped(&level, elements));
        steps
    }

    /// Split the element steps of a level into groups of `chunk` elements (ADR-0039 (l)).
    ///
    /// An array inside a group, and an array with at most `chunk` elements, has no group.
    fn grouped<'t>(&self, level: &str, elements: Vec<(String, Step<'t>)>) -> Vec<Step<'t>> {
        if self.chunk == 0 || self.group.is_some() || elements.len() <= self.chunk {
            return elements.into_iter().map(|(_index, step)| step).collect();
        }
        let mut steps = Vec::new();
        let mut rest = elements.into_iter().peekable();
        let mut first = true;
        while rest.peek().is_some() {
            let chunk: Vec<(String, Step<'t>)> = rest.by_ref().take(self.chunk).collect();
            let start = chunk
                .first()
                .map(|(index, _)| index.clone())
                .unwrap_or_default();
            let end = chunk
                .last()
                .map(|(index, _)| index.clone())
                .unwrap_or_default();
            steps.push(Step::Open {
                level: level.to_owned(),
                range: format!("{start}..{end}"),
                count: chunk.len(),
                open: first,
            });
            steps.extend(chunk.into_iter().map(|(_index, step)| step));
            steps.push(Step::Close);
            first = false;
        }
        steps
    }

    /// Write the toggle row in its own body and start the body of the group.
    ///
    /// The label names the index range and the count of the elements (`design-system.md` 10.8).
    fn open_group(&mut self, level: &str, (range, count): (&str, usize), open: bool) {
        self.flush();
        let group = format!("grid-group-{}", next(&mut self.counts.groups));
        let count = count.to_string();
        let text = message(
            self.t,
            "grid-group",
            &[("range", range), ("level", level), ("count", &count)],
        );
        self.bodies.push(GridBody {
            id: String::new(),
            hidden: false,
            rows: vec![GridRow::Toggle(ToggleRow {
                group: group.clone(),
                expanded: open,
                text,
            })],
        });
        self.group = Some((group, !open));
    }

    /// Close the current body. A body without rows gives no `tbody`.
    fn flush(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let (id, hidden) = self.group.clone().unwrap_or_default();
        self.bodies.push(GridBody {
            id,
            hidden,
            rows: std::mem::take(&mut self.rows),
        });
    }

    /// Write the mode row of the type level of the array level `level` (ADR-0039 (k), ADR-0223).
    ///
    /// The first array level of a type level writes the row. The array levels that follow write
    /// none. The row shows the mode of the type level. The type level has a limit per element when
    /// an index row sits at any of its array levels. The pressed switch overrides this mode.
    fn mode_row(&mut self, level: &str) {
        let level = type_level(level);
        if !self.mode_levels.insert(level.clone()) {
            return;
        }
        let each = self.limits.is_each_of_type_level(&level);
        let (button, current) = if each {
            ("grid-mode-join", "grid-mode-current-each")
        } else {
            ("grid-mode-split", "grid-mode-current-all")
        };
        let args = [("level", level.as_str())];
        let notice = self.notice_of(&level);
        self.rows.push(GridRow::Mode(ModeRow {
            id: format!("grid-mode-{}", next(&mut self.counts.modes)),
            label: message(self.t, button, &args),
            current: message(self.t, current, &args),
            notice,
            level,
        }));
    }

    /// Write the notice of the mode row of the type level `level`, or an empty text.
    ///
    /// A switch back with unequal limits gives the join notice. A type level that had mixed modes
    /// gives the mixed notice (ADR-0223 (f)).
    fn notice_of(&self, level: &str) -> String {
        let unequal = self.limits.unequal_count(level);
        if unequal > 0 {
            let count = unequal.to_string();
            return message(
                self.t,
                "grid-mode-join-notice",
                &[("level", level), ("count", count.as_str())],
            );
        }
        if self.limits.is_mixed(level) {
            return message(self.t, "grid-mode-mixed-notice", &[("level", level)]);
        }
        String::new()
    }

    /// Write the row of one leaf or of one leaf of the rows `[*]`.
    fn leaf_row(&mut self, spot: &LeafSpot<'_>) {
        let id = format!("grid-row-{}", next(&mut self.counts.rows));
        let shown = spot.value.unwrap_or(spot.limit);
        let path = if shown.is_empty() {
            self.root_label.clone()
        } else {
            shown.to_owned()
        };
        let all = spot.value.is_none().then(|| AllCell {
            id: format!("{id}-all"),
            text: message(self.t, "grid-all-elements", &[("range", spot.range)]),
        });
        let value = spot.value.map(|path| self.value_cell(spot, path));
        let mut row = LeafRow {
            id,
            path,
            type_text: type_text(spot.primitive, spot.string_length),
            all,
            value,
            limit_path: None,
            min: None,
            max: None,
            max_length: None,
            from_all: String::new(),
        };
        self.limit_cells(spot, &mut row);
        self.rows.push(GridRow::Leaf(Box::new(row)));
    }

    /// Build the value input of a leaf and note its `id` for the error lines.
    fn value_cell(&mut self, spot: &LeafSpot<'_>, path: &str) -> ValueCell {
        let id = format!("value-{}", next(&mut self.counts.values));
        self.note(path, LeafInput::Value, &id);
        let class = spot.primitive.leaf_class();
        ValueCell {
            name: format!("value{path}"),
            text: self.values.get(path).to_owned(),
            choice: class == LeafClass::Bool,
            maxlength: if class == LeafClass::String {
                spot.string_length.unwrap_or_default()
            } else {
                0
            },
            hint: self.hint(spot.primitive, spot.string_length),
            id,
        }
    }

    /// Return the `id` of the hint of a leaf, and add the hint at its first leaf.
    fn hint(&mut self, primitive: Primitive, string_length: Option<u16>) -> String {
        let Some(key) = hint_key(primitive, string_length) else {
            return String::new();
        };
        if self.hint_ids.insert(key.id.clone()) {
            let text = match &key.arg {
                Some((name, value)) => message(self.t, key.key, &[(*name, value.as_str())]),
                None => self.t.get(key.key),
            };
            self.hints.push(GridHint {
                id: key.id.clone(),
                text,
            });
        }
        key.id
    }

    /// Fill the limit cells of a row: the inputs, the text of the row `[*]`, or nothing.
    fn limit_cells(&mut self, spot: &LeafSpot<'_>, row: &mut LeafRow) {
        let class = spot.primitive.leaf_class();
        let is_string = class == LeafClass::String;
        if matches!(class, LeafClass::Bool | LeafClass::Char) {
            return;
        }
        if let Some(path) = spot.value.filter(|path| *path != spot.limit) {
            row.from_all = self.limit_of_all(spot.limit, is_string);
            let all = self.ids.get(spot.limit);
            let found = [
                (LeafInput::Limit, all.and_then(|ids| ids.limit.clone())),
                (
                    LeafInput::LimitMax,
                    all.and_then(|ids| ids.limit_max.clone()),
                ),
                (
                    LeafInput::LimitMaxLength,
                    all.and_then(|ids| ids.limit_max_length.clone()),
                ),
            ];
            for (input, id) in found {
                if let Some(id) = id {
                    self.note(path, input, &id);
                }
            }
            return;
        }
        let index = next(&mut self.counts.limits);
        let hint = if is_string {
            String::new()
        } else {
            self.hint(spot.primitive, spot.string_length)
        };
        let texts = self.limits.get(spot.limit);
        let input = |field: &str, text: Option<&String>| InputCell {
            id: limit_input_id(index, field),
            name: format!("limits[{index}].{field}"),
            text: text.cloned().unwrap_or_default(),
            hint: hint.clone(),
        };
        if is_string {
            row.max_length = Some(input("max_length", texts.map(|row| &row.max_length)));
        } else {
            row.min = Some(input("min", texts.map(|row| &row.min)));
            row.max = Some(input("max", texts.map(|row| &row.max)));
        }
        let first = row.min.as_ref().or(row.max_length.as_ref());
        if let Some(first) = first {
            self.note(spot.limit, LeafInput::Limit, &first.id);
        }
        if let Some(max) = &row.max {
            self.note(spot.limit, LeafInput::LimitMax, &max.id);
        }
        if let Some(length) = &row.max_length {
            self.note(spot.limit, LeafInput::LimitMaxLength, &length.id);
        }
        row.limit_path = Some(HiddenField {
            name: format!("limits[{index}].leaf_path"),
            value: spot.limit.to_owned(),
        });
    }

    /// Write the limit of the row `[*]` of an element row (design D8).
    fn limit_of_all(&self, limit: &str, is_string: bool) -> String {
        let row = self.limits.get(limit);
        let path = ("path", limit);
        match row {
            Some(row) if is_string && !row.max_length.trim().is_empty() => message(
                self.t,
                "grid-length-from-all",
                &[("length", row.max_length.trim()), path],
            ),
            Some(row)
                if !is_string && !(row.min.trim().is_empty() && row.max.trim().is_empty()) =>
            {
                message(
                    self.t,
                    "grid-limit-from-all",
                    &[("min", row.min.trim()), ("max", row.max.trim()), path],
                )
            }
            _none => message(self.t, "grid-limit-none-from-all", &[path]),
        }
    }

    /// Note the `id` of one input of the leaf path `path`. The first note of a kind stays.
    fn note(&mut self, path: &str, input: LeafInput, id: &str) {
        let ids = self.ids.entry(path.to_owned()).or_default();
        let slot = match input {
            LeafInput::Value => &mut ids.value,
            LeafInput::Limit => &mut ids.limit,
            LeafInput::LimitMax => &mut ids.limit_max,
            LeafInput::LimitMaxLength => &mut ids.limit_max_length,
        };
        slot.get_or_insert_with(|| id.to_owned());
    }
}
