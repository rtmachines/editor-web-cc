//! Property tests of `S7Layout` on random type trees (`reliability.md` section 6).

use proptest::prelude::*;

use super::super::type_tree::fixtures::{id, machine, on_platform, struct_row, types_for};
use super::super::type_tree::strategy::type_trees;
use super::super::{PlatformTypes, PlcPlatform, PlcType, Primitive, TypeNode, TypeTree};
use super::reference::{Row, catalog, rows};
use super::*;

/// Bits in one byte.
const BITS_PER_BYTE: u64 = 8;

fn resolved(
    generated: &super::super::type_tree::strategy::GeneratedTypes,
) -> Result<TypeTree, TestCaseError> {
    TypeTree::resolve(generated.root, &generated.types)
        .map_err(|error| TestCaseError::fail(error.to_string()))
}

fn layout(tree: &TypeTree) -> Result<S7Layout, TestCaseError> {
    S7Layout::of(tree).map_err(|error| TestCaseError::fail(error.to_string()))
}

/// Lower bound of the size: the non-`BOOL` leaves plus the `BOOL` leaves in whole bytes.
///
/// The walk counts each leaf once per element of every array above it.
fn storage_lower_bound(tree: &TypeTree) -> u64 {
    let mut stack = vec![(tree.root(), 1_u64)];
    let mut bytes = 0_u64;
    let mut bools = 0_u64;
    while let Some((node, copies)) = stack.pop() {
        match node {
            TypeNode::Leaf {
                primitive: Primitive::Bool,
                ..
            } => bools = bools.checked_add(copies).unwrap(),
            TypeNode::Leaf { size_bytes, .. } => {
                let leaf_bytes = copies.checked_mul(u64::from(*size_bytes)).unwrap();
                bytes = bytes.checked_add(leaf_bytes).unwrap();
            }
            TypeNode::Array {
                dimensions,
                element,
                ..
            } => {
                let count = dimensions.element_count().unwrap_or(0);
                stack.push((element, copies.checked_mul(count).unwrap()));
            }
            TypeNode::Struct { members, .. } => {
                stack.extend(members.iter().map(|(_, child)| (child, copies)));
            }
        }
    }
    bytes.checked_add(bools.div_ceil(BITS_PER_BYTE)).unwrap()
}

/// Bits in two bytes, the unit of the size rounding.
const BITS_PER_EVEN_BYTES: u64 = 16;

/// Width of the last top-level member in bits: 1 for `BOOL`, else the bytes times 8.
fn member_width(member: &S7Member) -> u64 {
    match member.layout().start() {
        S7Start::Bit => 1,
        S7Start::Byte | S7Start::Even => u64::from(member.layout().size_bytes())
            .checked_mul(BITS_PER_BYTE)
            .unwrap(),
    }
}

/// Start of a member in bits from the start of the root.
fn start_bit(member: &S7Member) -> u64 {
    u64::from(member.byte())
        .checked_mul(BITS_PER_BYTE)
        .and_then(|bits| bits.checked_add(u64::from(member.bit())))
        .unwrap()
}

/// Number of the struct row that holds the drawn members. The shared catalog has no such number.
const HOLDER_ROOT: u128 = 900;

/// Smallest number of drawn members.
const MIN_DRAWN: usize = 1;

/// Largest number of drawn members, the members `m01` to `m16`.
const MAX_DRAWN: usize = 16;

/// Width in bits of a member of the reference table: 1 for `Bit`, else the bytes times 8.
fn reference_width(row: &Row) -> u64 {
    match row.start {
        S7Start::Bit => 1,
        S7Start::Byte | S7Start::Even => u64::from(row.size_bytes)
            .checked_mul(BITS_PER_BYTE)
            .unwrap(),
    }
}

/// First position in bits at or after `cursor` that the start class of the row allows.
///
/// The function states the placement rule of `domain-model.md` section 2.1 once,
/// because that rule is the subject of the property (design D3).
fn first_fit(cursor: u64, row: &Row) -> u64 {
    let unit = match row.start {
        S7Start::Bit => 1,
        S7Start::Byte => BITS_PER_BYTE,
        S7Start::Even => BITS_PER_EVEN_BYTES,
    };
    cursor.checked_next_multiple_of(unit).unwrap()
}

/// The shared catalog and one struct row whose members are the drawn rows, in order.
fn catalog_with_holder(drawn: &[&Row]) -> PlatformTypes {
    let names: Vec<String> = (1..=drawn.len()).map(|n| format!("m{n:02}")).collect();
    let members: Vec<(&str, u128)> = names
        .iter()
        .map(String::as_str)
        .zip(drawn.iter().map(|row| row.root))
        .collect();
    let holder = on_platform(
        PlcPlatform::S7_1500,
        vec![struct_row(HOLDER_ROOT, "ST_Random", &members)],
    );
    let all: Vec<PlcType> = catalog().rows().cloned().chain(holder).collect();
    types_for(machine(), PlcPlatform::S7_1500, all)
}

proptest! {
    #[test]
    fn members_keep_their_reference_layout_in_a_random_struct(
        picks in proptest::collection::vec(
            proptest::sample::select((0..rows().len()).collect::<Vec<_>>()),
            MIN_DRAWN..=MAX_DRAWN,
        )
    ) {
        let table = rows();
        let drawn: Vec<&Row> = picks.iter().map(|index| table.get(*index).unwrap()).collect();
        let tree = TypeTree::resolve(id(HOLDER_ROOT), &catalog_with_holder(&drawn))
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let members = s7_members(&tree)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let top_level: Vec<&S7Member> = members
            .iter()
            .filter(|member| member.names().len() == 1)
            .collect();
        prop_assert_eq!(top_level.len(), drawn.len());
        let mut cursor = 0_u64;
        for (member, row) in top_level.iter().zip(&drawn) {
            prop_assert_eq!(member.layout().size_bytes(), row.size_bytes, "size of {}", row.name);
            prop_assert_eq!(member.layout().start(), row.start, "start class of {}", row.name);
            let expected_start = first_fit(cursor, row);
            prop_assert_eq!(start_bit(member), expected_start, "start of {}", row.name);
            cursor = expected_start.checked_add(reference_width(row)).unwrap();
        }
        let expected_size = cursor
            .checked_next_multiple_of(BITS_PER_EVEN_BYTES)
            .unwrap()
            .checked_div(BITS_PER_BYTE)
            .unwrap();
        prop_assert_eq!(u64::from(layout(&tree)?.size_bytes()), expected_size);
    }

    #[test]
    fn last_top_level_member_end_rounded_up_is_the_size(generated in type_trees()) {
        let tree = resolved(&generated)?;
        if matches!(tree.root(), TypeNode::Struct { .. }) {
            let members = s7_members(&tree)
                .map_err(|error| TestCaseError::fail(error.to_string()))?;
            let last = members.iter().rfind(|member| member.names().len() == 1);
            let end = match last {
                Some(member) => {
                    let start = u64::from(member.byte())
                        .checked_mul(BITS_PER_BYTE)
                        .and_then(|bits| bits.checked_add(u64::from(member.bit())))
                        .unwrap();
                    start.checked_add(member_width(member)).unwrap()
                }
                None => 0,
            };
            let rounded = end.checked_next_multiple_of(BITS_PER_EVEN_BYTES).unwrap();
            prop_assert_eq!(rounded, u64::from(layout(&tree)?.size_bytes()) * BITS_PER_BYTE);
        }
    }

    #[test]
    fn every_row_of_a_struct_child_lies_inside_its_parent(generated in type_trees()) {
        let tree = resolved(&generated)?;
        let members = s7_members(&tree).map_err(|error| TestCaseError::fail(error.to_string()))?;
        for (index, parent) in members.iter().enumerate() {
            let parent_start = start_bit(parent);
            let parent_end = parent_start.checked_add(member_width(parent)).unwrap();
            let children = members
                .iter()
                .skip(index.checked_add(1).unwrap())
                .take_while(|member| member.names().starts_with(parent.names()));
            for child in children {
                let child_start = start_bit(child);
                let child_end = child_start.checked_add(member_width(child)).unwrap();
                prop_assert!(child_start >= parent_start);
                prop_assert!(child_end <= parent_end);
            }
        }
    }

    #[test]
    fn layout_of_a_generated_tree_is_ok(generated in type_trees()) {
        let tree = resolved(&generated)?;
        prop_assert!(S7Layout::of(&tree).is_ok());
    }

    #[test]
    fn struct_and_array_sizes_are_even_and_start_even(generated in type_trees()) {
        let tree = resolved(&generated)?;
        let found = layout(&tree)?;
        if matches!(tree.root(), TypeNode::Leaf { .. }) {
            prop_assert_eq!(tree.leaf_count(), 1);
        } else {
            prop_assert_eq!(found.size_bytes() % 2, 0);
            prop_assert_eq!(found.start(), S7Start::Even);
        }
    }

    #[test]
    fn size_is_at_least_the_storage_of_the_leaves(generated in type_trees()) {
        let tree = resolved(&generated)?;
        let found = layout(&tree)?;
        prop_assert!(u64::from(found.size_bytes()) >= storage_lower_bound(&tree));
    }

    #[test]
    fn leaf_root_has_the_size_of_the_leaf(generated in type_trees()) {
        let tree = resolved(&generated)?;
        if let TypeNode::Leaf { size_bytes, .. } = tree.root() {
            prop_assert_eq!(layout(&tree)?.size_bytes(), *size_bytes);
        }
    }
}

#[test]
fn strategy_reaches_struct_array_and_leaf_roots() {
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::TestRunner;

    let mut runner = TestRunner::deterministic();
    let strategy = type_trees();
    let mut kinds = [0_u32; 3];
    for _ in 0..300 {
        let generated = strategy.new_tree(&mut runner).unwrap().current();
        let tree = TypeTree::resolve(generated.root, &generated.types).unwrap();
        let slot = match tree.root() {
            TypeNode::Leaf { .. } => 0,
            TypeNode::Array { .. } => 1,
            TypeNode::Struct { .. } => 2,
        };
        kinds[slot] += 1;
    }
    assert!(kinds.iter().all(|count| *count > 0), "{kinds:?}");
}
