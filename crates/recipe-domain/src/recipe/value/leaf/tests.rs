use serde_json::json;

use crate::plant::fixtures::{
    id, machine, on_platform, primitive_row, string_row, types_for, wstring_row,
};
use crate::plant::{LeafPath, PlcPlatform, Primitive, RuleSet, TypeTree};
use crate::recipe::value::test_support::{
    BOOL, BOUNDARIES, CHAR, DATE, INT, LREAL, LTIME, REAL, TIME, WCHAR, integer_tree,
    primitive_tree, string_tree, wstring_tree, zone_tree,
};
use crate::recipe::value::{Expected, ParameterValue, StoredValue, ValueFault};

/// Parse `json` and return the stored JSON text.
fn stored_text(json: &str, tree: &TypeTree) -> String {
    ParameterValue::parse(json, tree)
        .unwrap()
        .as_json()
        .to_string()
}

/// Parse `json` and return the fault.
fn fault(json: &str, tree: &TypeTree) -> ValueFault {
    ParameterValue::parse(json, tree).unwrap_err()
}

#[test]
fn string_with_a_character_outside_latin_1_gives_a_fault() {
    let fault = fault(r#""Łódź""#, &string_tree(10));
    assert_eq!(fault.leaf().to_string(), "");
    assert_eq!(fault.expected(), Some(&Expected::String { length: 10 }));
}

#[test]
fn string_with_latin_1_characters_passes() {
    assert_eq!(stored_text(r#""Zéàü""#, &string_tree(10)), r#""Zéàü""#);
}

#[test]
fn string_longer_than_its_type_gives_a_fault() {
    assert!(ParameterValue::parse(r#""ABCDE""#, &string_tree(4)).is_err());
}

#[test]
fn string_as_long_as_its_type_passes() {
    assert_eq!(stored_text(r#""ABCD""#, &string_tree(4)), r#""ABCD""#);
}

#[test]
fn string_length_counts_characters_and_not_bytes() {
    assert_eq!(stored_text(r#""éééé""#, &string_tree(4)), r#""éééé""#);
}

#[test]
fn number_for_a_string_gives_a_fault() {
    assert!(ParameterValue::parse("5", &string_tree(4)).is_err());
}

#[test]
fn wstring_counts_utf16_units() {
    let fault = fault(r#""a😀""#, &wstring_tree(2));
    assert_eq!(fault.expected(), Some(&Expected::Wstring { length: 2 }));
}

#[test]
fn wstring_with_one_surrogate_pair_fits_two_units() {
    assert_eq!(stored_text(r#""😀""#, &wstring_tree(2)), r#""😀""#);
}

#[test]
fn wstring_accepts_characters_outside_latin_1() {
    assert_eq!(stored_text(r#""Łódź""#, &wstring_tree(4)), r#""Łódź""#);
}

#[test]
fn number_and_text_for_a_bool_give_a_fault() {
    let tree = primitive_tree(BOOL);
    assert_eq!(fault("1", &tree).expected(), Some(&Expected::Bool));
    assert!(ParameterValue::parse(r#""true""#, &tree).is_err());
    assert!(ParameterValue::parse("null", &tree).is_err());
}

#[test]
fn bool_values_pass() {
    let tree = primitive_tree(BOOL);
    assert_eq!(stored_text("true", &tree), "true");
    assert_eq!(stored_text("false", &tree), "false");
}

#[test]
fn char_takes_exactly_one_scalar_up_to_latin_1() {
    let tree = primitive_tree(CHAR);
    assert!(ParameterValue::parse(r#""ab""#, &tree).is_err());
    assert!(ParameterValue::parse(r#""""#, &tree).is_err());
    assert!(ParameterValue::parse(r#""Ł""#, &tree).is_err());
    assert_eq!(stored_text(r#""é""#, &tree), r#""é""#);
}

#[test]
fn wchar_takes_one_scalar_up_to_the_basic_plane() {
    let tree = primitive_tree(WCHAR);
    assert!(ParameterValue::parse(r#""😀""#, &tree).is_err());
    assert_eq!(stored_text(r#""Ł""#, &tree), r#""Ł""#);
}

#[test]
fn char_and_wchar_refuse_u0000() {
    assert_eq!(
        fault(r#""\u0000""#, &primitive_tree(CHAR)).expected(),
        Some(&Expected::Char { max: 0xFF })
    );
    assert_eq!(
        fault(r#""\u0000""#, &primitive_tree(WCHAR)).expected(),
        Some(&Expected::Char { max: 0xFFFF })
    );
}

#[test]
fn string_refuses_u0000_at_any_position() {
    assert_eq!(
        fault(r#""AB\u0000""#, &string_tree(10)).expected(),
        Some(&Expected::String { length: 10 })
    );
}

#[test]
fn wstring_refuses_u0000_at_any_position() {
    assert_eq!(
        fault(r#""A\u0000""#, &wstring_tree(10)).expected(),
        Some(&Expected::Wstring { length: 10 })
    );
}

#[test]
fn u0001_is_the_lowest_text_scalar_that_passes() {
    assert_eq!(
        stored_text(r#""\u0001""#, &primitive_tree(CHAR)),
        r#""\u0001""#
    );
    assert_eq!(
        stored_text(r#""A\u0001B""#, &string_tree(10)),
        r#""A\u0001B""#
    );
}

#[test]
fn number_for_a_char_gives_a_fault() {
    assert!(ParameterValue::parse("65", &primitive_tree(CHAR)).is_err());
}

#[test]
fn time_literal_is_stored_as_a_count() {
    assert_eq!(stored_text(r#""T#90m""#, &primitive_tree(TIME)), "5400000");
}

#[test]
fn time_literal_with_parts_in_the_wrong_order_gives_a_fault() {
    let fault = fault(r#""T#2h1d""#, &primitive_tree(TIME));
    assert_eq!(
        fault.expected(),
        Some(&Expected::Literal {
            type_name: Primitive::Time,
        })
    );
}

#[test]
fn time_literal_with_the_prefix_of_another_type_gives_a_fault() {
    assert!(ParameterValue::parse(r#""LT#5s""#, &primitive_tree(TIME)).is_err());
}

#[test]
fn negative_time_on_twincat3_gives_a_fault() {
    assert!(ParameterValue::parse(r#""T#-1s""#, &primitive_tree(TIME)).is_err());
}

#[test]
fn ltime_literal_is_stored_as_a_count_of_nanoseconds() {
    assert_eq!(
        stored_text(r#""LT#5s""#, &primitive_tree(LTIME)),
        "5000000000"
    );
}

#[test]
fn date_literal_is_stored_as_a_count() {
    assert_eq!(
        stored_text(r#""D#2026-10-12""#, &primitive_tree(DATE)),
        "20738"
    );
}

#[test]
fn number_for_a_time_leaf_gives_a_fault_in_a_request() {
    assert!(ParameterValue::parse("5400000", &primitive_tree(TIME)).is_err());
}

#[test]
fn real_takes_one_significant_digit_with_a_large_exponent() {
    assert_eq!(
        stored_text("1e28", &primitive_tree(REAL)),
        "10000000000000000000000000000"
    );
}

#[test]
fn real_with_29_fraction_digits_gives_a_mismatch_fault() {
    let fault = fault("1e-29", &primitive_tree(REAL));
    assert!(
        matches!(&fault, ValueFault::Mismatch { leaf, .. } if leaf.is_empty()),
        "{fault:?}"
    );
}

#[test]
fn real_with_29_significant_digits_gives_a_fault() {
    assert!(
        ParameterValue::parse("1.2345678901234567890123456789", &primitive_tree(REAL)).is_err()
    );
}

#[test]
fn real_with_29_significant_digits_in_scientific_form_gives_a_fault() {
    let tree = primitive_tree(REAL);
    assert!(ParameterValue::parse("1.00000000000000000000000000001e0", &tree).is_err());
}

/// The fault of a number with too many significant digits (ADR-0117).
fn precision(type_name: Primitive, max_digits: u8) -> ValueFault {
    ValueFault::Precision {
        leaf: LeafPath::whole(),
        type_name,
        max_digits,
    }
}

/// Read a stored number under `rules`.
fn stored_under(
    json: serde_json::Value,
    tree: &TypeTree,
    rules: RuleSet,
) -> Result<ParameterValue, ValueFault> {
    ParameterValue::from_stored(&StoredValue::from(json), tree, rules)
}

#[test]
fn adr_0117_real_16777217_gives_a_precision_fault_in_a_request() {
    let fault = fault("16777217", &primitive_tree(REAL));
    assert_eq!(fault, precision(Primitive::Real, 7));
}

#[test]
fn adr_0117_lreal_with_16_digits_gives_a_precision_fault_in_a_request() {
    let fault = fault("0.1234567890123456", &primitive_tree(LREAL));
    assert_eq!(fault, precision(Primitive::Lreal, 15));
}

#[test]
fn adr_0117_max_significant_digits_is_7_for_real_15_for_lreal_and_none_for_dint() {
    assert_eq!(super::max_significant_digits(Primitive::Real), Some(7));
    assert_eq!(super::max_significant_digits(Primitive::Lreal), Some(15));
    assert_eq!(super::max_significant_digits(Primitive::Dint), None);
}

#[test]
fn adr_0117_real_1234_567_and_lreal_with_15_digits_pass() {
    assert_eq!(stored_text("1234.567", &primitive_tree(REAL)), "1234.567");
    assert_eq!(
        stored_text("0.123456789012345", &primitive_tree(LREAL)),
        "0.123456789012345"
    );
}

#[test]
fn adr_0117_trailing_zeros_are_not_significant_digits() {
    assert_eq!(
        stored_text("1000000000", &primitive_tree(REAL)),
        "1000000000"
    );
}

#[test]
fn adr_0117_the_current_set_rejects_a_stored_real_16777217() {
    let result = stored_under(json!(16_777_217), &primitive_tree(REAL), RuleSet::Current);
    assert_eq!(result.unwrap_err(), precision(Primitive::Real, 7));
}

#[test]
fn adr_0117_the_earlier_set_accepts_a_stored_real_16777217() {
    let value = stored_under(json!(16_777_217), &primitive_tree(REAL), RuleSet::Earlier).unwrap();
    assert_eq!(value.as_json().to_string(), "16777217");
}

#[test]
fn adr_0117_the_earlier_set_accepts_a_stored_lreal_with_16_digits() {
    let tree = primitive_tree(LREAL);
    let json: serde_json::Value = "0.1234567890123456".parse().unwrap();
    assert!(stored_under(json.clone(), &tree, RuleSet::Earlier).is_ok());
    assert_eq!(
        stored_under(json, &tree, RuleSet::Current).unwrap_err(),
        precision(Primitive::Lreal, 15)
    );
}

#[test]
fn adr_0117_a_precision_fault_names_the_leaf_of_a_struct() {
    let json = r#"{"temp_sp": 16777217, "active": true, "ramps": [10, 20, 30], "label": "Zone A"}"#;
    let fault = fault(json, &zone_tree());
    assert_eq!(fault.leaf().to_string(), ".temp_sp");
    assert!(matches!(fault, ValueFault::Precision { .. }));
}

#[test]
fn parse_decimal_follows_the_json_number_grammar() {
    use crate::recipe::value::parse_decimal;
    for text in [
        "1_000", "+5", ".5", "5.", "01", "", "-", "1e", "1e+", "0x10", " 5",
    ] {
        assert!(parse_decimal(text).is_none(), "{text:?}");
    }
    for text in ["0", "-0", "10", "-1.5", "1e2", "1E+2", "2.5e-3"] {
        assert!(parse_decimal(text).is_some(), "{text:?}");
    }
}

#[test]
fn exact_decimal_with_trailing_zeros_below_the_scale_limit_is_stored_exact() {
    let tree = primitive_tree(REAL);
    assert_eq!(
        stored_text("150e-29", &tree),
        "0.0000000000000000000000000015"
    );
    assert_eq!(
        stored_text("100e-30", &tree),
        "0.0000000000000000000000000001"
    );
}

#[test]
fn fraction_of_29_zeros_is_stored_as_the_whole_number() {
    let json = format!("1.{}", "0".repeat(29));
    assert_eq!(stored_text(&json, &primitive_tree(INT)), "1");
}

#[test]
fn zero_with_an_exponent_above_the_range_of_i64_is_stored_as_zero() {
    let tree = primitive_tree(REAL);
    assert_eq!(stored_text("0e99999999999999999999", &tree), "0");
    assert_eq!(stored_text("-0e-99999999999999999999", &tree), "0");
}

#[test]
fn integer_types_accept_their_ends_and_refuse_the_values_next_to_them() {
    for row in &BOUNDARIES {
        let tree = integer_tree(row.primitive);
        let expected = Expected::NumberRange {
            type_name: row.primitive,
            low: row.low.parse().unwrap(),
            high: row.high.parse().unwrap(),
        };
        let name = row.primitive;
        assert_eq!(stored_text(row.low, &tree), row.low, "{name} min");
        assert_eq!(stored_text(row.high, &tree), row.high, "{name} max");
        for text in [row.below_low, row.above_high] {
            let refused = ValueFault::Mismatch {
                leaf: LeafPath::whole(),
                expected: expected.clone(),
            };
            assert_eq!(fault(text, &tree), refused, "{name} {text}");
        }
    }
}

/// The tree of one `WCHAR` parameter on `platform`.
fn wchar_tree_on(platform: PlcPlatform) -> TypeTree {
    let rows = on_platform(platform, vec![primitive_row(WCHAR, Primitive::Wchar)]);
    TypeTree::resolve(id(WCHAR), &types_for(machine(), platform, rows)).unwrap()
}

/// The tree of one `WSTRING(length)` parameter on `platform`.
fn wstring_tree_on(platform: PlcPlatform, length: u16) -> TypeTree {
    let rows = on_platform(platform, vec![wstring_row(50, length)]);
    TypeTree::resolve(id(50), &types_for(machine(), platform, rows)).unwrap()
}

/// The tree of one `STRING(length)` parameter on `platform`.
fn string_tree_on(platform: PlcPlatform, length: u16) -> TypeTree {
    let rows = on_platform(platform, vec![string_row(50, length)]);
    TypeTree::resolve(id(50), &types_for(machine(), platform, rows)).unwrap()
}

const S7_PLATFORMS: [PlcPlatform; 2] = [PlcPlatform::S7_1200, PlcPlatform::S7_1500];

/// The `WSTRING` form that `E-RCP-005` names on S7 (ADR-0180 (d)).
fn s7_wstring(length: u16) -> Expected {
    Expected::WstringS7 {
        length,
        max: 0xCFFF,
    }
}

fn request_under(
    json: &str,
    tree: &TypeTree,
    rules: RuleSet,
) -> Result<ParameterValue, ValueFault> {
    let stored = serde_json::from_str::<serde_json::Value>(json).unwrap();
    ParameterValue::from_stored(&StoredValue::from(stored), tree, rules)
}

#[test]
fn adr_0180_wchar_on_s7_accepts_u_d7ff_and_refuses_u_e000() {
    for platform in S7_PLATFORMS {
        let tree = wchar_tree_on(platform);
        assert_eq!(stored_text(r#""퟿""#, &tree), "\"\u{D7FF}\"");
        assert_eq!(
            fault(r#""""#, &tree).expected(),
            Some(&Expected::Char { max: 0xD7FF })
        );
    }
}

#[test]
fn adr_0180_wstring_on_s7_accepts_u_cfff_and_refuses_u_d000_and_u_1f600() {
    for platform in S7_PLATFORMS {
        let tree = wstring_tree_on(platform, 10);
        assert_eq!(stored_text(r#""A쿿""#, &tree), "\"A\u{CFFF}\"");
        for text in [r#""A퀀""#, r#""😀""#] {
            assert_eq!(
                fault(text, &tree).expected(),
                Some(&s7_wstring(10)),
                "{text}"
            );
        }
    }
}

#[test]
fn adr_0180_the_earlier_set_accepts_the_values_that_the_current_set_refuses() {
    for platform in S7_PLATFORMS {
        let wchar = wchar_tree_on(platform);
        assert!(request_under(r#""""#, &wchar, RuleSet::Earlier).is_ok());
        assert!(request_under(r#""""#, &wchar, RuleSet::Current).is_err());
        let wstring = wstring_tree_on(platform, 10);
        for text in [r#""A퀀""#, r#""😀""#] {
            assert!(
                request_under(text, &wstring, RuleSet::Earlier).is_ok(),
                "{text}"
            );
            assert_eq!(
                request_under(text, &wstring, RuleSet::Current)
                    .unwrap_err()
                    .expected(),
                Some(&s7_wstring(10)),
                "{text}"
            );
        }
    }
}

#[test]
fn adr_0180_the_earlier_set_still_refuses_u0000_in_a_wstring() {
    let tree = wstring_tree_on(PlcPlatform::S7_1500, 10);
    assert!(request_under(r#""A\u0000""#, &tree, RuleSet::Earlier).is_err());
}

#[test]
fn adr_0180_wstring_on_codesys_and_twincat_keeps_adr_0137() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        let tree = wstring_tree_on(platform, 10);
        assert_eq!(stored_text(r#""A퀀""#, &tree), "\"A\u{D000}\"");
        assert_eq!(stored_text(r#""😀""#, &tree), r#""😀""#);
        assert_eq!(
            fault(r#""A\u0000""#, &tree).expected(),
            Some(&Expected::Wstring { length: 10 })
        );
    }
}

#[test]
fn adr_0180_wstring_on_s7_keeps_the_length_in_utf16_units() {
    let tree = wstring_tree_on(PlcPlatform::S7_1500, 2);
    assert_eq!(fault(r#""ABC""#, &tree).expected(), Some(&s7_wstring(2)));
    assert_eq!(stored_text(r#""AB""#, &tree), r#""AB""#);
}

#[test]
fn adr_0180_a_non_string_value_of_an_s7_wstring_gives_the_s7_form() {
    let tree = wstring_tree_on(PlcPlatform::S7_1200, 4);
    assert_eq!(fault("5", &tree).expected(), Some(&s7_wstring(4)));
}

/// ADR-0137 pin for review item R04. Every machine of this wave has the encoding `Latin1`.
/// U+00FF is the last Latin-1 character and U+0100 is the first scalar outside it.
#[test]
fn adr_0137_last_latin_1_character() {
    let platforms = [
        PlcPlatform::S7_1500,
        PlcPlatform::CodesysIec,
        PlcPlatform::TwinCat3,
    ];
    for platform in platforms {
        let tree = string_tree_on(platform, 1);
        assert_eq!(
            stored_text(r#""\u00FF""#, &tree),
            "\"\u{FF}\"",
            "{platform:?}"
        );
        assert_eq!(
            fault(r#""\u0100""#, &tree).expected(),
            Some(&Expected::String { length: 1 }),
            "{platform:?}"
        );
    }
}
