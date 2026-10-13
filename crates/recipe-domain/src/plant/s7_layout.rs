//! S7 layout of a type tree (`domain-model.md` section 2.1, ADR-0017).

use super::{PlantError, PlcTypeId, Primitive, TypeLimit, TypeNode, TypeTree};

/// Where a value of a type starts inside an S7 data block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum S7Start {
    /// The value starts at any bit. A `BOOL` leaf has this class.
    Bit,
    /// The value starts at any byte. A leaf of one byte has this class.
    Byte,
    /// The value starts at an even byte. Every other type has this class.
    Even,
}

/// The size and the start class of a type tree on S7.
///
/// The type solves one problem. A size is computed from the tree, so no
/// stored size can disagree with the members of a struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct S7Layout {
    size_bytes: u32,
    start: S7Start,
}

impl S7Layout {
    /// Compute the layout of a tree.
    ///
    /// The golden test holds the values of the rules of `domain-model.md`.
    /// The reference table of the Siemens manual (ADR-0017) is not in the
    /// repository yet.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-018` when an arithmetic step overflows. The limits of a
    /// resolved tree make this impossible.
    pub fn of(tree: &TypeTree) -> Result<Self, PlantError> {
        let root = tree.root();
        let mut stack = vec![Frame::new(root)];
        while let Some(frame) = stack.last_mut() {
            if let Some(child) = frame.children.next() {
                stack.push(Frame::new(child));
                continue;
            }
            let Some(finished) = stack.pop() else { break };
            let layout = finished.finish().ok_or_else(|| too_large(root))?;
            match stack.last_mut() {
                Some(parent) => parent.done.push(layout),
                None => return Ok(layout),
            }
        }
        Err(too_large(root))
    }

    /// Return the size of the value in bytes.
    #[must_use]
    pub const fn size_bytes(&self) -> u32 {
        self.size_bytes
    }

    /// Return the start class of the value.
    #[must_use]
    pub const fn start(&self) -> S7Start {
        self.start
    }
}

/// One node on the current path of the walk and the layouts of its children.
struct Frame<'t> {
    node: &'t TypeNode,
    children: std::vec::IntoIter<&'t TypeNode>,
    done: Vec<S7Layout>,
}

impl<'t> Frame<'t> {
    fn new(node: &'t TypeNode) -> Self {
        let children: Vec<&TypeNode> = match node {
            TypeNode::Leaf { .. } => Vec::new(),
            TypeNode::Array { element, .. } => vec![element.as_ref()],
            TypeNode::Struct { members, .. } => members.iter().map(|(_, child)| child).collect(),
        };
        Self {
            node,
            children: children.into_iter(),
            done: Vec::new(),
        }
    }

    /// Compute the layout of the node from the layouts of its children.
    ///
    /// Returns `None` when an arithmetic step overflows.
    fn finish(self) -> Option<S7Layout> {
        match self.node {
            TypeNode::Leaf {
                primitive,
                size_bytes,
                ..
            } => Some(S7Layout {
                size_bytes: *size_bytes,
                start: leaf_start(*primitive, *size_bytes),
            }),
            TypeNode::Array { dimensions, .. } => {
                let count = dimensions.element_count()?;
                array_layout(count, self.done.first()?)
            }
            TypeNode::Struct { .. } => struct_layout(&self.done),
        }
    }
}

/// Return the start class of a leaf.
fn leaf_start(primitive: Primitive, size_bytes: u32) -> S7Start {
    match (primitive, size_bytes) {
        (Primitive::Bool, _) => S7Start::Bit,
        (Primitive::String | Primitive::Wstring, _) => S7Start::Even,
        (_, 1) => S7Start::Byte,
        (_, _) => S7Start::Even,
    }
}

/// Place the members of a struct one after the other. The cursor counts bits.
fn struct_layout(members: &[S7Layout]) -> Option<S7Layout> {
    let mut cursor = 0_u64;
    for member in members {
        cursor = match member.start {
            S7Start::Bit => cursor,
            S7Start::Byte => cursor.checked_next_multiple_of(BITS_PER_BYTE)?,
            S7Start::Even => cursor.checked_next_multiple_of(BITS_PER_EVEN_BYTES)?,
        };
        let bits = u64::from(member.size_bytes).checked_mul(BITS_PER_BYTE)?;
        let width = match member.start {
            S7Start::Bit => 1,
            S7Start::Byte | S7Start::Even => bits,
        };
        cursor = cursor.checked_add(width)?;
    }
    let bytes = cursor
        .checked_next_multiple_of(BITS_PER_EVEN_BYTES)?
        .checked_div(BITS_PER_BYTE)?;
    Some(S7Layout {
        size_bytes: u32::try_from(bytes).ok()?,
        start: S7Start::Even,
    })
}

/// Pack `count` elements of one layout. `BOOL` elements share bytes.
fn array_layout(count: u64, element: &S7Layout) -> Option<S7Layout> {
    let bytes = match element.start {
        S7Start::Bit => count.div_ceil(BITS_PER_BYTE),
        S7Start::Byte => count.checked_mul(u64::from(element.size_bytes))?,
        S7Start::Even => {
            let stride = u64::from(element.size_bytes).checked_next_multiple_of(2)?;
            count.checked_mul(stride)?
        }
    };
    Some(S7Layout {
        size_bytes: u32::try_from(bytes.checked_next_multiple_of(2)?).ok()?,
        start: S7Start::Even,
    })
}

/// Build `E-PLT-018` for an overflow of the layout arithmetic.
fn too_large(root: &TypeNode) -> PlantError {
    PlantError::TypeTooLarge {
        type_name: node_type_id(root).to_string(),
        limit: TypeLimit::Leaves,
    }
}

fn node_type_id(node: &TypeNode) -> PlcTypeId {
    match node {
        TypeNode::Leaf { type_id, .. }
        | TypeNode::Array { type_id, .. }
        | TypeNode::Struct { type_id, .. } => *type_id,
    }
}

/// Number of bits in one byte.
const BITS_PER_BYTE: u64 = 8;
/// Number of bits in two bytes, the unit of an even start.
const BITS_PER_EVEN_BYTES: u64 = 16;

#[cfg(test)]
mod prop;

#[cfg(test)]
mod tests {
    use super::super::type_tree::fixtures::{
        array_row, id, primitive_row, string_row, struct_row, types,
    };
    use super::super::{PlcType, Primitive, TypeTree};
    use super::*;

    const INT: u128 = 1;
    const REAL: u128 = 2;
    const BOOL: u128 = 3;
    const BYTE: u128 = 4;
    const CHAR: u128 = 5;
    const DTL: u128 = 6;
    const LREAL: u128 = 7;
    const STRING_3: u128 = 8;
    const ST_FLAGS: u128 = 9;
    const ROOT: u128 = 100;

    struct Case {
        name: &'static str,
        rows: Vec<PlcType>,
        size_bytes: u32,
        start: S7Start,
    }

    fn case(name: &'static str, rows: Vec<PlcType>, size_bytes: u32, start: S7Start) -> Case {
        Case {
            name,
            rows,
            size_bytes,
            start,
        }
    }

    fn layout_of(rows: Vec<PlcType>) -> Result<S7Layout, PlantError> {
        let mut all = vec![
            primitive_row(INT, Primitive::Int),
            primitive_row(REAL, Primitive::Real),
            primitive_row(BOOL, Primitive::Bool),
            primitive_row(BYTE, Primitive::Byte),
            primitive_row(CHAR, Primitive::Char),
            primitive_row(DTL, Primitive::Dtl),
            primitive_row(LREAL, Primitive::Lreal),
            string_row(STRING_3, 3),
            struct_row(
                ST_FLAGS,
                "ST_Flags",
                &[("a", BOOL), ("b", BOOL), ("c", BOOL)],
            ),
        ];
        all.extend(rows);
        let tree = TypeTree::resolve(id(ROOT), &types(all)).expect("fixture tree resolves");
        S7Layout::of(&tree)
    }

    /// A struct root with the given members, one row per distinct member.
    fn struct_case(name: &'static str, members: &[(&str, u128)], size_bytes: u32) -> Case {
        case(
            name,
            vec![struct_row(ROOT, "ST_Case", members)],
            size_bytes,
            S7Start::Even,
        )
    }

    fn array_case(name: &'static str, bounds: &[(i32, i32)], element: u128, size: u32) -> Case {
        case(
            name,
            vec![array_row(ROOT, element, bounds)],
            size,
            S7Start::Even,
        )
    }

    fn struct_cases() -> Vec<Case> {
        let zone = vec![
            array_row(10, INT, &[(1, 3)]),
            string_row(11, 16),
            struct_row(
                ROOT,
                "ST_Zone",
                &[
                    ("temp_sp", REAL),
                    ("active", BOOL),
                    ("ramps", 10),
                    ("label", 11),
                ],
            ),
        ];
        vec![
            struct_case("ST_Flags", &[("a", BOOL), ("b", BOOL), ("c", BOOL)], 2),
            struct_case(
                "ST_Mix",
                &[("flag", BOOL), ("speed", INT), ("mode", BYTE)],
                6,
            ),
            case("ST_Zone", zone, 30, S7Start::Even),
            struct_case("three bytes", &[("a", BYTE), ("b", BYTE), ("c", BYTE)], 4),
            struct_case("byte then int", &[("a", BYTE), ("b", INT)], 4),
            struct_case(
                "nine bools",
                &[
                    ("a", BOOL),
                    ("b", BOOL),
                    ("c", BOOL),
                    ("d", BOOL),
                    ("e", BOOL),
                    ("f", BOOL),
                    ("g", BOOL),
                    ("h", BOOL),
                    ("i", BOOL),
                ],
                2,
            ),
            struct_case(
                "int, two bools, byte",
                &[("a", INT), ("b", BOOL), ("c", BOOL), ("d", BYTE)],
                4,
            ),
            struct_case("bool then byte", &[("a", BOOL), ("b", BYTE)], 2),
            struct_case("byte then ST_Flags", &[("a", BYTE), ("b", ST_FLAGS)], 4),
            case(
                "bool then bool array",
                vec![
                    array_row(10, BOOL, &[(1, 2)]),
                    struct_row(ROOT, "ST_Case", &[("a", BOOL), ("b", 10)]),
                ],
                4,
                S7Start::Even,
            ),
        ]
    }

    fn array_cases() -> Vec<Case> {
        vec![
            array_case("4 strings of 3", &[(1, 4)], STRING_3, 24),
            array_case("3 bytes", &[(1, 3)], BYTE, 4),
            array_case("8 bools", &[(1, 8)], BOOL, 2),
            array_case("10 bools", &[(1, 10)], BOOL, 2),
            array_case("17 bools", &[(1, 17)], BOOL, 4),
            array_case("2 by 2 ints", &[(0, 1), (1, 2)], INT, 8),
        ]
    }

    fn leaf_cases() -> Vec<Case> {
        vec![
            case("INT root", Vec::new(), 2, S7Start::Even),
            case("BOOL root", Vec::new(), 1, S7Start::Bit),
            case("BYTE root", Vec::new(), 1, S7Start::Byte),
            case("CHAR root", Vec::new(), 1, S7Start::Byte),
            case("DTL root", Vec::new(), 12, S7Start::Even),
            case("LREAL root", Vec::new(), 8, S7Start::Even),
            case("STRING(3) root", Vec::new(), 5, S7Start::Even),
        ]
    }

    #[test]
    fn s7_layout_golden_struct_and_array_sizes() {
        for case in struct_cases().into_iter().chain(array_cases()) {
            let layout = layout_of(case.rows).expect(case.name);
            assert_eq!(layout.size_bytes(), case.size_bytes, "{}", case.name);
            assert_eq!(layout.start(), case.start, "{}", case.name);
        }
    }

    #[test]
    fn s7_layout_golden_leaf_roots_are_not_rounded() {
        let roots = [INT, BOOL, BYTE, CHAR, DTL, LREAL, STRING_3];
        for (case, root) in leaf_cases().into_iter().zip(roots) {
            let mut all = vec![
                primitive_row(INT, Primitive::Int),
                primitive_row(BOOL, Primitive::Bool),
                primitive_row(BYTE, Primitive::Byte),
                primitive_row(CHAR, Primitive::Char),
                primitive_row(DTL, Primitive::Dtl),
                primitive_row(LREAL, Primitive::Lreal),
                string_row(STRING_3, 3),
            ];
            all.extend(case.rows);
            let tree = TypeTree::resolve(id(root), &types(all)).expect("leaf root resolves");
            let layout = S7Layout::of(&tree).expect(case.name);
            assert_eq!(layout.size_bytes(), case.size_bytes, "{}", case.name);
            assert_eq!(layout.start(), case.start, "{}", case.name);
        }
    }
}
