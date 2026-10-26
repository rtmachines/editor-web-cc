//! Generators of values and limits for a resolved type tree (test code).
//!
//! A pool of random numbers drives the generator, so proptest shrinks the pool. The walks use
//! an explicit stack and never recurse.

use std::collections::BTreeMap;

use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Map, Number, Value};

use crate::plant::fixtures::{
    array_row, id, machine, on_platform, primitive_row, string_row, struct_row, types_for,
    wstring_row,
};
use crate::plant::{
    ArrayDimensions, LeafClass, LeafPath, MemberName, PlcPlatform, PlcType, Primitive, TimeType,
    TypeNode, TypeTree,
};
use crate::recipe::limit::{LimitInput, ProcessLimit};
use crate::recipe::value::leaves::value_leaves;
use crate::recipe::value::{ParameterValue, StoredValue, numeric_range};

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
                push_struct_members(task.slot, members, mutation, &mut counts, &mut stack);
            }
            TypeNode::Array {
                dimensions,
                element,
                ..
            } => push_array_items(task, dimensions, element, mutation, &mut counts, &mut stack),
        }
    }
    (json, counts)
}

/// Fill the slot of a struct with an object. Push one task for each member in reverse order.
///
/// The mutation `ExtraKey` adds the key `extra` to the object that it selects.
fn push_struct_members<'a>(
    slot: &'a mut Value,
    members: &'a [(MemberName, TypeNode)],
    mutation: Mutation,
    counts: &mut Counts,
    stack: &mut Vec<Task<'a>>,
) {
    let mut map = Map::new();
    for (name, _) in members {
        map.insert(name.as_str().to_owned(), Value::Null);
    }
    if mutation == Mutation::ExtraKey(counts.structs) {
        map.insert("extra".to_owned(), Value::Null);
    }
    counts.structs = counts.structs.saturating_add(1);
    *slot = Value::Object(map);
    let Value::Object(map) = slot else {
        return;
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

/// Fill the slot of an array dimension with a list. Push one task for each item in reverse order.
///
/// The mutation `ShortArray` removes the last item of the list that it selects.
fn push_array_items<'a>(
    task: Task<'a>,
    dimensions: &ArrayDimensions,
    element: &'a TypeNode,
    mutation: Mutation,
    counts: &mut Counts,
    stack: &mut Vec<Task<'a>>,
) {
    let Some(dimension) = dimensions.iter().nth(task.level) else {
        return;
    };
    let count = usize::try_from(dimension.count()).unwrap_or_default();
    let mut items = vec![Value::Null; count];
    if mutation == Mutation::ShortArray(counts.arrays) {
        items.pop();
    }
    counts.arrays = counts.arrays.saturating_add(1);
    *task.slot = Value::Array(items);
    let Value::Array(items) = task.slot else {
        return;
    };
    let next_level = task.level.saturating_add(1);
    let (node, level) = if next_level == dimensions.len() {
        (element, 0)
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

/// Length of the `STRING` and `WSTRING` rows of a mixed tree.
const MIXED_STRING_LENGTH: u16 = 10;
/// Most members of the root struct of a mixed tree.
const MAX_MIXED_MEMBERS: usize = 5;
/// Most elements of an array member, and most members of a nested struct.
const MAX_MIXED_ITEMS: usize = 3;
/// First row number of a generated array or struct row of a mixed tree.
const FIRST_MIXED_ROW: u128 = 100;
/// Nanoseconds in one day. An `LDATE` count is a whole number of days.
const NANOS_PER_DAY: i128 = 86_400_000_000_000;

/// One member of the root struct of a mixed tree, with the stored JSON of each leaf.
#[derive(Clone, Debug)]
pub(super) enum MixedMember {
    /// One primitive leaf.
    Leaf(Primitive, Value),
    /// An array `[1..n]` of one primitive, with one value per element.
    Array(Primitive, Vec<Value>),
    /// A nested struct of primitive leaves.
    Struct(Vec<(Primitive, Value)>),
}

/// A tree with leaves of every class on one platform, and its stored value (test code).
///
/// The spec solves one problem: the round trip of texts sees every primitive of the platform
/// (`data-model.md` section 7). The strategy `type_trees` draws only `INT`, `REAL`, and `BOOL`.
#[derive(Clone, Debug)]
pub(super) struct MixedSpec {
    platform: PlcPlatform,
    members: Vec<MixedMember>,
}

/// Return `true` when the matrix of `data-model.md` section 7 puts the primitive on the platform.
fn exists_on(primitive: Primitive, platform: PlcPlatform) -> bool {
    use PlcPlatform::{CodesysIec, S7_300, S7_1200, S7_1500, TwinCat3};
    let is_iec = matches!(platform, CodesysIec | TwinCat3);
    match primitive {
        Primitive::Bool
        | Primitive::Byte
        | Primitive::Word
        | Primitive::Dword
        | Primitive::Int
        | Primitive::Dint
        | Primitive::Real
        | Primitive::String
        | Primitive::Time
        | Primitive::Date
        | Primitive::TimeOfDay => true,
        Primitive::Lword
        | Primitive::Lint
        | Primitive::Ulint
        | Primitive::Ltime
        | Primitive::Ltod
        | Primitive::Ldt => is_iec || platform == S7_1500,
        Primitive::Sint
        | Primitive::Usint
        | Primitive::Uint
        | Primitive::Udint
        | Primitive::Lreal
        | Primitive::Wstring => platform != S7_300,
        Primitive::Char => !is_iec,
        Primitive::Wchar | Primitive::Dtl => matches!(platform, S7_1200 | S7_1500),
        Primitive::Ldate => is_iec,
        Primitive::DateAndTime => platform != S7_1200,
        Primitive::S5Time => matches!(platform, S7_1500 | S7_300),
    }
}

/// Return the primitives of a platform in the order of `Primitive::ALL`.
fn primitives_on(platform: PlcPlatform) -> Vec<Primitive> {
    Primitive::ALL
        .into_iter()
        .filter(|primitive| exists_on(*primitive, platform))
        .collect()
}

/// Return the row number of a primitive: its position in `Primitive::ALL`, plus one.
fn primitive_row_id(primitive: Primitive) -> u128 {
    let position = Primitive::ALL
        .iter()
        .position(|candidate| *candidate == primitive)
        .unwrap();
    u128::try_from(position).unwrap().checked_add(1).unwrap()
}

/// Write a JSON number from its text. A text that is not a number gives `null`.
fn number_value(text: &str) -> Value {
    text.parse::<Number>().map_or(Value::Null, Value::Number)
}

/// Draw a valid stored leaf of a primitive on a platform.
fn stored_leaf(primitive: Primitive, platform: PlcPlatform) -> BoxedStrategy<Value> {
    match primitive.leaf_class() {
        LeafClass::Bool => any::<bool>().prop_map(Value::Bool).boxed(),
        LeafClass::Numeric => numeric_leaf(primitive),
        LeafClass::String => string_leaf(primitive),
        LeafClass::Char => {
            let highest = if primitive == Primitive::Wchar {
                '\u{FFFF}'
            } else {
                '\u{FF}'
            };
            prop::char::range('\0', highest)
                .prop_map(|c| Value::String(c.to_string()))
                .boxed()
        }
        LeafClass::Duration | LeafClass::Instant => count_leaf(primitive, platform),
    }
}

/// Draw a numeric leaf: an integer with both ends of its range, or a decimal with a scale.
fn numeric_leaf(primitive: Primitive) -> BoxedStrategy<Value> {
    if matches!(primitive, Primitive::Real | Primitive::Lreal) {
        return (any::<i64>(), 0..=12_u32)
            .prop_map(|(mantissa, scale)| number_value(&Decimal::new(mantissa, scale).to_string()))
            .boxed();
    }
    let (low, high) = numeric_range(primitive).unwrap();
    let (low, high) = (low.to_i128().unwrap(), high.to_i128().unwrap());
    prop_oneof![Just(low), Just(high), low..=high]
        .prop_map(|count| number_value(&count.to_string()))
        .boxed()
}

/// Draw a `STRING` of Latin-1 characters, or a `WSTRING` of any characters.
///
/// A `WSTRING` holds at most half its length in characters, because a character outside the
/// BMP takes two UTF-16 units.
fn string_leaf(primitive: Primitive) -> BoxedStrategy<Value> {
    let length = usize::from(MIXED_STRING_LENGTH);
    let characters = if primitive == Primitive::Wstring {
        prop::collection::vec(any::<char>(), 0..=length.checked_div(2).unwrap()).boxed()
    } else {
        prop::collection::vec(prop::char::range('\0', '\u{FF}'), 0..=length).boxed()
    };
    characters
        .prop_map(|text| Value::String(text.into_iter().collect()))
        .boxed()
}

/// Draw the stored count of a time leaf inside the range of its type on the platform.
fn count_leaf(primitive: Primitive, platform: PlcPlatform) -> BoxedStrategy<Value> {
    let range = TimeType::of(primitive, platform).and_then(TimeType::range);
    let (low, high) = range.unwrap();
    let counts = match primitive {
        Primitive::S5Time => s5time_counts(),
        Primitive::Ldate => (0..=high.checked_div(NANOS_PER_DAY).unwrap())
            .prop_map(|days| days.checked_mul(NANOS_PER_DAY).unwrap())
            .boxed(),
        _ => prop_oneof![Just(low), Just(high), low..=high].boxed(),
    };
    counts
        .prop_map(|count| number_value(&count.to_string()))
        .boxed()
}

/// Draw an S5TIME count in milliseconds on one of its steps of 10, 100, 1 000, or 10 000 ms.
fn s5time_counts() -> BoxedStrategy<i128> {
    let on_step = |step: i128| move |n: i128| n.checked_mul(step).unwrap();
    prop_oneof![
        (0..=999_i128).prop_map(on_step(10)),
        (100..=999_i128).prop_map(on_step(100)),
        (100..=999_i128).prop_map(on_step(1_000)),
        (100..=999_i128).prop_map(on_step(10_000)),
    ]
    .boxed()
}

/// Draw one member of the root struct with primitives of the platform.
fn mixed_member(platform: PlcPlatform) -> BoxedStrategy<MixedMember> {
    let primitives = primitives_on(platform);
    let leaf = move |primitive: Primitive| {
        stored_leaf(primitive, platform).prop_map(move |value| (primitive, value))
    };
    let single = prop::sample::select(primitives.clone())
        .prop_flat_map(leaf)
        .prop_map(|(primitive, value)| MixedMember::Leaf(primitive, value));
    let array = prop::sample::select(primitives.clone()).prop_flat_map(move |primitive| {
        prop::collection::vec(stored_leaf(primitive, platform), 1..=MAX_MIXED_ITEMS)
            .prop_map(move |values| MixedMember::Array(primitive, values))
    });
    let nested = prop::collection::vec(
        prop::sample::select(primitives).prop_flat_map(leaf),
        1..=MAX_MIXED_ITEMS,
    )
    .prop_map(MixedMember::Struct);
    prop_oneof![single, array, nested].boxed()
}

/// Strategy for a platform and a struct tree with leaves of every class of that platform.
pub(super) fn mixed_specs() -> impl Strategy<Value = MixedSpec> {
    prop::sample::select(PlcPlatform::ALL.to_vec()).prop_flat_map(|platform| {
        prop::collection::vec(mixed_member(platform), 1..=MAX_MIXED_MEMBERS)
            .prop_map(move |members| MixedSpec { platform, members })
    })
}

/// Take the next free row number.
fn take_row(next: &mut u128) -> u128 {
    let current = *next;
    *next = current.checked_add(1).unwrap();
    current
}

/// Write the rows of every primitive of the platform. A string row has the mixed length.
fn primitive_rows(platform: PlcPlatform) -> Vec<PlcType> {
    primitives_on(platform)
        .into_iter()
        .map(|primitive| {
            let row = primitive_row_id(primitive);
            match primitive {
                Primitive::String => string_row(row, MIXED_STRING_LENGTH),
                Primitive::Wstring => wstring_row(row, MIXED_STRING_LENGTH),
                _ => primitive_row(row, primitive),
            }
        })
        .collect()
}

/// Write the rows of one member. Return the row number of its type and its stored JSON.
fn member_rows(member: &MixedMember, next: &mut u128, rows: &mut Vec<PlcType>) -> (u128, Value) {
    match member {
        MixedMember::Leaf(primitive, value) => (primitive_row_id(*primitive), value.clone()),
        MixedMember::Array(primitive, values) => {
            let row = take_row(next);
            let upper = i32::try_from(values.len()).unwrap();
            rows.push(array_row(row, primitive_row_id(*primitive), &[(1, upper)]));
            (row, Value::Array(values.clone()))
        }
        MixedMember::Struct(leaves) => {
            let row = take_row(next);
            let names: Vec<String> = (0..leaves.len()).map(|n| format!("n{n}")).collect();
            let types = leaves
                .iter()
                .map(|(primitive, _)| primitive_row_id(*primitive));
            let pairs: Vec<(&str, u128)> = names.iter().map(String::as_str).zip(types).collect();
            rows.push(struct_row(row, &format!("ST_{row}"), &pairs));
            let values = leaves.iter().map(|(_, value)| value.clone());
            (row, Value::Object(names.into_iter().zip(values).collect()))
        }
    }
}

impl MixedSpec {
    /// Write the rows of the spec on its platform and resolve the tree.
    ///
    /// Return the tree and the stored form of the value, or the text of the resolve error.
    pub(super) fn build(&self) -> Result<(TypeTree, StoredValue), String> {
        let mut rows = primitive_rows(self.platform);
        let mut next = FIRST_MIXED_ROW;
        let mut names = Vec::new();
        let mut json = Map::new();
        for (position, member) in self.members.iter().enumerate() {
            let (row, value) = member_rows(member, &mut next, &mut rows);
            let name = format!("m{position}");
            json.insert(name.clone(), value);
            names.push((name, row));
        }
        let root = take_row(&mut next);
        let pairs: Vec<(&str, u128)> = names.iter().map(|(n, row)| (n.as_str(), *row)).collect();
        rows.push(struct_row(root, &format!("ST_{root}"), &pairs));
        let types = types_for(machine(), self.platform, on_platform(self.platform, rows));
        let tree = TypeTree::resolve(id(root), &types).map_err(|error| error.to_string())?;
        Ok((tree, StoredValue::from(Value::Object(json))))
    }

    /// Return every primitive that the spec uses.
    pub(super) fn primitives(&self) -> Vec<Primitive> {
        self.members
            .iter()
            .flat_map(|member| match member {
                MixedMember::Leaf(primitive, _) | MixedMember::Array(primitive, _) => {
                    vec![*primitive]
                }
                MixedMember::Struct(leaves) => leaves.iter().map(|(p, _)| *p).collect(),
            })
            .collect()
    }
}
