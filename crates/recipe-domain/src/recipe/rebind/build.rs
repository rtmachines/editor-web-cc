//! Build and visit JSON values along a type tree without recursion (design D13).

use serde_json::{Map, Value};

use crate::plant::{ArrayDimensions, LeafSegment, MemberName, TypeNode, extended, index_tuples};
use crate::recipe::value::plc_index;

/// One step of the bottom-up build.
enum Step<'t> {
    /// Build the value of a node.
    Visit(&'t TypeNode, Vec<LeafSegment>),
    /// Join the last values into the object of a struct.
    CloseObject(&'t [(MemberName, TypeNode)]),
    /// Join the last values into the nested arrays of an array node.
    CloseArray(&'t ArrayDimensions),
}

/// Group a flat list of values into arrays of `size` values.
fn group(items: Vec<Value>, size: usize) -> Vec<Value> {
    let mut groups = Vec::new();
    let mut rest = items.into_iter();
    loop {
        let chunk: Vec<Value> = rest.by_ref().take(size).collect();
        if chunk.is_empty() {
            return groups;
        }
        groups.push(Value::Array(chunk));
    }
}

/// Build the value of `node` with the exact shape of its type.
///
/// A struct gives an object with its member names. An array gives nested arrays with the exact
/// element counts. `leaf` gives the value of each scalar leaf. It gets the concrete path of the
/// leaf, which starts with `path`. The function uses an explicit stack. A leaf comes in
/// declaration order, and array elements come in row-major order.
pub(super) fn build_value(
    node: &TypeNode,
    path: Vec<LeafSegment>,
    leaf: &mut impl FnMut(&[LeafSegment], &TypeNode) -> Value,
) -> Value {
    let mut steps = vec![Step::Visit(node, path)];
    let mut done: Vec<Value> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Visit(visited @ TypeNode::Leaf { .. }, at) => done.push(leaf(&at, visited)),
            Step::Visit(TypeNode::Struct { members, .. }, at) => {
                steps.push(Step::CloseObject(members));
                for (name, child) in members.iter().rev() {
                    let child_path = extended(&at, LeafSegment::Member(name.clone()));
                    steps.push(Step::Visit(child, child_path));
                }
            }
            Step::Visit(
                TypeNode::Array {
                    dimensions,
                    element,
                    ..
                },
                at,
            ) => {
                steps.push(Step::CloseArray(dimensions));
                for tuple in index_tuples(dimensions).into_iter().rev() {
                    let child_path = extended(&at, LeafSegment::Index(tuple));
                    steps.push(Step::Visit(element, child_path));
                }
            }
            Step::CloseObject(members) => close_object(&mut done, members),
            Step::CloseArray(dimensions) => close_array(&mut done, dimensions),
        }
    }
    done.pop().unwrap_or(Value::Null)
}

/// Join the last values of `done` into the object of a struct. A short list stays as it is.
fn close_object(done: &mut Vec<Value>, members: &[(MemberName, TypeNode)]) {
    let Some(start) = done.len().checked_sub(members.len()) else {
        return;
    };
    let object: Map<String, Value> = members
        .iter()
        .zip(done.split_off(start))
        .map(|((name, _), value)| (name.as_str().to_owned(), value))
        .collect();
    done.push(Value::Object(object));
}

/// Join the last values of `done` into the nested arrays of an array node. A short list stays.
fn close_array(done: &mut Vec<Value>, dimensions: &ArrayDimensions) {
    let Some(total) = dimensions
        .element_count()
        .and_then(|count| usize::try_from(count).ok())
    else {
        return;
    };
    let Some(start) = done.len().checked_sub(total) else {
        return;
    };
    let mut items = done.split_off(start);
    for dimension in dimensions.iter().rev() {
        let size = usize::try_from(dimension.count()).unwrap_or(usize::MAX);
        items = group(items, size);
    }
    done.extend(items);
}

/// One array level that the visit has not entered yet.
struct Cursor<'v> {
    slot: &'v mut Value,
    /// Position of the array node in the chain.
    array: usize,
    /// Number of dimensions of that array that the visit has passed.
    level: usize,
    /// PLC indexes of the dimensions that the visit has passed in that array.
    indexes: Vec<i32>,
    /// Index segments of the arrays above.
    prefix: Vec<LeafSegment>,
}

/// Visit each element at the end of an array chain, in row-major order.
///
/// `chain` holds the dimensions of each array node, outermost first. A chain without arrays
/// visits the value itself. `visit` gets the element and the index segments of its path.
/// The function returns `None` when the value does not have the shape of the chain.
pub(super) fn visit_elements(
    root: &mut Value,
    chain: &[&ArrayDimensions],
    visit: &mut impl FnMut(&mut Value, &[LeafSegment]),
) -> Option<()> {
    let mut stack = vec![Cursor {
        slot: root,
        array: 0,
        level: 0,
        indexes: Vec::new(),
        prefix: Vec::new(),
    }];
    while let Some(cursor) = stack.pop() {
        let Some(dimensions) = chain.get(cursor.array) else {
            visit(cursor.slot, &cursor.prefix);
            continue;
        };
        let dimension = dimensions.iter().nth(cursor.level)?;
        let Value::Array(items) = cursor.slot else {
            return None;
        };
        if u32::try_from(items.len()) != Ok(dimension.count()) {
            return None;
        }
        let next_level = cursor.level.checked_add(1)?;
        let is_last = next_level == dimensions.len();
        let mut children = Vec::with_capacity(items.len());
        for (position, item) in items.iter_mut().enumerate() {
            let mut indexes = cursor.indexes.clone();
            indexes.push(plc_index(dimension.lower(), position)?);
            children.push(if is_last {
                Cursor {
                    slot: item,
                    array: cursor.array.checked_add(1)?,
                    level: 0,
                    indexes: Vec::new(),
                    prefix: extended(&cursor.prefix, LeafSegment::Index(indexes)),
                }
            } else {
                Cursor {
                    slot: item,
                    array: cursor.array,
                    level: next_level,
                    indexes,
                    prefix: cursor.prefix.clone(),
                }
            });
        }
        children.reverse();
        stack.extend(children);
    }
    Some(())
}
