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
mod tests {
    use super::*;

    /// Name, class, and S7 size of each primitive (`data-model.md` section 7).
    const MATRIX: [(Primitive, &str, LeafClass, Option<u32>); 29] = [
        (Primitive::Bool, "BOOL", LeafClass::Bool, Some(1)),
        (Primitive::Byte, "BYTE", LeafClass::Numeric, Some(1)),
        (Primitive::Word, "WORD", LeafClass::Numeric, Some(2)),
        (Primitive::Dword, "DWORD", LeafClass::Numeric, Some(4)),
        (Primitive::Lword, "LWORD", LeafClass::Numeric, Some(8)),
        (Primitive::Sint, "SINT", LeafClass::Numeric, Some(1)),
        (Primitive::Usint, "USINT", LeafClass::Numeric, Some(1)),
        (Primitive::Int, "INT", LeafClass::Numeric, Some(2)),
        (Primitive::Uint, "UINT", LeafClass::Numeric, Some(2)),
        (Primitive::Dint, "DINT", LeafClass::Numeric, Some(4)),
        (Primitive::Udint, "UDINT", LeafClass::Numeric, Some(4)),
        (Primitive::Lint, "LINT", LeafClass::Numeric, Some(8)),
        (Primitive::Ulint, "ULINT", LeafClass::Numeric, Some(8)),
        (Primitive::Real, "REAL", LeafClass::Numeric, Some(4)),
        (Primitive::Lreal, "LREAL", LeafClass::Numeric, Some(8)),
        (Primitive::String, "STRING", LeafClass::String, None),
        (Primitive::Wstring, "WSTRING", LeafClass::String, None),
        (Primitive::Char, "CHAR", LeafClass::Char, Some(1)),
        (Primitive::Wchar, "WCHAR", LeafClass::Char, Some(2)),
        (Primitive::Time, "TIME", LeafClass::Duration, Some(4)),
        (Primitive::Ltime, "LTIME", LeafClass::Duration, Some(8)),
        (Primitive::Date, "DATE", LeafClass::Instant, Some(2)),
        (Primitive::Ldate, "LDATE", LeafClass::Instant, Some(8)),
        (
            Primitive::TimeOfDay,
            "TIME_OF_DAY",
            LeafClass::Instant,
            Some(4),
        ),
        (Primitive::Ltod, "LTOD", LeafClass::Instant, Some(8)),
        (
            Primitive::DateAndTime,
            "DATE_AND_TIME",
            LeafClass::Instant,
            Some(8),
        ),
        (Primitive::Ldt, "LDT", LeafClass::Instant, Some(8)),
        (Primitive::Dtl, "DTL", LeafClass::Instant, Some(12)),
        (Primitive::S5Time, "S5TIME", LeafClass::Duration, Some(2)),
    ];

    #[test]
    fn all_lists_the_matrix_in_order() {
        let listed: Vec<Primitive> = MATRIX.iter().map(|row| row.0).collect();
        assert_eq!(Primitive::ALL.to_vec(), listed);
    }

    #[test]
    fn every_name_matches_the_matrix() {
        for (primitive, name, _, _) in MATRIX {
            assert_eq!(primitive.name(), name);
        }
    }

    #[test]
    fn every_leaf_class_matches_the_matrix() {
        for (primitive, _, class, _) in MATRIX {
            assert_eq!(primitive.leaf_class(), class, "class of {primitive}");
        }
    }

    #[test]
    fn every_fixed_size_matches_the_matrix() {
        for (primitive, _, _, size) in MATRIX {
            assert_eq!(primitive.fixed_size_bytes(), size, "size of {primitive}");
        }
    }

    #[test]
    fn parse_round_trips_every_name() {
        for primitive in Primitive::ALL {
            assert_eq!(Primitive::parse(primitive.name()), Ok(primitive));
        }
    }

    #[test]
    fn display_writes_the_name() {
        assert_eq!(Primitive::TimeOfDay.to_string(), "TIME_OF_DAY");
        assert_eq!(Primitive::S5Time.to_string(), "S5TIME");
    }

    #[test]
    fn short_or_wrong_names_fail() {
        for text in ["int", "TOD", "DT", "ENUM", "", "Int", "LTIME_OF_DAY"] {
            assert_eq!(
                Primitive::parse(text),
                Err(PrimitiveReason::Unknown),
                "{text}"
            );
        }
    }

    #[test]
    fn leaf_class_sql_values_round_trip() {
        for class in LeafClass::ALL {
            assert_eq!(LeafClass::parse(class.as_sql()), Ok(class));
        }
    }

    #[test]
    fn leaf_class_sql_values_match_data_model() {
        assert_eq!(LeafClass::Numeric.as_sql(), "numeric");
        assert_eq!(LeafClass::String.as_sql(), "string");
        assert_eq!(LeafClass::Bool.as_sql(), "bool");
        assert_eq!(LeafClass::Char.as_sql(), "char");
        assert_eq!(LeafClass::Duration.as_sql(), "duration");
        assert_eq!(LeafClass::Instant.as_sql(), "instant");
    }

    #[test]
    fn bad_leaf_class_value_fails() {
        assert_eq!(LeafClass::parse("Numeric"), Err(LeafClassReason::Unknown));
        assert_eq!(LeafClass::parse(""), Err(LeafClassReason::Unknown));
    }
}
