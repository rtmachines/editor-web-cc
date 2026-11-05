//! S7 layout of a type tree (`domain-model.md` section 2.1, ADR-0017).

use super::{MemberName, PlantError, PlcTypeId, Primitive, TypeLimit, TypeNode, TypeTree};

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
    /// The composite rows of the reference table in `s7_layout/reference.rs`
    /// stay unverified until the TIA Portal check (Q10).
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-018` when an arithmetic step overflows. The limits of a
    /// resolved tree make this impossible.
    pub fn of(tree: &TypeTree) -> Result<Self, PlantError> {
        let root = tree.root();
        layout_of_node(root).ok_or_else(|| too_large(root))
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

/// One struct member of a type tree with its place in an S7 data block.
///
/// The type solves one problem. The type page shows the offset of every
/// member, and the offset comes from the same rules as the size of the type.
/// The member holds its names from the root and not a `LeafPath`, because a
/// `LeafPath` names one scalar and a member can be a struct or an array.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct S7Member {
    names: Vec<MemberName>,
    type_id: PlcTypeId,
    layout: S7Layout,
    byte: u32,
    bit: u8,
}

impl S7Member {
    /// Return the member names from the root to this member.
    #[must_use]
    pub fn names(&self) -> &[MemberName] {
        &self.names
    }

    /// Return the path as text, for example `.zone.active`.
    #[must_use]
    pub fn path_text(&self) -> String {
        self.names.iter().map(|name| format!(".{name}")).collect()
    }

    /// Return the row of the type of the member.
    #[must_use]
    pub const fn type_id(&self) -> PlcTypeId {
        self.type_id
    }

    /// Return the size and the start class of the member.
    #[must_use]
    pub const fn layout(&self) -> S7Layout {
        self.layout
    }

    /// Return the byte of the offset.
    #[must_use]
    pub const fn byte(&self) -> u32 {
        self.byte
    }

    /// Return the bit of the offset, from 0 to 7.
    #[must_use]
    pub const fn bit(&self) -> u8 {
        self.bit
    }
}

/// Return one row per struct member of the tree, in the order of the tree.
///
/// A nested struct has a row and then the rows of its members. An array is
/// one row, and its elements have no row. A root that is not a struct has no
/// member and gives an empty list.
///
/// # Errors
///
/// Returns `E-PLT-018` when an arithmetic step overflows. The limits of a
/// resolved tree make this impossible.
pub fn s7_members(tree: &TypeTree) -> Result<Vec<S7Member>, PlantError> {
    let root = tree.root();
    members_of_root(root).ok_or_else(|| too_large(root))
}

/// One struct on the current path of the member walk.
struct MemberFrame<'t> {
    members: std::slice::Iter<'t, (MemberName, TypeNode)>,
    /// Bit where the next member of this struct starts to look for a place.
    cursor: u64,
}

fn members_of_root(root: &TypeNode) -> Option<Vec<S7Member>> {
    let TypeNode::Struct { members, .. } = root else {
        return Some(Vec::new());
    };
    let mut rows = Vec::new();
    let mut path: Vec<MemberName> = Vec::new();
    let mut stack = vec![MemberFrame {
        members: members.iter(),
        cursor: 0,
    }];
    while let Some(frame) = stack.last_mut() {
        let Some((name, node)) = frame.members.next() else {
            stack.pop();
            path.pop();
            continue;
        };
        let layout = layout_of_node(node)?;
        let placed = place(frame.cursor, &layout)?;
        frame.cursor = placed.end;
        path.push(name.clone());
        rows.push(S7Member {
            names: path.clone(),
            type_id: node_type_id(node),
            layout,
            byte: u32::try_from(placed.start.checked_div(BITS_PER_BYTE)?).ok()?,
            bit: u8::try_from(placed.start.checked_rem(BITS_PER_BYTE)?).ok()?,
        });
        if let TypeNode::Struct { members, .. } = node {
            stack.push(MemberFrame {
                members: members.iter(),
                cursor: placed.start,
            });
        } else {
            path.pop();
        }
    }
    Some(rows)
}

/// Compute the layout of a node and of every node below it.
///
/// Returns `None` when an arithmetic step overflows.
fn layout_of_node(root: &TypeNode) -> Option<S7Layout> {
    let mut stack = vec![Frame::new(root)];
    while let Some(frame) = stack.last_mut() {
        if let Some(child) = frame.children.next() {
            stack.push(Frame::new(child));
            continue;
        }
        let layout = stack.pop()?.finish()?;
        match stack.last_mut() {
            Some(parent) => parent.done.push(layout),
            None => return Some(layout),
        }
    }
    None
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
        cursor = place(cursor, member)?.end;
    }
    let bytes = cursor
        .checked_next_multiple_of(BITS_PER_EVEN_BYTES)?
        .checked_div(BITS_PER_BYTE)?;
    Some(S7Layout {
        size_bytes: u32::try_from(bytes).ok()?,
        start: S7Start::Even,
    })
}

/// The bit range that one member takes inside a data block.
struct Placement {
    /// First bit of the member, after the alignment of its start class.
    start: u64,
    /// First bit after the member.
    end: u64,
}

/// Place one member at the cursor. The cursor counts bits.
///
/// The step solves one problem. `struct_layout` and `s7_members` share it, so
/// the alignment rules of S7 exist once. Returns `None` on overflow.
fn place(cursor: u64, member: &S7Layout) -> Option<Placement> {
    let start = match member.start {
        S7Start::Bit => cursor,
        S7Start::Byte => cursor.checked_next_multiple_of(BITS_PER_BYTE)?,
        S7Start::Even => cursor.checked_next_multiple_of(BITS_PER_EVEN_BYTES)?,
    };
    let bits = u64::from(member.size_bytes).checked_mul(BITS_PER_BYTE)?;
    let width = match member.start {
        S7Start::Bit => 1,
        S7Start::Byte | S7Start::Even => bits,
    };
    Some(Placement {
        start,
        end: start.checked_add(width)?,
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
mod reference;

#[cfg(test)]
mod tests;
