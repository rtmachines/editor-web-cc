//! The leaf grid of a parameter: one row per leaf with its value and its limits (design D8,
//! ADR-0039 (j) to (n)).
//!
//! The module solves one problem: askama refuses a recursive macro, so a template cannot walk a
//! type tree. [`GridView::build`] walks the tree once and returns a flat list of rows in preorder.
//! The macros of `components/grid.html` write each row. The view also gives the `id` of the input
//! of each fault, so each error line stands under its input (design D9, ADR-0039 (n)).
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "the parameter handlers of wave 5 call it")
)]

use std::collections::{BTreeMap, BTreeSet};

use askama::Template;
use recipe_domain::plant::{LeafPath, TypeTree};
use recipe_domain::recipe::RecipeError;

use crate::error::{FormErrors, leaf_of};
use crate::pages::Texts;

#[cfg(test)]
mod fault_tests;
#[cfg(test)]
mod index_level_tests;
#[cfg(test)]
mod limit_tests;
mod limits;
mod texts;
mod walk;

pub(crate) use limits::{LimitTexts, ValueTexts};

/// What a grid shows: the tree, the texts of the inputs, and the pressed mode button.
pub(crate) struct GridSource<'a> {
    /// The type tree of the shown value.
    pub(crate) tree: &'a TypeTree,
    /// The text of each value input: the stored texts or the sent texts.
    pub(crate) values: ValueTexts,
    /// The limit rows: the stored limits or the sent rows.
    pub(crate) limits: Vec<LimitTexts>,
    /// The array level of the pressed mode button (`limit_mode`), if any (ADR-0039 (k)).
    pub(crate) toggle: Option<&'a LeafPath>,
    /// The number of elements in one group (`form.array_chunk`). Zero gives no group.
    pub(crate) chunk: usize,
    /// The text of the row of a primitive parameter, whose leaf path is empty: the tag.
    pub(crate) root_label: String,
}

/// The leaf grid of one parameter (design D8).
pub(crate) struct GridView {
    /// The hints above the grid, one per class of leaves, in the order of the first leaf.
    pub(crate) hints: Vec<GridHint>,
    /// The bodies of the table. A group of a large array is one body (ADR-0039 (l)).
    pub(crate) bodies: Vec<GridBody>,
    /// The `id` of the inputs of each leaf path, by the text of the path.
    ids: BTreeMap<String, LeafIds>,
}

/// The kind of input that a fault of a leaf names (ADR-0039 (n)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LeafInput {
    /// The value input of the leaf.
    Value,
    /// The first limit input of the limit row of the leaf: the minimum or the maximum length.
    Limit,
}

/// The `id` of the value input and of the first limit input of one leaf path.
#[derive(Default)]
struct LeafIds {
    value: Option<String>,
    limit: Option<String>,
}

/// One hint above the grid. Each input of its class names it in `aria-describedby`.
pub(crate) struct GridHint {
    /// The fixed `id` of the hint, for example `grid-hint-integer`.
    pub(crate) id: String,
    /// The sentence of the hint in the UI language.
    pub(crate) text: String,
}

/// One `tbody` of the grid.
pub(crate) struct GridBody {
    /// The `id` of a group, for example `grid-group-0`. Empty for a body that is not a group.
    pub(crate) id: String,
    /// True when the group is closed. An input in a closed group still submits.
    pub(crate) hidden: bool,
    /// The rows of the body in preorder.
    pub(crate) rows: Vec<GridRow>,
}

/// One row of the grid. A leaf row is boxed, so each row stays small (`clippy::large_enum_variant`).
pub(crate) enum GridRow {
    /// The row of a leaf or a row `[*]` with the limit inputs of all elements.
    Leaf(Box<LeafRow>),
    /// The row with the mode button of an array level.
    Mode(ModeRow),
    /// The row with the toggle button of a group.
    Toggle(ToggleRow),
}

impl GridRow {
    /// Return the leaf row, or `None` for another kind.
    pub(crate) fn leaf(&self) -> Option<&LeafRow> {
        match self {
            Self::Leaf(row) => Some(row.as_ref()),
            Self::Mode(_) | Self::Toggle(_) => None,
        }
    }

    /// Return the mode row, or `None` for another kind.
    pub(crate) const fn mode(&self) -> Option<&ModeRow> {
        match self {
            Self::Mode(row) => Some(row),
            Self::Leaf(_) | Self::Toggle(_) => None,
        }
    }

    /// Return the toggle row, or `None` for another kind.
    pub(crate) const fn toggle(&self) -> Option<&ToggleRow> {
        match self {
            Self::Toggle(row) => Some(row),
            Self::Leaf(_) | Self::Mode(_) => None,
        }
    }
}

/// The row of one leaf, or a row `[*]` that holds the limit of every element of a level.
pub(crate) struct LeafRow {
    /// The `id` of the row header, for example `grid-row-3`.
    pub(crate) id: String,
    /// The leaf path that the row header shows. The row of a primitive shows the root label.
    pub(crate) path: String,
    /// The type of the leaf, for example `STRING(20)`.
    pub(crate) type_text: String,
    /// The cell "every element" of a row `[*]`. Its inputs name it in `aria-labelledby`.
    pub(crate) all: Option<AllCell>,
    /// The value input. A row `[*]` has none.
    pub(crate) value: Option<ValueCell>,
    /// The hidden field `limits[i].leaf_path` of a row with limit inputs.
    pub(crate) limit_path: Option<HiddenField>,
    /// The input of the minimum of a numeric or time leaf.
    pub(crate) min: Option<InputCell>,
    /// The input of the maximum of a numeric or time leaf.
    pub(crate) max: Option<InputCell>,
    /// The input of the maximum length of a string leaf.
    pub(crate) max_length: Option<InputCell>,
    /// The limit of the row `[*]` as text, for an element row of the mode "one limit for all".
    pub(crate) from_all: String,
}

impl LeafRow {
    /// Return the `id` of each input of the row.
    fn input_ids(&self) -> impl Iterator<Item = &str> {
        let value = self.value.as_ref().map(|cell| cell.id.as_str());
        let limits = [&self.min, &self.max, &self.max_length]
            .into_iter()
            .filter_map(|cell| cell.as_ref().map(|cell| cell.id.as_str()));
        value.into_iter().chain(limits)
    }
}

/// The cell "every element" of a row `[*]`.
pub(crate) struct AllCell {
    /// The `id` of the cell.
    pub(crate) id: String,
    /// The words "every element" with the index range of the level.
    pub(crate) text: String,
}

/// The value input of a leaf (ADR-0039 (a)).
pub(crate) struct ValueCell {
    /// The `id` `value-<n>`. `n` is the index of the leaf in preorder.
    pub(crate) id: String,
    /// The field name: `value` and the leaf path.
    pub(crate) name: String,
    /// The text of the input.
    pub(crate) text: String,
    /// True for a `BOOL` leaf. It shows a select with `true` and `false`.
    pub(crate) choice: bool,
    /// The length of a string type. Zero gives no attribute `maxlength`.
    pub(crate) maxlength: u16,
    /// The `id` of the hint of the class of the leaf. Empty without a hint.
    pub(crate) hint: String,
}

/// One limit input of a row.
pub(crate) struct InputCell {
    /// The `id`, for example `limits-0-min`.
    pub(crate) id: String,
    /// The field name, for example `limits[0].min`.
    pub(crate) name: String,
    /// The text of the input.
    pub(crate) text: String,
    /// The `id` of the hint of the class of the leaf, for a bound. Empty for a maximum length.
    pub(crate) hint: String,
}

/// A hidden field of a row.
pub(crate) struct HiddenField {
    /// The field name, for example `limits[0].leaf_path`.
    pub(crate) name: String,
    /// The value of the field.
    pub(crate) value: String,
}

/// The row with the mode button of one array level (ADR-0039 (k)).
pub(crate) struct ModeRow {
    /// The fixed `id` of the button, for example `grid-mode-0`.
    pub(crate) id: String,
    /// The leaf path of the level, for example `.zones[*]`. The button sends it as `limit_mode`.
    pub(crate) level: String,
    /// The label of the button.
    pub(crate) label: String,
    /// The sentence that names the current mode.
    pub(crate) current: String,
    /// The notice after a switch back with different element limits. Empty otherwise.
    pub(crate) notice: String,
}

/// The row with the toggle button of a group (ADR-0039 (l)).
pub(crate) struct ToggleRow {
    /// The `id` of the group that the button controls.
    pub(crate) group: String,
    /// True when the group is open.
    pub(crate) expanded: bool,
    /// The label: the index range and the level.
    pub(crate) text: String,
}

impl GridView {
    /// Build the grid of a tree in the language of `t` (design D8).
    pub(crate) fn build(source: GridSource<'_>, t: Texts<'_>) -> Self {
        walk::build(source, t)
    }

    /// Return the `id` of one input of a leaf path, or `None` when the grid has no such input.
    ///
    /// `LeafInput::Value` gives the value input `value-<n>`. `LeafInput::Limit` gives the first
    /// limit input of the row that holds the limit of the path. In the mode "one limit for all
    /// elements", that row is the row `[*]`.
    pub(crate) fn input_id(&self, path: &LeafPath, input: LeafInput) -> Option<String> {
        let ids = self.ids.get(&path.to_string())?;
        match input {
            LeafInput::Value => ids.value.clone(),
            LeafInput::Limit => ids.limit.clone(),
        }
    }

    /// Return the `id` of the input of a fault of a leaf, or `None` (design D9, ADR-0039 (n)).
    ///
    /// A fault of the value (`E-RCP-004`, `E-RCP-005`, `E-RCP-028`) names the value input. Any
    /// other fault of a leaf names the first limit input of its limit row.
    pub(crate) fn input_of(&self, error: &RecipeError) -> Option<String> {
        let path = leaf_of(error)?;
        let input = match error {
            RecipeError::ValueOutOfLimit { .. }
            | RecipeError::ValueShapeMismatch { .. }
            | RecipeError::RebindNeedsValue { .. } => LeafInput::Value,
            _limit => LeafInput::Limit,
        };
        self.input_id(path, input)
    }

    /// Open each group that holds an input with an error line (ADR-0039 (l), ADR-0041 (e)).
    #[must_use]
    pub(crate) fn with_errors(self, errors: &FormErrors) -> Self {
        let open: BTreeSet<String> = self
            .bodies
            .iter()
            .filter(|body| !body.id.is_empty())
            .filter(|body| {
                body.rows
                    .iter()
                    .filter_map(GridRow::leaf)
                    .flat_map(LeafRow::input_ids)
                    .any(|id| !errors.lines(id).is_empty())
            })
            .map(|body| body.id.clone())
            .collect();
        let bodies = self
            .bodies
            .into_iter()
            .map(|body| GridBody {
                hidden: body.hidden && !open.contains(&body.id),
                rows: body
                    .rows
                    .into_iter()
                    .map(|row| opened(row, &open))
                    .collect(),
                id: body.id,
            })
            .collect();
        Self { bodies, ..self }
    }
}

/// The read-only grid of a value and its limits, the answer of `SetParameterValue` (design D7).
#[derive(Template)]
#[template(path = "grid-values.html")]
pub(crate) struct GridValuesFragment<'a> {
    /// The texts of the UI language.
    pub(crate) t: Texts<'a>,
    /// The grid. The fragment shows its texts and no input.
    pub(crate) view: &'a GridView,
}

/// The read-only rows of the limits, the answer of `GetLimits` and `ReplaceLimits` (design D7).
#[derive(Template)]
#[template(path = "grid-limits.html")]
pub(crate) struct GridLimitsFragment<'a> {
    /// The texts of the UI language.
    pub(crate) t: Texts<'a>,
    /// One row per limit row of the parameter. A row `[*]` holds the limit of every element.
    pub(crate) rows: &'a [LimitTexts],
    /// The text of the row with the empty leaf path: the tag of the parameter.
    pub(crate) root_label: String,
}

/// Return the row with its toggle expanded when its group is in `open`.
fn opened(row: GridRow, open: &BTreeSet<String>) -> GridRow {
    match row {
        GridRow::Toggle(toggle) => GridRow::Toggle(ToggleRow {
            expanded: toggle.expanded || open.contains(&toggle.group),
            ..toggle
        }),
        other => other,
    }
}
