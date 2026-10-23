//! The part `parameters[]` of the hashed body (`api.md` sections 6.1 and 6.3).

use rust_decimal::Decimal;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};

use super::type_reference::TypeReference;
use super::types::type_ref;
use crate::plant::{MemberName, S7Address, TypeNode, UnitPath};
use crate::recipe::{Parameter, ParameterKey, ParameterTag, StoredLimit};

/// One limit of a parameter. A bound is canonical decimal text, or `None`.
///
/// The order of the fields is part of the contract of `api.md` section 6.1. The document writer
/// uses this `Serialize` and writes the fields in this order. Do not change the order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LimitEntry {
    leaf_path: String,
    min: Option<String>,
    max: Option<String>,
    max_length: Option<String>,
}

/// One entry of `parameters[]`.
///
/// The type solves one problem: the hashed body holds the fields marked "In hash" and nothing
/// else. The entry has no name and no ordinal, so a rename cannot change the hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ParameterEntry {
    #[serde(serialize_with = "super::display_text")]
    unit_path: UnitPath,
    #[serde(serialize_with = "super::display_text")]
    tag: ParameterTag,
    #[serde(serialize_with = "super::display_text")]
    key: ParameterKey,
    type_id: TypeReference,
    #[serde(serialize_with = "super::optional_display_text")]
    s7_address: Option<S7Address>,
    limits: Vec<LimitEntry>,
    value: OrderedValue,
}

impl ParameterEntry {
    /// Build the entry of a parameter.
    ///
    /// `unit` is the path of the unit of the parameter. `root` is the root node of its type.
    pub(super) fn of(parameter: &Parameter, unit: &UnitPath, root: &TypeNode) -> Self {
        Self {
            unit_path: unit.clone(),
            tag: parameter.tag().clone(),
            key: parameter.key(),
            type_id: type_ref(root),
            s7_address: parameter.s7_address(),
            limits: limit_entries(parameter.limits()),
            value: ordered_value(parameter.value().as_json(), root),
        }
    }

    /// Return the path of the unit of the parameter, the value of `unit_path`.
    #[must_use]
    pub const fn unit_path(&self) -> &UnitPath {
        &self.unit_path
    }

    /// Return the tag of the parameter, the value of `tag`.
    #[must_use]
    pub const fn tag(&self) -> &ParameterTag {
        &self.tag
    }

    /// Return the key of the parameter.
    #[must_use]
    pub const fn key(&self) -> ParameterKey {
        self.key
    }

    /// Return the type reference of the parameter, the value of `type_id`.
    #[must_use]
    pub const fn type_id(&self) -> TypeReference {
        self.type_id
    }

    /// Return the S7 address, the value of `s7_address`, or `None` when the parameter has no address.
    #[must_use]
    pub const fn s7_address(&self) -> Option<S7Address> {
        self.s7_address
    }

    /// Return the limit entries, ordered by `leaf_path`.
    #[must_use]
    pub fn limits(&self) -> &[LimitEntry] {
        &self.limits
    }

    /// Return the value, with the members of each struct in the order of `members[]`.
    #[must_use]
    pub const fn value(&self) -> &OrderedValue {
        &self.value
    }
}

/// Write a bound as canonical decimal text.
fn decimal_text(bound: Decimal) -> String {
    bound.normalize().to_string()
}

/// Build the limit entries ordered by `leaf_path`, compared as text byte by byte.
fn limit_entries(limits: &[StoredLimit]) -> Vec<LimitEntry> {
    let mut entries: Vec<LimitEntry> = limits
        .iter()
        .map(|limit| LimitEntry {
            leaf_path: limit.leaf_path.to_string(),
            min: limit.min.map(decimal_text),
            max: limit.max.map(decimal_text),
            max_length: limit.max_length.map(|length| length.to_string()),
        })
        .collect();
    entries.sort_by(|a, b| a.leaf_path.cmp(&b.leaf_path));
    entries
}

/// The value of a parameter with the members of each struct in the order of `members[]`.
///
/// The type solves one problem: a `serde_json::Value` map writes its keys in byte order, but
/// the document writes the members of a struct in their declaration order (`api.md` section
/// 6.2). A numeric leaf is its canonical decimal text (`reliability.md` section 4.4). A
/// boolean and a string stay as they are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrderedValue {
    /// A JSON null.
    Null,
    /// A boolean leaf.
    Bool(bool),
    /// A string leaf, or the canonical decimal text of a numeric leaf.
    Text(String),
    /// An array. The elements keep their stored order.
    List(Vec<OrderedValue>),
    /// A struct. The members come in the order of `members[]`, then the other keys in byte order.
    Members(Vec<(String, OrderedValue)>),
}

impl Serialize for OrderedValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(flag) => serializer.serialize_bool(*flag),
            Self::Text(text) => serializer.serialize_str(text),
            Self::List(items) => serializer.collect_seq(items),
            Self::Members(members) => {
                serializer.collect_map(members.iter().map(|(name, value)| (name, value)))
            }
        }
    }
}

/// One stored value with the type node that applies to it.
///
/// `node` is `None` below a key that `members[]` does not name. `level` is the dimension of
/// an array node that the value fills.
#[derive(Clone, Copy)]
struct Typed<'a> {
    value: &'a Value,
    node: Option<&'a TypeNode>,
    level: usize,
}

/// One array or struct of the ordered tree while the walk builds it.
struct Frame<'a> {
    /// The key of the container in its parent struct. It is empty in an array and at the root.
    key: String,
    is_list: bool,
    pending: std::vec::IntoIter<(String, Typed<'a>)>,
    done: Vec<(String, OrderedValue)>,
}

impl<'a> Frame<'a> {
    fn new(key: String, is_list: bool, pending: Vec<(String, Typed<'a>)>) -> Self {
        Self {
            key,
            is_list,
            done: Vec::with_capacity(pending.len()),
            pending: pending.into_iter(),
        }
    }

    /// Return the key of the container and the finished container.
    fn finish(self) -> (String, OrderedValue) {
        let value = if self.is_list {
            OrderedValue::List(self.done.into_iter().map(|(_, item)| item).collect())
        } else {
            OrderedValue::Members(self.done)
        };
        (self.key, value)
    }
}

/// A value that the walk opened: a finished scalar with its key, or a container to fill.
enum Opened<'a> {
    Scalar(String, OrderedValue),
    Container(Frame<'a>),
}

fn open(key: String, typed: Typed<'_>) -> Opened<'_> {
    match typed.value {
        Value::Null => Opened::Scalar(key, OrderedValue::Null),
        Value::Bool(flag) => Opened::Scalar(key, OrderedValue::Bool(*flag)),
        Value::Number(number) => Opened::Scalar(key, OrderedValue::Text(number.to_string())),
        Value::String(text) => Opened::Scalar(key, OrderedValue::Text(text.clone())),
        Value::Array(items) => Opened::Container(Frame::new(key, true, elements(items, typed))),
        Value::Object(map) => Opened::Container(Frame::new(key, false, members(map, typed.node))),
    }
}

/// Pair each element of an array value with its node. The elements keep their order.
fn elements<'a>(items: &'a [Value], typed: Typed<'a>) -> Vec<(String, Typed<'a>)> {
    let (node, level) = element_node(typed);
    items
        .iter()
        .map(|value| (String::new(), Typed { value, node, level }))
        .collect()
}

/// Return the node and the level of the elements of an array value.
///
/// A value of an array with several dimensions nests one JSON array for each dimension. The
/// element node applies below the last dimension. A value that does not fit its node gets none.
fn element_node(typed: Typed<'_>) -> (Option<&TypeNode>, usize) {
    let Some(TypeNode::Array {
        dimensions,
        element,
        ..
    }) = typed.node
    else {
        return (None, 0);
    };
    match typed.level.checked_add(1) {
        Some(next) if next < dimensions.len() => (typed.node, next),
        Some(_) => (Some(element.as_ref()), 0),
        None => (None, 0),
    }
}

/// Pair each member of a struct value with its node, in the order of `members[]`.
///
/// A key that `members[]` does not name comes after the members, in byte order.
fn members<'a>(
    map: &'a Map<String, Value>,
    node: Option<&'a TypeNode>,
) -> Vec<(String, Typed<'a>)> {
    let declared: &[(MemberName, TypeNode)] = match node {
        Some(TypeNode::Struct { members, .. }) => members,
        _ => &[],
    };
    let typed = |value, node| Typed {
        value,
        node,
        level: 0,
    };
    let mut entries: Vec<(String, Typed<'a>)> = declared
        .iter()
        .filter_map(|(name, member)| {
            let value = map.get(name.as_str())?;
            Some((name.as_str().to_owned(), typed(value, Some(member))))
        })
        .collect();
    let mut extra: Vec<(&String, &Value)> = map
        .iter()
        .filter(|(key, _)| {
            !declared
                .iter()
                .any(|(name, _)| name.as_str() == key.as_str())
        })
        .collect();
    extra.sort_by(|a, b| a.0.cmp(b.0));
    entries.extend(
        extra
            .into_iter()
            .map(|(key, value)| (key.clone(), typed(value, None))),
    );
    entries
}

/// Build the ordered tree of a stored value of the type `root`.
///
/// The walk uses an explicit stack of open containers and never recurses (`reliability.md`
/// section 2.2).
fn ordered_value<'a>(value: &'a Value, root: &'a TypeNode) -> OrderedValue {
    let start = Typed {
        value,
        node: Some(root),
        level: 0,
    };
    let mut frame = match open(String::new(), start) {
        Opened::Scalar(_, scalar) => return scalar,
        Opened::Container(frame) => frame,
    };
    let mut parents: Vec<Frame<'a>> = Vec::new();
    loop {
        if let Some((key, child)) = frame.pending.next() {
            match open(key, child) {
                Opened::Scalar(key, scalar) => frame.done.push((key, scalar)),
                Opened::Container(inner) => parents.push(std::mem::replace(&mut frame, inner)),
            }
            continue;
        }
        let (key, finished) = frame.finish();
        let Some(parent) = parents.pop() else {
            return finished;
        };
        frame = parent;
        frame.done.push((key, finished));
    }
}

#[cfg(test)]
mod tests;
