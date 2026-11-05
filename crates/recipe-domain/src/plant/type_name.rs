//! Names of structs and struct members of the Plant Model
//! (`reliability.md` section 4.3, `data-model.md` section 3.4).

use std::fmt;

use super::primitive::Primitive;

/// Longest length of a name in bytes. Names are ASCII, so bytes are characters.
const MAX_NAME_LENGTH: usize = 64;

/// Name of a struct type, such as `ST_Zone`.
///
/// The grammar is `^[A-Za-z_][A-Za-z0-9_]{0,63}$`. A name is not a primitive
/// type name. The check is case-sensitive, so `Int` is a valid struct name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeName(String);

/// Name of a member of a struct, such as `Limit_High`.
///
/// The grammar is the same as the grammar of [`TypeName`]. A member name
/// may equal a primitive name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemberName(String);

/// Reason why a text is not a struct type name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeNameReason {
    /// The text does not match the grammar of a name.
    Format,
    /// The text is the name of a primitive type.
    Reserved,
}

/// Reason why a text is not a struct member name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberNameReason {
    /// The text does not match the grammar of a name.
    Format,
}

impl fmt::Display for TypeNameReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => {
                write!(
                    f,
                    "a type name is 1 to {MAX_NAME_LENGTH} ASCII letters, digits, or underscores"
                )
            }
            Self::Reserved => f.write_str("a type name cannot be a primitive type name"),
        }
    }
}

impl fmt::Display for MemberNameReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => {
                write!(
                    f,
                    "a member name is 1 to {MAX_NAME_LENGTH} ASCII letters, digits, or underscores"
                )
            }
        }
    }
}

/// Check the grammar `^[A-Za-z_][A-Za-z0-9_]{0,63}$` without a regex.
fn is_name(text: &str) -> bool {
    let bytes = text.as_bytes();
    let known_length = (1..=MAX_NAME_LENGTH).contains(&bytes.len());
    let starts_well = bytes
        .first()
        .is_some_and(|first| first.is_ascii_alphabetic() || *first == b'_');
    known_length
        && starts_well
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
}

impl TypeName {
    /// Parse the name of a struct type.
    ///
    /// # Errors
    ///
    /// Returns [`TypeNameReason::Format`] when the text does not match the grammar.
    /// Returns [`TypeNameReason::Reserved`] when the text is a primitive name.
    pub fn parse(text: &str) -> Result<Self, TypeNameReason> {
        if !is_name(text) {
            return Err(TypeNameReason::Format);
        }
        if Primitive::parse(text).is_ok() {
            return Err(TypeNameReason::Reserved);
        }
        Ok(Self(text.to_owned()))
    }

    /// Return the name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl MemberName {
    /// Parse the name of a struct member.
    ///
    /// # Errors
    ///
    /// Returns [`MemberNameReason::Format`] when the text does not match the grammar.
    pub fn parse(text: &str) -> Result<Self, MemberNameReason> {
        if !is_name(text) {
            return Err(MemberNameReason::Format);
        }
        Ok(Self(text.to_owned()))
    }

    /// Return the name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MemberName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests;
