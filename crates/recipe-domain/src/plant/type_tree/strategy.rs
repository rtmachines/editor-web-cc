//! Proptest strategy for random type trees (`reliability.md` section 6).
//!
//! The strategy first draws a `Spec`, a plain description of a tree. It then
//! writes the rows of the spec. The expected counts come from the spec and
//! not from `resolve`, so a test compares two independent computations.

use proptest::prelude::*;

use super::fixtures::{array_row, id, primitive_row, struct_row, types};
use crate::plant::{PlatformTypes, PlcType, PlcTypeId, Primitive};

/// Deepest JSON nesting that the strategy builds.
const MAX_DEPTH: u32 = 6;
/// Most elements in one array dimension.
const MAX_DIMENSION_COUNT: i32 = 3;
/// Most members in one struct.
const MAX_MEMBERS: usize = 4;

const INT: u128 = 1;
const REAL: u128 = 2;
const BOOL: u128 = 3;
/// First identifier that a generated array or struct row uses.
const FIRST_GENERATED_ID: u128 = 100;

/// A generated platform with one root type and the counts that the generator expects.
#[derive(Clone, Debug)]
pub(crate) struct GeneratedTypes {
    /// The rows of the platform.
    pub(crate) types: PlatformTypes,
    /// The root of the tree.
    pub(crate) root: PlcTypeId,
    /// The leaf count that the generator computed on its own.
    pub(crate) expected_leaves: u64,
    /// The depth that the generator computed on its own.
    pub(crate) expected_depth: u32,
}

/// Description of a tree before it becomes rows.
#[derive(Clone, Debug)]
enum Spec {
    /// A seed primitive: the number is the row id of `INT`, `REAL`, or `BOOL`.
    Leaf(u128),
    /// An array with bounds per dimension.
    Array(Vec<(i32, i32)>, Box<Spec>),
    /// A struct with at least one member.
    Struct(Vec<Spec>),
}

impl Spec {
    fn leaves(&self) -> u64 {
        match self {
            Self::Leaf(_) => 1,
            Self::Array(bounds, element) => {
                let count: u64 = bounds
                    .iter()
                    .map(|(lower, upper)| {
                        let width = upper.checked_sub(*lower).unwrap().checked_add(1).unwrap();
                        u64::try_from(width).unwrap_or(0)
                    })
                    .product();
                count.checked_mul(element.leaves()).unwrap()
            }
            Self::Struct(members) => members.iter().map(Self::leaves).sum(),
        }
    }

    fn depth(&self) -> u32 {
        match self {
            Self::Leaf(_) => 0,
            Self::Array(bounds, element) => {
                let dimensions = u32::try_from(bounds.len()).unwrap_or(0);
                element.depth().checked_add(dimensions).unwrap()
            }
            Self::Struct(members) => members
                .iter()
                .map(Self::depth)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .unwrap(),
        }
    }

    /// Write the rows of this spec and return the id of its root row.
    fn emit(&self, next_id: &mut u128, rows: &mut Vec<PlcType>) -> u128 {
        match self {
            Self::Leaf(row_id) => *row_id,
            Self::Array(bounds, element) => {
                let element_id = element.emit(next_id, rows);
                let row_id = take_id(next_id);
                rows.push(array_row(row_id, element_id, bounds));
                row_id
            }
            Self::Struct(members) => {
                let member_ids: Vec<u128> = members
                    .iter()
                    .map(|member| member.emit(next_id, rows))
                    .collect();
                let names: Vec<String> = (0..member_ids.len()).map(|n| format!("m{n}")).collect();
                let pairs: Vec<(&str, u128)> = names
                    .iter()
                    .map(String::as_str)
                    .zip(member_ids.iter().copied())
                    .collect();
                let row_id = take_id(next_id);
                rows.push(struct_row(row_id, &format!("ST_{row_id}"), &pairs));
                row_id
            }
        }
    }

    fn generate(&self) -> GeneratedTypes {
        let mut rows = vec![
            primitive_row(INT, Primitive::Int),
            primitive_row(REAL, Primitive::Real),
            primitive_row(BOOL, Primitive::Bool),
        ];
        let mut next_id = FIRST_GENERATED_ID;
        let root = self.emit(&mut next_id, &mut rows);
        GeneratedTypes {
            types: types(rows),
            root: id(root),
            expected_leaves: self.leaves(),
            expected_depth: self.depth(),
        }
    }
}

fn take_id(next_id: &mut u128) -> u128 {
    let current = *next_id;
    *next_id = current.checked_add(1).unwrap();
    current
}

fn leaf() -> BoxedStrategy<Spec> {
    prop_oneof![Just(INT), Just(REAL), Just(BOOL)]
        .prop_map(Spec::Leaf)
        .boxed()
}

fn dimension() -> impl Strategy<Value = (i32, i32)> {
    (-2..=2_i32, 1..=MAX_DIMENSION_COUNT).prop_map(|(lower, count)| {
        (
            lower,
            lower.checked_add(count).unwrap().checked_sub(1).unwrap(),
        )
    })
}

/// Draw a spec that uses at most `budget` JSON levels.
fn spec(budget: u32) -> BoxedStrategy<Spec> {
    if budget == 0 {
        return leaf();
    }
    let members = prop::collection::vec(spec(budget.checked_sub(1).unwrap()), 1..=MAX_MEMBERS)
        .prop_map(Spec::Struct);
    let mut choices = vec![leaf(), members.boxed()];
    for levels in 1..=budget.min(3) {
        let len = usize::try_from(levels).unwrap_or(1);
        let array = (
            prop::collection::vec(dimension(), len..=len),
            spec(budget.checked_sub(levels).unwrap()),
        )
            .prop_map(|(bounds, element)| Spec::Array(bounds, Box::new(element)));
        choices.push(array.boxed());
    }
    proptest::strategy::Union::new(choices).boxed()
}

/// Strategy for a platform with a random type tree up to depth 6.
///
/// P2.2 reuses this strategy to test the validation of values against trees.
pub(crate) fn type_trees() -> impl Strategy<Value = GeneratedTypes> {
    spec(MAX_DEPTH).prop_map(|spec| spec.generate())
}
