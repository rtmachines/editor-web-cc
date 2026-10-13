//! The part `types[]` of the hashed body (`api.md` sections 6.1 and 6.3).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::plant::TypeNode;

/// Revision of every type that is not a struct.
const PLAIN_REVISION: u32 = 1;

/// One bound pair of an array dimension.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Bounds {
    lower: i32,
    upper: i32,
}

/// One member of a struct entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Member {
    name: String,
    type_id: String,
}

/// One entry of `types[]`.
///
/// The type solves one problem: the presence rule of `api.md` section 6.1. The fields
/// `element_type_id`, `dimensions`, and `members` exist only for their kind and are never null.
/// Every other field is always present.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct TypeEntry {
    id: String,
    name: Option<String>,
    revision: u32,
    kind: &'static str,
    string_length: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    element_type_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<Vec<Bounds>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    members: Option<Vec<Member>>,
}

/// Return the type reference of a node.
///
/// The reference is the primitive name for a primitive without a length. For every other type
/// it is the identifier of the type row.
pub(super) fn type_ref(node: &TypeNode) -> String {
    match node {
        TypeNode::Leaf {
            primitive,
            string_length: None,
            ..
        } => primitive.name().to_owned(),
        other => other.type_id().to_string(),
    }
}

/// Build the entry of a node, or `None` for a primitive without a length.
fn entry_of(node: &TypeNode) -> Option<TypeEntry> {
    let id = node.type_id().to_string();
    match node {
        TypeNode::Leaf {
            primitive,
            string_length: Some(length),
            ..
        } => Some(TypeEntry {
            id,
            name: Some(primitive.name().to_owned()),
            revision: PLAIN_REVISION,
            kind: "Primitive",
            string_length: Some(*length),
            element_type_id: None,
            dimensions: None,
            members: None,
        }),
        TypeNode::Leaf { .. } => None,
        TypeNode::Array {
            dimensions,
            element,
            ..
        } => Some(TypeEntry {
            id,
            name: None,
            revision: PLAIN_REVISION,
            kind: "Array",
            string_length: None,
            element_type_id: Some(type_ref(element)),
            dimensions: Some(
                dimensions
                    .iter()
                    .map(|dimension| Bounds {
                        lower: dimension.lower(),
                        upper: dimension.upper(),
                    })
                    .collect(),
            ),
            members: None,
        }),
        TypeNode::Struct {
            name,
            revision,
            members,
            ..
        } => Some(TypeEntry {
            id,
            name: Some(name.to_string()),
            revision: *revision,
            kind: "Struct",
            string_length: None,
            element_type_id: None,
            dimensions: None,
            members: Some(
                members
                    .iter()
                    .map(|(member, child)| Member {
                        name: member.as_str().to_owned(),
                        type_id: type_ref(child),
                    })
                    .collect(),
            ),
        }),
    }
}

/// Collect the entries of `types[]` for the root nodes of the parameter types.
///
/// The walk uses an explicit stack and never recurses (`reliability.md` section 2.2). Each type
/// appears once. The entries come ordered by `id`, compared as text byte by byte.
pub(super) fn collect_types<'a>(roots: impl IntoIterator<Item = &'a TypeNode>) -> Vec<TypeEntry> {
    let mut entries: BTreeMap<String, TypeEntry> = BTreeMap::new();
    let mut stack: Vec<&TypeNode> = roots.into_iter().collect();
    while let Some(node) = stack.pop() {
        let Some(entry) = entry_of(node) else {
            continue;
        };
        if entries.contains_key(&entry.id) {
            continue;
        }
        match node {
            TypeNode::Leaf { .. } => {}
            TypeNode::Array { element, .. } => stack.push(element),
            TypeNode::Struct { members, .. } => {
                stack.extend(members.iter().map(|(_, child)| child));
            }
        }
        entries.insert(entry.id.clone(), entry);
    }
    entries.into_values().collect()
}

#[cfg(test)]
mod tests;
