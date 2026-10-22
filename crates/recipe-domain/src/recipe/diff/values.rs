//! The diff of the values of one parameter (A43, ADR-0039, ADR-0050 (c)).

use std::collections::{HashMap, HashSet};

use super::limits::{LimitSide, bounds, effective_limits};
use super::rows::DiffPart;
use crate::plant::{LeafPath, TypeTrees};
use crate::recipe::{Parameter, ParameterValue};

/// The text of each leaf of a value, in the preorder of the leaves.
///
/// The texts are the texts of [`ParameterValue::texts`]. The use cases require each tree and
/// check each pair with [`DiffPair::check_stored`](super::DiffPair::check_stored) first. A
/// value without a tree gives one leaf at the empty path with its stored JSON text. An
/// unchecked value that does not fit its tree gives the same leaf. The use cases never get this
/// leaf. The diff of unchecked input therefore never hides a change.
pub(super) fn leaf_texts(parameter: &Parameter, trees: &TypeTrees) -> Vec<(LeafPath, String)> {
    let texts = trees.tree(parameter.plc_type()).and_then(|tree| {
        let value = ParameterValue::try_from((parameter.value(), tree)).ok()?;
        value.texts(tree).ok()
    });
    match texts {
        Some(texts) => texts
            .into_iter()
            .map(|leaf| (leaf.path, leaf.text))
            .collect(),
        None => vec![(LeafPath::whole(), parameter.value().as_json().to_string())],
    }
}

/// Return one row for each leaf whose text differs, matched by the leaf path.
///
/// With `is_retyped`, every leaf of both sides gives a row, also a leaf with an equal text.
/// A type change sets `is_retyped` (ADR-0050 (c)). A leaf of one side only has an empty other
/// side. The rows of the new leaves come first in preorder. The leaves of the old side only
/// follow in preorder. Each row of a new leaf holds the limit of the new version that covers it.
pub(super) fn value_rows(
    old: &Parameter,
    new: &Parameter,
    trees: &TypeTrees,
    is_retyped: bool,
) -> Vec<DiffPart> {
    if !is_retyped && old.value() == new.value() {
        return Vec::new();
    }
    let old_texts = leaf_texts(old, trees);
    let new_texts = leaf_texts(new, trees);
    let old_by_path: HashMap<&LeafPath, &str> = old_texts
        .iter()
        .map(|(path, text)| (path, text.as_str()))
        .collect();
    let new_side = LimitSide::of(new, trees);
    let covered = effective_limits(new_side);
    let changed = new_texts.iter().filter_map(|(path, text)| {
        let before = old_by_path.get(path).copied();
        (is_retyped || before != Some(text.as_str())).then(|| DiffPart::Value {
            path: path.clone(),
            old: before.map(str::to_owned),
            new: Some(text.clone()),
            limit: covered
                .get(path)
                .map(|limit| bounds(limit, new_side.tree())),
        })
    });
    let new_paths: HashSet<&LeafPath> = new_texts.iter().map(|(path, _)| path).collect();
    let only_old = old_texts
        .iter()
        .filter(|(path, _)| !new_paths.contains(path))
        .map(|(path, text)| DiffPart::Value {
            path: path.clone(),
            old: Some(text.clone()),
            new: None,
            limit: None,
        });
    changed.chain(only_old).collect()
}

/// Return the row of a parameter of one side only (ADR-0050 (d)).
///
/// A scalar value shows its text. A structured value shows only the count of its leaves.
pub(super) fn parameter_row(parameter: &Parameter, trees: &TypeTrees) -> DiffPart {
    let texts = leaf_texts(parameter, trees);
    let leaves = texts.len();
    let scalar = match texts.as_slice() {
        [(path, text)] if path.is_empty() => Some(text.clone()),
        _ => None,
    };
    DiffPart::Parameter {
        name_en: parameter.name_en().clone(),
        unit: parameter.unit(),
        plc_type: parameter.plc_type(),
        scalar,
        leaves,
    }
}
