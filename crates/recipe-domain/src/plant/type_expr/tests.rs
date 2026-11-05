use super::*;
use proptest::prelude::*;

fn reason_of(text: &str) -> Option<DerivedTypeReason> {
    match TypeExpr::parse(text) {
        Err(PlantError::DerivedTypeInvalid { type_expr, reason }) => {
            assert_eq!(type_expr, text);
            Some(reason)
        }
        _ => None,
    }
}

fn assert_round_trip(text: &str) {
    let parsed = TypeExpr::parse(text);
    assert!(parsed.is_ok(), "{text} must parse");
    assert_eq!(parsed.ok().map(|t| t.to_string()), Some(text.to_owned()));
}

#[test]
fn type_expr_examples_round_trip() {
    for text in [
        "INT",
        "STRING",
        "STRING(20)",
        "WSTRING(80)",
        "ST_Zone@3",
        "ARRAY[1..8] OF REAL",
        "ARRAY[0..1,1..2] OF ST_Zone@3",
        "ARRAY[1..4] OF STRING(16)",
        "ARRAY[1..2] OF ARRAY[1..3] OF INT",
    ] {
        assert_round_trip(text);
    }
}

#[test]
fn type_expr_nested_array_has_two_prefixes() {
    let parsed = TypeExpr::parse("ARRAY[1..2] OF ARRAY[1..3] OF INT");
    assert_eq!(parsed.ok().map(|t| t.arrays().len()), Some(2));
}

#[test]
fn type_expr_base_of_string_without_length_is_none() {
    let parsed = TypeExpr::parse("STRING");
    assert_eq!(
        parsed.ok().map(|t| t.base().clone()),
        Some(BaseExpr::String {
            kind: StringKind::String,
            length: None,
        })
    );
}

#[test]
fn type_expr_udt_without_at_is_grammar_error() {
    assert_eq!(reason_of("ST_Zone"), Some(DerivedTypeReason::Grammar));
}

#[test]
fn type_expr_missing_of_is_grammar_error() {
    assert_eq!(
        reason_of("ARRAY[1..8] INT"),
        Some(DerivedTypeReason::Grammar)
    );
}

#[test]
fn type_expr_four_dimensions_is_grammar_error() {
    assert_eq!(
        reason_of("ARRAY[1..2,1..2,1..2,1..2] OF INT"),
        Some(DerivedTypeReason::Grammar)
    );
}

#[test]
fn type_expr_reversed_bounds_is_out_of_range() {
    assert_eq!(
        reason_of("ARRAY[5..1] OF INT"),
        Some(DerivedTypeReason::OutOfRange)
    );
}

#[test]
fn type_expr_span_of_65536_is_out_of_range() {
    assert_eq!(
        reason_of("ARRAY[0..65536] OF INT"),
        Some(DerivedTypeReason::OutOfRange)
    );
}

#[test]
fn type_expr_string_length_above_u16_is_out_of_range() {
    assert_eq!(
        reason_of("STRING(99999)"),
        Some(DerivedTypeReason::OutOfRange)
    );
}

#[test]
fn type_expr_revision_above_integer_range_is_unknown_struct() {
    assert_eq!(
        reason_of("ST_Zone@4294967296"),
        Some(DerivedTypeReason::UnknownStruct)
    );
    assert_eq!(
        reason_of("ST_Zone@2147483648"),
        Some(DerivedTypeReason::UnknownStruct)
    );
}

#[test]
fn type_expr_revision_at_integer_max_parses() {
    assert_round_trip("ST_Zone@2147483647");
}

#[test]
fn type_expr_lower_bound_above_integer_max_is_out_of_range() {
    assert_eq!(
        reason_of("ARRAY[2147483648..1] OF INT"),
        Some(DerivedTypeReason::OutOfRange)
    );
}

#[test]
fn type_expr_lower_bound_below_integer_min_is_out_of_range() {
    assert_eq!(
        reason_of("ARRAY[-2147483649..1] OF INT"),
        Some(DerivedTypeReason::OutOfRange)
    );
}

#[test]
fn type_expr_bound_with_eleven_digits_is_grammar_error() {
    assert_eq!(
        reason_of("ARRAY[12345678901..1] OF INT"),
        Some(DerivedTypeReason::Grammar)
    );
}

#[test]
fn type_expr_malformed_texts_are_grammar_errors() {
    for text in [
        "STRING(0)",
        "STRING(020)",
        "STRING()",
        "ST_Zone@0",
        "ST_Zone@01",
        "ST_Zone",
        "1ST@1",
        "int",
        "DT",
        "array[1..2] OF INT",
        "ARRAY[1..2]OF INT",
        "ARRAY [1..2] OF INT",
        "ARRAY[1..2] OF  INT",
        "ARRAY[] OF INT",
        "ARRAY[1..2,] OF INT",
        "ARRAY[1..2] OF ",
        "",
    ] {
        assert_eq!(reason_of(text), Some(DerivedTypeReason::Grammar), "{text}");
    }
}

#[test]
fn type_expr_primitive_name_with_at_is_unknown_struct() {
    assert_eq!(reason_of("INT@1"), Some(DerivedTypeReason::UnknownStruct));
}

#[test]
fn type_expr_reversed_bounds_with_leading_zeros_is_out_of_range() {
    assert_eq!(
        reason_of("ARRAY[007..-0] OF INT"),
        Some(DerivedTypeReason::OutOfRange)
    );
}

#[test]
fn type_expr_negative_zero_bound_displays_canonical_number() {
    let parsed = TypeExpr::parse("ARRAY[-0..007] OF INT");
    assert_eq!(
        parsed.ok().map(|t| t.to_string()),
        Some("ARRAY[0..7] OF INT".to_owned())
    );
}

fn depth_error(text: &str) -> bool {
    matches!(
        TypeExpr::parse(text),
        Err(PlantError::TypeTooLarge {
            limit: TypeLimit::Depth,
            ..
        })
    )
}

#[test]
fn type_expr_32_one_dimensional_prefixes_parse() {
    let text = format!("{}INT", "ARRAY[1..1] OF ".repeat(32));
    let parsed = TypeExpr::parse(&text);
    assert_eq!(parsed.as_ref().ok().map(|t| t.arrays().len()), Some(32));
    assert_eq!(parsed.ok().map(|t| t.to_string()), Some(text));
}

#[test]
fn type_expr_33_one_dimensional_prefixes_are_too_deep() {
    assert!(depth_error(&format!("{}INT", "ARRAY[1..1] OF ".repeat(33))));
}

#[test]
fn type_expr_ten_thousand_prefixes_are_too_deep_without_recursion() {
    assert!(depth_error(&format!(
        "{}INT",
        "ARRAY[1..1] OF ".repeat(10_000)
    )));
}

#[test]
fn type_expr_eleven_three_dimensional_prefixes_are_too_deep() {
    let prefix = "ARRAY[1..1,1..1,1..1] OF ";
    assert!(!depth_error(&format!("{}INT", prefix.repeat(10))));
    assert!(depth_error(&format!("{}INT", prefix.repeat(11))));
}

#[test]
fn type_expr_too_deep_error_carries_the_input_text() {
    let text = format!("{}INT", "ARRAY[1..1] OF ".repeat(33));
    assert!(matches!(
        TypeExpr::parse(&text),
        Err(PlantError::TypeTooLarge { type_name, .. }) if type_name == text
    ));
}

#[test]
fn type_expr_hundred_thousand_commas_are_a_grammar_error() {
    let text = format!("ARRAY[{}] OF INT", ",".repeat(100_000));
    assert_eq!(reason_of(&text), Some(DerivedTypeReason::Grammar));
}

fn dimension() -> impl Strategy<Value = String> {
    (-100_i32..=100, -100_i32..=100).prop_map(|(a, b)| {
        let (lower, upper) = (a.min(b), a.max(b));
        format!("{lower}..{upper}")
    })
}

fn array_prefix() -> impl Strategy<Value = String> {
    prop::collection::vec(dimension(), 1..=3)
        .prop_map(|dims| format!("ARRAY[{}] OF ", dims.join(",")))
}

fn base() -> impl Strategy<Value = String> {
    let primitives: Vec<&'static str> = Primitive::ALL
        .into_iter()
        .filter(|p| !matches!(p, Primitive::String | Primitive::Wstring))
        .map(Primitive::name)
        .collect();
    let primitive = prop::sample::select(primitives).prop_map(str::to_owned);
    let string = (
        prop_oneof![Just("STRING"), Just("WSTRING")],
        prop::option::of(1_u16..=16_382),
    )
        .prop_map(|(keyword, length)| match length {
            Some(length) => format!("{keyword}({length})"),
            None => keyword.to_owned(),
        });
    let udt = ("[A-Za-z_][A-Za-z0-9_]{0,10}", 1_u32..=2_147_483_647)
        .prop_filter("name is not a primitive", |(name, _)| {
            Primitive::parse(name).is_err()
        })
        .prop_map(|(name, revision)| format!("{name}@{revision}"));
    prop_oneof![primitive, string, udt]
}

proptest! {
    #[test]
    fn type_expr_generated_text_round_trips(
        prefixes in prop::collection::vec(array_prefix(), 0..=4),
        base_text in base(),
    ) {
        let text = format!("{}{base_text}", prefixes.concat());
        match TypeExpr::parse(&text) {
            Ok(parsed) => prop_assert_eq!(parsed.to_string(), text),
            Err(error) => prop_assert!(false, "{text} failed: {error}"),
        }
    }
}
