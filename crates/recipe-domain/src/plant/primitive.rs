//! PLC primitive types of the Plant Model (`domain-model.md` section 2.1,
//! `data-model.md` section 7).

use std::fmt;

/// Primitive PLC type of a leaf. Each name is an IEC or S7 type name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Primitive {
    /// `BOOL`, one bit.
    Bool,
    /// `BYTE`, 8 bits.
    Byte,
    /// `WORD`, 16 bits.
    Word,
    /// `DWORD`, 32 bits.
    Dword,
    /// `LWORD`, 64 bits.
    Lword,
    /// `SINT`, signed 8-bit integer.
    Sint,
    /// `USINT`, unsigned 8-bit integer.
    Usint,
    /// `INT`, signed 16-bit integer.
    Int,
    /// `UINT`, unsigned 16-bit integer.
    Uint,
    /// `DINT`, signed 32-bit integer.
    Dint,
    /// `UDINT`, unsigned 32-bit integer.
    Udint,
    /// `LINT`, signed 64-bit integer.
    Lint,
    /// `ULINT`, unsigned 64-bit integer.
    Ulint,
    /// `REAL`, 32-bit float.
    Real,
    /// `LREAL`, 64-bit float.
    Lreal,
    /// `STRING`, one byte per character.
    String,
    /// `WSTRING`, two bytes per character.
    Wstring,
    /// `CHAR`, one byte character.
    Char,
    /// `WCHAR`, two byte character.
    Wchar,
    /// `TIME`, a duration in milliseconds.
    Time,
    /// `LTIME`, a duration in nanoseconds.
    Ltime,
    /// `DATE`, a day count since 1970-01-01.
    Date,
    /// `LDATE`, a nanosecond count since 1970-01-01.
    Ldate,
    /// `TIME_OF_DAY`, a millisecond count since midnight.
    TimeOfDay,
    /// `LTOD`, a nanosecond count since midnight.
    Ltod,
    /// `DATE_AND_TIME`, a second count since 1970-01-01.
    DateAndTime,
    /// `LDT`, a nanosecond count since 1970-01-01.
    Ldt,
    /// `DTL`, a nanosecond count since 1970-01-01 in 12 bytes.
    Dtl,
    /// `S5TIME`, a duration in milliseconds in 2 bytes.
    S5Time,
}

/// Class of a primitive leaf. The class sets the value form and the process limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LeafClass {
    /// Integer or float value.
    Numeric,
    /// Text value.
    String,
    /// Boolean value.
    Bool,
    /// One character value.
    Char,
    /// Duration value.
    Duration,
    /// Point in time value.
    Instant,
}

/// Reason why a text is not a primitive type name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveReason {
    /// The text is not one of the 29 primitive names.
    Unknown,
}

/// Reason why a text is not a leaf class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeafClassReason {
    /// The text is not one of the six leaf class values.
    Unknown,
}

impl fmt::Display for PrimitiveReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => f.write_str("the text is not a PLC primitive type"),
        }
    }
}

impl fmt::Display for LeafClassReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => f.write_str("the text is not a leaf class"),
        }
    }
}

impl Primitive {
    /// Every primitive, in the order of `data-model.md` section 7.
    pub const ALL: [Self; 29] = [
        Self::Bool,
        Self::Byte,
        Self::Word,
        Self::Dword,
        Self::Lword,
        Self::Sint,
        Self::Usint,
        Self::Int,
        Self::Uint,
        Self::Dint,
        Self::Udint,
        Self::Lint,
        Self::Ulint,
        Self::Real,
        Self::Lreal,
        Self::String,
        Self::Wstring,
        Self::Char,
        Self::Wchar,
        Self::Time,
        Self::Ltime,
        Self::Date,
        Self::Ldate,
        Self::TimeOfDay,
        Self::Ltod,
        Self::DateAndTime,
        Self::Ldt,
        Self::Dtl,
        Self::S5Time,
    ];

    /// Parse the upper-case IEC or S7 name of a primitive, such as `INT`.
    ///
    /// # Errors
    ///
    /// Returns [`PrimitiveReason::Unknown`] for any other text.
    pub fn parse(name: &str) -> Result<Self, PrimitiveReason> {
        Self::ALL
            .into_iter()
            .find(|primitive| primitive.name() == name)
            .ok_or(PrimitiveReason::Unknown)
    }

    /// Return the IEC or S7 name of this primitive, such as `TIME_OF_DAY`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bool => "BOOL",
            Self::Byte => "BYTE",
            Self::Word => "WORD",
            Self::Dword => "DWORD",
            Self::Lword => "LWORD",
            Self::Sint => "SINT",
            Self::Usint => "USINT",
            Self::Int => "INT",
            Self::Uint => "UINT",
            Self::Dint => "DINT",
            Self::Udint => "UDINT",
            Self::Lint => "LINT",
            Self::Ulint => "ULINT",
            Self::Real => "REAL",
            Self::Lreal => "LREAL",
            Self::String => "STRING",
            Self::Wstring => "WSTRING",
            Self::Char => "CHAR",
            Self::Wchar => "WCHAR",
            Self::Time => "TIME",
            Self::Ltime => "LTIME",
            Self::Date => "DATE",
            Self::Ldate => "LDATE",
            Self::TimeOfDay => "TIME_OF_DAY",
            Self::Ltod => "LTOD",
            Self::DateAndTime => "DATE_AND_TIME",
            Self::Ldt => "LDT",
            Self::Dtl => "DTL",
            Self::S5Time => "S5TIME",
        }
    }

    /// Return the class of this primitive leaf.
    #[must_use]
    pub const fn leaf_class(self) -> LeafClass {
        match self {
            Self::Bool => LeafClass::Bool,
            Self::Char | Self::Wchar => LeafClass::Char,
            Self::String | Self::Wstring => LeafClass::String,
            Self::Time | Self::Ltime | Self::S5Time => LeafClass::Duration,
            Self::Date
            | Self::Ldate
            | Self::TimeOfDay
            | Self::Ltod
            | Self::DateAndTime
            | Self::Ldt
            | Self::Dtl => LeafClass::Instant,
            Self::Byte
            | Self::Word
            | Self::Dword
            | Self::Lword
            | Self::Sint
            | Self::Usint
            | Self::Int
            | Self::Uint
            | Self::Dint
            | Self::Udint
            | Self::Lint
            | Self::Ulint
            | Self::Real
            | Self::Lreal => LeafClass::Numeric,
        }
    }

    /// Return the S7 size of this primitive in bytes, or `None` for a size
    /// that depends on the length (`STRING` and `WSTRING`).
    #[must_use]
    pub const fn fixed_size_bytes(self) -> Option<u32> {
        match self {
            Self::Bool | Self::Byte | Self::Sint | Self::Usint | Self::Char => Some(1),
            Self::Word | Self::Int | Self::Uint | Self::Wchar | Self::Date | Self::S5Time => {
                Some(2)
            }
            Self::Dword | Self::Dint | Self::Udint | Self::Real | Self::Time | Self::TimeOfDay => {
                Some(4)
            }
            Self::Lword
            | Self::Lint
            | Self::Ulint
            | Self::Lreal
            | Self::Ltime
            | Self::Ldate
            | Self::Ltod
            | Self::DateAndTime
            | Self::Ldt => Some(8),
            Self::Dtl => Some(12),
            Self::String | Self::Wstring => None,
        }
    }
}

impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl LeafClass {
    /// Every leaf class, in the order of `domain-model.md` section 2.1.
    const ALL: [Self; 6] = [
        Self::Numeric,
        Self::String,
        Self::Bool,
        Self::Char,
        Self::Duration,
        Self::Instant,
    ];

    /// Return the SQL value of this class, such as `numeric`.
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Numeric => "numeric",
            Self::String => "string",
            Self::Bool => "bool",
            Self::Char => "char",
            Self::Duration => "duration",
            Self::Instant => "instant",
        }
    }

    /// Parse the SQL value of a class, such as `duration`.
    ///
    /// # Errors
    ///
    /// Returns [`LeafClassReason::Unknown`] for any other text.
    pub fn parse(text: &str) -> Result<Self, LeafClassReason> {
        Self::ALL
            .into_iter()
            .find(|class| class.as_sql() == text)
            .ok_or(LeafClassReason::Unknown)
    }
}

#[cfg(test)]
mod tests;
