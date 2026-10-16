//! The typed form that a shape fault expects (ADR-0039, ADR-0041).

use std::fmt;

use rust_decimal::Decimal;

use crate::plant::{MemberName, Primitive};

/// The form that the type needs at the leaf of a shape fault (`E-RCP-005`).
///
/// The type solves one problem: the web layer shows the expected form in the language of the
/// user, so the form cannot be English text. Each variant maps to one key of the bundles `pl`
/// and `en`, and its fields carry the data of the phrase. `Display` writes the English phrase
/// for the log. The enum is exhaustive on purpose, so a new variant forces a new key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expected {
    /// The request text is not JSON.
    Json,
    /// The value has more nested arrays and objects than `json.max_depth` allows.
    Depth {
        /// The largest permitted number of levels.
        levels: u32,
    },
    /// A struct needs an object with exactly its members.
    Object {
        /// The member names in the order of the struct.
        members: Vec<MemberName>,
    },
    /// The tree expects an array at this place.
    Array,
    /// An array dimension needs an exact count of elements.
    ArrayCount {
        /// The number of elements of the dimension.
        count: u32,
    },
    /// An element index is outside the INTEGER range.
    Index,
    /// A new member of a rebind is a scalar leaf, and the request gives another form.
    Scalar,
    /// A `WSTRING(n)` leaf needs a string inside its length in UTF-16 units.
    Wstring {
        /// The length of the type.
        length: u16,
    },
    /// A `STRING(n)` leaf needs a string of Latin-1 characters inside its length.
    String {
        /// The length of the type.
        length: u16,
    },
    /// A `CHAR` or `WCHAR` leaf needs one Unicode scalar inside the range of the type.
    Char {
        /// The highest scalar of the type.
        max: u32,
    },
    /// A time leaf of a request needs an IEC literal of its type.
    Literal {
        /// The time type of the leaf.
        type_name: Primitive,
    },
    /// A time leaf of storage needs a count of its type.
    Count {
        /// The time type of the leaf.
        type_name: Primitive,
    },
    /// An integer leaf needs a number without a fraction.
    Integer {
        /// The integer type of the leaf.
        type_name: Primitive,
    },
    /// An integer leaf needs a number inside the range of its type.
    NumberRange {
        /// The integer type of the leaf.
        type_name: Primitive,
        /// The lowest value of the type.
        low: Decimal,
        /// The highest value of the type.
        high: Decimal,
    },
    /// A `REAL` or `LREAL` leaf needs a number with a limited count of significant digits.
    NumberDigits {
        /// The type of the leaf.
        type_name: Primitive,
        /// The largest permitted count of significant digits.
        digits: u32,
    },
    /// The value belongs to another root type than the tree of the parameter.
    ParameterType,
    /// A type change of a parameter needs a new value.
    NewValue,
    /// A `BOOL` leaf needs `true` or `false`.
    Bool,
    /// The text of a numeric leaf needs a decimal number with a dot.
    Decimal,
    /// A leaf of the type has no text.
    Leaf,
    /// A text names a leaf path that the type does not hold.
    KnownLeaf,
}

impl fmt::Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json => f.write_str("valid JSON text"),
            Self::Depth { levels } => write!(f, "a JSON value with at most {levels} levels"),
            Self::Object { members } => write_members(f, members),
            Self::Array => f.write_str("an array"),
            Self::ArrayCount { count } => write!(f, "an array of {count} elements"),
            Self::Index => f.write_str("an index inside the INTEGER range"),
            Self::Scalar => f.write_str("a scalar value"),
            Self::Wstring { length } => {
                write!(f, "a string of at most {length} UTF-16 units")
            }
            Self::String { length } => {
                write!(f, "a string of at most {length} characters from Latin-1")
            }
            Self::Char { max } => write!(f, "one character from U+0000 to U+{max:04X}"),
            Self::Literal { type_name } => write!(f, "an IEC literal of type {type_name}"),
            Self::Count { type_name } => write!(f, "a count of type {type_name}"),
            Self::Integer { type_name } => write!(f, "an integer of type {type_name}"),
            Self::NumberRange {
                type_name,
                low,
                high,
            } => write!(f, "a number of type {type_name} from {low} to {high}"),
            Self::NumberDigits { type_name, digits } => write!(
                f,
                "a number of type {type_name} with at most {digits} significant digits"
            ),
            Self::ParameterType => f.write_str("a value of the type of the parameter"),
            Self::NewValue => f.write_str("a new value for the new type"),
            Self::Bool => f.write_str("true or false"),
            Self::Decimal => f.write_str("a decimal number with a dot, for example 1.5"),
            Self::Leaf => f.write_str("a value for this field"),
            Self::KnownLeaf => f.write_str("a path of the type of the parameter"),
        }
    }
}

/// Write the phrase of an object fault with its member names, separated by a comma.
fn write_members(f: &mut fmt::Formatter<'_>, members: &[MemberName]) -> fmt::Result {
    f.write_str("an object with the members ")?;
    for (index, member) in members.iter().enumerate() {
        if index > 0 {
            f.write_str(", ")?;
        }
        write!(f, "{member}")?;
    }
    Ok(())
}
