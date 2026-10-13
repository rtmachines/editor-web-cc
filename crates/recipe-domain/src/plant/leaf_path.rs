//! Path to one scalar leaf of a type tree (`domain-model.md` section 2.1).

use std::fmt;

use super::lexical::{IntegerError, parse_integer, split_at_most};
use super::limits::MAX_DIMENSIONS;
use super::{LeafClass, MemberName, PlcTypeId, Primitive, TypeNode, TypeTree};

/// One segment of a [`LeafPath`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LeafSegment {
    /// A struct member, written `.name`.
    Member(MemberName),
    /// One to three indexes of an array, written `[1,2]`.
    Index(Vec<i32>),
    /// Every element of an array, written `[*]`.
    All,
}

/// Return `path` with one more segment, as a new vector.
///
/// Every walk over a type tree or over a value builds the path of a child with this function.
pub(crate) fn extended(path: &[LeafSegment], segment: LeafSegment) -> Vec<LeafSegment> {
    let mut longer = path.to_vec();
    longer.push(segment);
    longer
}

/// A path to one scalar inside a type tree, such as `.zones[2].temp_max`.
///
/// The empty path is the parameter itself. The grammar is the only
/// definition of section 2.1 of `domain-model.md`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeafPath(Vec<LeafSegment>);

/// Reason why a text is not a leaf path (ADR-0025).
///
/// The enum has no code. It is not an error type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeafPathReason {
    /// The text does not match the grammar of a leaf path.
    Grammar,
    /// An index is outside the INTEGER range.
    IndexOutOfRange,
    /// An index segment has more than three indexes.
    TooManyIndexes,
}

impl fmt::Display for LeafPathReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grammar => f.write_str("the leaf path does not match the grammar"),
            Self::IndexOutOfRange => f.write_str("an index is outside the INTEGER range"),
            Self::TooManyIndexes => {
                write!(f, "an index segment has more than {MAX_DIMENSIONS} indexes")
            }
        }
    }
}

impl LeafPath {
    /// Parse the text form of a leaf path.
    ///
    /// # Errors
    ///
    /// Returns [`LeafPathReason::Grammar`] when the text does not match the grammar.
    /// Returns [`LeafPathReason::IndexOutOfRange`] when an index is outside the INTEGER range.
    /// Returns [`LeafPathReason::TooManyIndexes`] when an index segment has four or more indexes.
    pub fn parse(text: &str) -> Result<Self, LeafPathReason> {
        let mut segments = Vec::new();
        let mut rest = text;
        while !rest.is_empty() {
            let (segment, tail) = parse_segment(rest)?;
            segments.push(segment);
            rest = tail;
        }
        Ok(Self(segments))
    }

    /// Build the path of one concrete leaf from its segments.
    ///
    /// The caller guarantees that the segments follow the grammar. A tree walk is the caller.
    pub(crate) const fn from_segments(segments: Vec<LeafSegment>) -> Self {
        Self(segments)
    }

    /// Build the empty path, which names the whole value of a parameter.
    pub(crate) const fn whole() -> Self {
        Self(Vec::new())
    }

    /// Return the segments, from the root to the leaf.
    #[must_use]
    pub fn segments(&self) -> &[LeafSegment] {
        &self.0
    }

    /// Return `true` when the path is empty, which names the parameter itself.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The scalar leaves that a [`LeafPath`] names inside one type tree.
///
/// The type describes the leaf. The count is the number of leaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeafTarget {
    type_id: PlcTypeId,
    primitive: Primitive,
    leaf_class: LeafClass,
    size_bytes: u32,
    string_length: Option<u16>,
    count: u32,
}

impl LeafTarget {
    /// Describe one single leaf of a tree. The count is 1.
    pub(crate) const fn single(
        type_id: PlcTypeId,
        primitive: Primitive,
        leaf_class: LeafClass,
        size_bytes: u32,
        string_length: Option<u16>,
    ) -> Self {
        Self {
            type_id,
            primitive,
            leaf_class,
            size_bytes,
            string_length,
            count: 1,
        }
    }

    /// Return the row of the leaf type.
    #[must_use]
    pub const fn type_id(&self) -> PlcTypeId {
        self.type_id
    }

    /// Return the primitive of the leaf.
    #[must_use]
    pub const fn primitive(&self) -> Primitive {
        self.primitive
    }

    /// Return the leaf class of the primitive.
    #[must_use]
    pub const fn leaf_class(&self) -> LeafClass {
        self.leaf_class
    }

    /// Return the S7 size of one leaf in bytes.
    #[must_use]
    pub const fn size_bytes(&self) -> u32 {
        self.size_bytes
    }

    /// Return the length of a string leaf.
    #[must_use]
    pub const fn string_length(&self) -> Option<u16> {
        self.string_length
    }

    /// Return the number of scalar leaves that the path names.
    ///
    /// The count is 1 without `[*]`. Each `[*]` multiplies it by the element
    /// count of its array. It cannot exceed the budget of 16 384 leaves.
    #[must_use]
    pub const fn count(&self) -> u32 {
        self.count
    }
}

impl LeafPath {
    /// Resolve this path against a type tree.
    ///
    /// Returns `None` when the tree has no such leaf. The caller maps `None`
    /// to `E-RCP-017`.
    #[must_use]
    pub fn resolve(&self, tree: &TypeTree) -> Option<LeafTarget> {
        let mut node = tree.root();
        let mut count = 1_u64;
        for segment in &self.0 {
            let (next, factor) = step(node, segment)?;
            node = next;
            count = count.checked_mul(factor)?;
        }
        let TypeNode::Leaf {
            type_id,
            primitive,
            leaf_class,
            size_bytes,
            string_length,
        } = node
        else {
            return None;
        };
        Some(LeafTarget {
            type_id: *type_id,
            primitive: *primitive,
            leaf_class: *leaf_class,
            size_bytes: *size_bytes,
            string_length: *string_length,
            count: u32::try_from(count).ok()?,
        })
    }
}

/// Follow one segment from `node`. Return the next node and the leaves factor.
fn step<'t>(node: &'t TypeNode, segment: &LeafSegment) -> Option<(&'t TypeNode, u64)> {
    match (node, segment) {
        (TypeNode::Struct { members, .. }, LeafSegment::Member(name)) => members
            .iter()
            .find(|(member, _)| member == name)
            .map(|(_, child)| (child, 1)),
        (
            TypeNode::Array {
                dimensions,
                element,
                ..
            },
            LeafSegment::Index(values),
        ) => {
            let is_inside = values.len() == dimensions.len()
                && dimensions.iter().zip(values).all(|(dimension, value)| {
                    (dimension.lower()..=dimension.upper()).contains(value)
                });
            is_inside.then_some((element.as_ref(), 1))
        }
        (
            TypeNode::Array {
                dimensions,
                element,
                ..
            },
            LeafSegment::All,
        ) => Some((element.as_ref(), dimensions.element_count()?)),
        _ => None,
    }
}

mod expand;

#[cfg(test)]
mod resolve;

/// Parse the first segment of `text` and return the rest of the text.
fn parse_segment(text: &str) -> Result<(LeafSegment, &str), LeafPathReason> {
    if let Some(after) = text.strip_prefix('.') {
        return parse_member(after);
    }
    if let Some(tail) = text.strip_prefix("[*]") {
        return Ok((LeafSegment::All, tail));
    }
    let after = text.strip_prefix('[').ok_or(LeafPathReason::Grammar)?;
    let (inside, tail) = after.split_once(']').ok_or(LeafPathReason::Grammar)?;
    Ok((LeafSegment::Index(parse_indexes(inside)?), tail))
}

/// Parse the member name after a `.` and return the rest of the text.
fn parse_member(after: &str) -> Result<(LeafSegment, &str), LeafPathReason> {
    let end = after.find(['.', '[']).unwrap_or(after.len());
    let name = after.get(..end).ok_or(LeafPathReason::Grammar)?;
    let tail = after.get(end..).ok_or(LeafPathReason::Grammar)?;
    let member = MemberName::parse(name).map_err(|_| LeafPathReason::Grammar)?;
    Ok((LeafSegment::Member(member), tail))
}

/// Parse the comma-separated indexes inside `[...]`.
fn parse_indexes(inside: &str) -> Result<Vec<i32>, LeafPathReason> {
    split_at_most(inside, ',', MAX_DIMENSIONS)
        .ok_or(LeafPathReason::TooManyIndexes)?
        .into_iter()
        .map(parse_index)
        .collect()
}

/// Parse one index `-?[0-9]{1,10}`. The value must fit the INTEGER range.
fn parse_index(text: &str) -> Result<i32, LeafPathReason> {
    parse_integer(text).map_err(|error| match error {
        IntegerError::Grammar => LeafPathReason::Grammar,
        IntegerError::OutOfRange => LeafPathReason::IndexOutOfRange,
    })
}

impl fmt::Display for LeafPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.0 {
            match segment {
                LeafSegment::Member(name) => write!(f, ".{name}")?,
                LeafSegment::All => f.write_str("[*]")?,
                LeafSegment::Index(values) => {
                    f.write_str("[")?;
                    for (position, value) in values.iter().enumerate() {
                        if position > 0 {
                            f.write_str(",")?;
                        }
                        write!(f, "{value}")?;
                    }
                    f.write_str("]")?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn member(name: &str) -> LeafSegment {
        LeafSegment::Member(MemberName::parse(name).expect("valid member name"))
    }

    fn assert_round_trip(text: &str) {
        let parsed = LeafPath::parse(text);
        assert!(parsed.is_ok(), "{text} must parse");
        assert_eq!(parsed.map(|path| path.to_string()), Ok(text.to_owned()));
    }

    fn reason_of(text: &str) -> Option<LeafPathReason> {
        LeafPath::parse(text).err()
    }

    #[test]
    fn leaf_path_examples_round_trip() {
        for text in [
            "",
            ".setpoint",
            "[3]",
            "[1,2]",
            "[-1,0,7]",
            ".zones[2].temp_max",
            ".zones[*].temp_max",
            "[*]",
        ] {
            assert_round_trip(text);
        }
    }

    #[test]
    fn leaf_path_segments_follow_the_text() {
        let parsed = LeafPath::parse(".zones[2].temp_max").map(|p| p.segments().to_vec());
        assert_eq!(
            parsed,
            Ok(vec![
                member("zones"),
                LeafSegment::Index(vec![2]),
                member("temp_max"),
            ])
        );
    }

    #[test]
    fn leaf_path_empty_text_is_empty_path() {
        assert_eq!(LeafPath::parse("").map(|p| p.is_empty()), Ok(true));
        assert_eq!(LeafPath::parse("[3]").map(|p| p.is_empty()), Ok(false));
    }

    #[test]
    fn leaf_path_four_indexes_is_too_many_indexes() {
        assert_eq!(reason_of("[1,2,3,4]"), Some(LeafPathReason::TooManyIndexes));
    }

    #[test]
    fn leaf_path_malformed_texts_are_grammar_errors() {
        for text in [
            "[*,1]",
            "[1,*]",
            "[]",
            "[1,]",
            ".",
            "..a",
            ".1a",
            "zones",
            "[ 1]",
            "[+1]",
            "[12345678901]",
        ] {
            assert_eq!(reason_of(text), Some(LeafPathReason::Grammar), "{text}");
        }
    }

    #[test]
    fn leaf_path_index_above_integer_max_is_out_of_range() {
        assert_eq!(
            reason_of("[2147483648]"),
            Some(LeafPathReason::IndexOutOfRange)
        );
    }

    #[test]
    fn leaf_path_index_below_integer_min_is_out_of_range() {
        assert_eq!(
            reason_of("[-2147483649]"),
            Some(LeafPathReason::IndexOutOfRange)
        );
    }

    #[test]
    fn leaf_path_integer_limits_pass() {
        assert_round_trip("[2147483647]");
        assert_round_trip("[-2147483648]");
    }

    #[test]
    fn leaf_path_leading_zero_displays_canonical_number() {
        assert_eq!(
            LeafPath::parse("[007]").map(|p| p.to_string()),
            Ok("[7]".to_owned())
        );
    }

    #[test]
    fn leaf_path_negative_zero_displays_as_zero() {
        assert_eq!(
            LeafPath::parse("[-0]").map(|p| p.to_string()),
            Ok("[0]".to_owned())
        );
    }

    fn member_text() -> impl Strategy<Value = String> {
        "[A-Za-z_][A-Za-z0-9_]{0,10}".prop_map(|name| format!(".{name}"))
    }

    fn index_text() -> impl Strategy<Value = String> {
        prop::collection::vec(any::<i32>(), 1..=3).prop_map(|values| {
            let joined: Vec<String> = values.iter().map(i32::to_string).collect();
            format!("[{}]", joined.join(","))
        })
    }

    fn segment_text() -> impl Strategy<Value = String> {
        prop_oneof![member_text(), index_text(), Just("[*]".to_owned()),]
    }

    proptest! {
        #[test]
        fn leaf_path_generated_text_round_trips(
            segments in prop::collection::vec(segment_text(), 0..=5),
        ) {
            let text = segments.concat();
            match LeafPath::parse(&text) {
                Ok(parsed) => prop_assert_eq!(parsed.to_string(), text),
                Err(reason) => prop_assert!(false, "{text} failed: {reason}"),
            }
        }
    }
}
