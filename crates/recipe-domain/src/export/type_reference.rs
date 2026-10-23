//! The type reference of the export document (`api.md` section 6.1).

use std::fmt;

use serde::{Serialize, Serializer};

use crate::plant::{PlcTypeId, Primitive};

/// A type reference: `type_id` of a parameter or a member, and `element_type_id` of an array.
///
/// The type solves one problem: a type reference is a domain identifier, so no signature gives it
/// as bare text. Its text is the primitive name for a primitive without a length. For every other
/// type, the text is the identifier of the type row, the `id` of its entry in `types[]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeReference {
    /// A primitive without a length, for example `INT`.
    Primitive(Primitive),
    /// The type row of every other type.
    Row(PlcTypeId),
}

impl fmt::Display for TypeReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primitive(primitive) => f.write_str(primitive.name()),
            Self::Row(id) => id.fmt(f),
        }
    }
}

impl Serialize for TypeReference {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
