//! Generators of values and limits for a resolved type tree (test code).
//!
//! A pool of random numbers drives the generator, so proptest shrinks the pool. The walks use
//! an explicit stack and never recurse.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde_json::{Map, Value};

use crate::plant::{LeafPath, Primitive, TypeNode, TypeTree};
use crate::recipe::limit::{LimitInput, ProcessLimit};
use crate::recipe::value::ParameterValue;
use crate::recipe::value::leaves::value_leaves;

/// Largest absolute value of a generated number.
const NUMBER_SPAN: i32 = 20_000;
/// Largest distance between a generated number and a generated bound.
const BOUND_SPAN: u32 = 5_000;

/// A change that makes a generated value invalid. The number picks the part to change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mutation {
    /// Keep the value valid.
    None,
    /// Give the leaf at this position in tree order the wrong JSON type.
    WrongLeaf(usize),
    /// Add the key `extra` to the struct at this position.
    ExtraKey(usize),
    /// Remove the last element of the array dimension at this position.
    ShortArray(usize),
}

/// The number of parts that a generated value has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Counts {
    pub(super) leaves: usize,
    pub(super) structs: usize,
    pub(super) arrays: usize,
}

/// One part of the value that the generator has not filled yet.
struct Task<'a> {
    slot: &'a mut Value,
    node: &'a TypeNode,
    level: usize,
}

/// Take the next number of the pool. An empty pool gives 0.
fn draw(pool: &mut impl Iterator<Item = i32>) -> i32 {
    pool.next().unwrap_or_default()
}

/// Generate a JSON value for `tree` and apply `mutation`.
pub(super) fn generate_value(
    tree: &TypeTree,
    seeds: &[i32],
    mutation: Mutation,
) -> (Value, Counts) {
    let mut pool = seeds.iter().copied().cycle();
    let mut counts = Counts::default();
    let mut json = Value::Null;
    let mut stack = vec![Task {
        slot: &mut json,
        node: tree.root(),
        level: 0,
    }];
    while let Some(task) = stack.pop() {
        match task.node {
            TypeNode::Leaf { primitive, .. } => {
                let is_wrong = mutation == Mutation::WrongLeaf(counts.leaves);
                *task.slot = leaf_value(*primitive, is_wrong, &mut pool);
                counts.leaves = counts.leaves.saturating_add(1);
            }
            TypeNode::Struct { members, .. } => {
                let mut map = Map::new();
                for (name, _) in members {
                    map.insert(name.as_str().to_owned(), Value::Null);
                }
                if mutation == Mutation::ExtraKey(counts.structs) {
                    map.insert("extra".to_owned(), Value::Null);
                }
                counts.structs = counts.structs.saturating_add(1);
                *task.slot = Value::Object(map);
                let Value::Object(map) = task.slot else {
                    continue;
                };
                let mut slots: BTreeMap<&str, &mut Value> = map
                    .iter_mut()
                    .map(|(key, slot)| (key.as_str(), slot))
                    .collect();
                let mut children = Vec::new();
                for (name, node) in members {
                    if let Some(slot) = slots.remove(name.as_str()) {
                        children.push(Task {
                            slot,
                            node,
                            level: 0,
                        });
                    }
                }
                stack.extend(children.into_iter().rev());
            }
            TypeNode::Array {
                dimensions,
                element,
                ..
            } => {
                let Some(dimension) = dimensions.iter().nth(task.level) else {
                    continue;
                };
                let count = usize::try_from(dimension.count()).unwrap_or_default();
                let mut items = vec![Value::Null; count];
                if mutation == Mutation::ShortArray(counts.arrays) {
                    items.pop();
                }
                counts.arrays = counts.arrays.saturating_add(1);
                *task.slot = Value::Array(items);
                let Value::Array(items) = task.slot else {
                    continue;
                };
                let next_level = task.level.saturating_add(1);
                let (node, level) = if next_level == dimensions.len() {
                    (element.as_ref(), 0)
                } else {
                    (task.node, next_level)
                };
                stack.extend(
                    items
                        .iter_mut()
                        .map(|slot| Task { slot, node, level })
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev(),
                );
            }
        }
    }
    (json, counts)
}

/// Generate the JSON of one leaf. A wrong leaf has the other JSON type.
fn leaf_value(primitive: Primitive, is_wrong: bool, pool: &mut impl Iterator<Item = i32>) -> Value {
    let number = draw(pool);
    let is_bool = primitive == Primitive::Bool;
    match (is_bool, is_wrong) {
        (true, false) => Value::Bool(number % 2 == 0),
        (true, true) => Value::from(1),
        (false, true) => Value::Bool(true),
        (false, false) if primitive == Primitive::Real => {
            let tenth = draw(pool).unsigned_abs() % 10;
            let text = format!("{}.{tenth}", number % NUMBER_SPAN);
            serde_json::from_str(&text).unwrap_or(Value::Null)
        }
        (false, false) => Value::from(number % NUMBER_SPAN),
    }
}

/// Generate at most one limit for each numeric leaf of a parsed value.
///
/// Each limit contains the value of its leaf. The pool picks which bounds a limit has.
pub(super) fn generate_limits(
    parsed: &ParameterValue,
    tree: &TypeTree,
    seeds: &[i32],
) -> Vec<ProcessLimit> {
    let mut pool = seeds.iter().copied().rev().cycle();
    let leaves = value_leaves(parsed.as_json(), tree).unwrap_or_default();
    leaves
        .iter()
        .filter_map(|leaf| {
            let found = decimal_of(leaf.value)?;
            let pick = draw(&mut pool).unsigned_abs() % 4;
            let span = Decimal::from(draw(&mut pool).unsigned_abs() % BOUND_SPAN);
            let min = (pick % 2 == 1).then(|| found.checked_sub(span)).flatten();
            let max = (pick >= 2).then(|| found.checked_add(span)).flatten();
            limit_row(&leaf.path, min, max).and_then(|row| ProcessLimit::parse(&row, tree).ok())
        })
        .collect()
}

/// Build a limit that the first numeric leaf of the value breaks. It returns the leaf path too.
///
/// The upper bound of the limit is one below the value.
pub(super) fn tighten_first_numeric(
    parsed: &ParameterValue,
    tree: &TypeTree,
) -> Option<(ProcessLimit, LeafPath)> {
    let leaves = value_leaves(parsed.as_json(), tree)?;
    let (leaf, found) = leaves
        .iter()
        .find_map(|leaf| decimal_of(leaf.value).map(|found| (leaf, found)))?;
    let max = found.checked_sub(Decimal::ONE)?;
    let row = limit_row(&leaf.path, None, Some(max))?;
    let limit = ProcessLimit::parse(&row, tree).ok()?;
    Some((limit, leaf.path.clone()))
}

/// Read a numeric leaf as a decimal. Other leaves give `None`.
fn decimal_of(value: &Value) -> Option<Decimal> {
    let Value::Number(number) = value else {
        return None;
    };
    Decimal::from_str_exact(&number.to_string()).ok()
}

/// Build the row of a limit, or `None` when it has no bound.
fn limit_row(path: &LeafPath, min: Option<Decimal>, max: Option<Decimal>) -> Option<LimitInput> {
    (min.is_some() || max.is_some()).then(|| LimitInput {
        leaf_path: path.clone(),
        min: min.map(|bound| bound.to_string()),
        max: max.map(|bound| bound.to_string()),
        max_length: None,
    })
}
