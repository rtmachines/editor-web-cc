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
