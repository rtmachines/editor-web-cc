//! Names of structs and struct members of the Plant Model
//! (`reliability.md` section 4.3, `data-model.md` section 3.4).

use std::fmt;

use super::RuleSet;
use super::primitive::Primitive;

/// Longest length of a name in bytes. Names are ASCII, so bytes are characters.
const MAX_NAME_LENGTH: usize = 64;

/// The IEC 61131-3 and TwinCAT keywords that are not primitive names, in upper case
/// (ADR-0143 (d)). A struct name is not one of them, in any case.
pub const RESERVED_KEYWORDS: [&str; 182] = [
    "ABS",
    "ABSTRACT",
    "ACOS",
    "ACTION",
    "ADD",
    "ADR",
    "AND",
    "ANDN",
    "ANY",
    "ANY_BIT",
    "ANY_CHAR",
    "ANY_CHARS",
    "ANY_DATE",
    "ANY_DERIVED",
    "ANY_DURATION",
    "ANY_ELEMENTARY",
    "ANY_INT",
    "ANY_MAGNITUDE",
    "ANY_NUM",
    "ANY_REAL",
    "ANY_SIGNED",
    "ANY_STRING",
    "ANY_UNSIGNED",
    "ARRAY",
    "ASIN",
    "AT",
    "ATAN",
    "BITADR",
    "BY",
    "CAL",
    "CALC",
    "CALCN",
    "CASE",
    "CLASS",
    "CONFIGURATION",
    "CONSTANT",
    "CONTINUE",
    "COS",
    "DIV",
    "DO",
    "DT",
    "ELSE",
    "ELSIF",
    "EN",
    "END_ACTION",
    "END_CASE",
    "END_CLASS",
    "END_CONFIGURATION",
    "END_FOR",
    "END_FUNCTION",
    "END_FUNCTION_BLOCK",
    "END_GET",
    "END_IF",
    "END_INTERFACE",
    "END_METHOD",
    "END_NAMESPACE",
    "END_PROGRAM",
    "END_PROPERTY",
    "END_REPEAT",
    "END_RESOURCE",
    "END_SET",
    "END_STEP",
    "END_STRUCT",
    "END_TRANSITION",
    "END_TYPE",
    "END_VAR",
    "END_WHILE",
    "ENO",
    "EQ",
    "EXIT",
    "EXP",
    "EXPT",
    "EXTENDS",
    "FALSE",
    "FINAL",
    "FOR",
    "FROM",
    "FUNCTION",
    "FUNCTION_BLOCK",
    "F_EDGE",
    "GE",
    "GET",
    "GT",
    "IF",
    "IMPLEMENTS",
    "INDEXOF",
    "INITIAL_STEP",
    "INTERFACE",
    "INTERNAL",
    "JMP",
    "JMPC",
    "JMPCN",
    "LD",
    "LDATE_AND_TIME",
    "LDN",
    "LE",
    "LN",
    "LOG",
    "LT",
    "LTIME_OF_DAY",
    "MAX",
    "METHOD",
    "MIN",
    "MOD",
    "MOVE",
    "MUL",
    "MUX",
    "NAMESPACE",
    "NE",
    "NON_RETAIN",
    "NOT",
    "NULL",
    "OF",
    "ON",
    "OR",
    "ORN",
    "OVERLAP",
    "OVERRIDE",
    "PARAMS",
    "PERSISTENT",
    "POINTER",
    "PRIVATE",
    "PROGRAM",
    "PROPERTY",
    "PROTECTED",
    "PUBLIC",
    "R",
    "READ_ONLY",
    "READ_WRITE",
    "REF",
    "REFERENCE",
    "REF_TO",
    "REPEAT",
    "RESOURCE",
    "RET",
    "RETAIN",
    "RETC",
    "RETCN",
    "RETURN",
    "ROL",
    "ROR",
    "R_EDGE",
    "S",
    "SEL",
    "SET",
    "SHL",
    "SHR",
    "SIN",
    "SIZEOF",
    "SQRT",
    "ST",
    "STEP",
    "STN",
    "STRUCT",
    "SUB",
    "SUPER",
    "TAN",
    "TASK",
    "THEN",
    "THIS",
    "TO",
    "TOD",
    "TRANSITION",
    "TRUE",
    "TRUNC",
    "TYPE",
    "UNTIL",
    "USING",
    "VAR",
    "VAR_ACCESS",
    "VAR_CONFIG",
    "VAR_EXTERNAL",
    "VAR_GLOBAL",
    "VAR_INPUT",
    "VAR_IN_OUT",
    "VAR_OUTPUT",
    "VAR_STAT",
    "VAR_TEMP",
    "WHILE",
    "WITH",
    "XOR",
    "XORN",
];

/// Tell whether a text is an IEC keyword or a primitive name, in any case (ADR-0143 (c)).
#[must_use]
pub fn is_reserved_name(text: &str) -> bool {
    RESERVED_KEYWORDS
        .iter()
        .any(|word| word.eq_ignore_ascii_case(text))
        || Primitive::ALL
            .iter()
            .any(|primitive| primitive.name().eq_ignore_ascii_case(text))
}

/// Name of a struct type, such as `ST_Zone`.
///
/// The grammar is `^[A-Za-z_][A-Za-z0-9_]{0,63}$`. A name is not a primitive
/// type name and not an IEC keyword, in any case (ADR-0143 (c)).
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
    /// The text is an IEC keyword or the name of a primitive type.
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
                    "a type name is 1 to {MAX_NAME_LENGTH} ASCII letters, digits, or underscores, and the first character is a letter or an underscore"
                )
            }
            Self::Reserved => {
                f.write_str("a type name cannot be an IEC keyword or a primitive type name")
            }
        }
    }
}

impl fmt::Display for MemberNameReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => {
                write!(
                    f,
                    "a member name is 1 to {MAX_NAME_LENGTH} ASCII letters, digits, or underscores, and the first character is a letter or an underscore"
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
    /// Returns [`TypeNameReason::Reserved`] when the text is an IEC keyword or a primitive name,
    /// in any case.
    pub fn parse(text: &str) -> Result<Self, TypeNameReason> {
        Self::from_stored(text, RuleSet::Current)
    }

    /// Read the name of a struct type under a rule set (ADR-0120, ADR-0143 (i1)).
    ///
    /// [`RuleSet::Current`] applies the rule of [`TypeName::parse`]. [`RuleSet::Earlier`]
    /// refuses only a primitive name in exact case. A stored struct such as `Int` stays readable.
    ///
    /// # Errors
    ///
    /// Returns [`TypeNameReason::Format`] when the text does not match the grammar.
    /// Returns [`TypeNameReason::Reserved`] when the rule set refuses the text as a reserved name.
    pub fn from_stored(text: &str, rules: RuleSet) -> Result<Self, TypeNameReason> {
        if !is_name(text) {
            return Err(TypeNameReason::Format);
        }
        let reserved = match rules {
            RuleSet::Current => is_reserved_name(text),
            RuleSet::Earlier => Primitive::parse(text).is_ok(),
        };
        if reserved {
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
