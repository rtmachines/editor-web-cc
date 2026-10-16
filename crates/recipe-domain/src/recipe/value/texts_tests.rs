//! Tests of `LeafTexts`, `ParameterValue::from_texts`, and `ParameterValue::texts` (ADR-0039).

use serde_json::{Value, json};

use crate::plant::fixtures::{id, machine, primitive_row, struct_row, types_for};
use crate::plant::{LeafClass, LeafPath, PlcPlatform, Primitive, TypeTree};
use crate::recipe::value::test_support::{
    BOOL, CHAR, DATE, INT, REAL, TIME, USINT, WCHAR, ZONE_JSON, int_array_tree, primitive_tree,
    string_tree, tree, wstring_tree, zone_tree,
};
use crate::recipe::value::{
    Expected, LeafText, LeafTexts, LeafTextsError, ParameterValue, ValueFault,
};

fn path(text: &str) -> LeafPath {
    LeafPath::parse(text).unwrap()
}

fn pairs(pairs: &[(&str, &str)]) -> Vec<(LeafPath, String)> {
    pairs
        .iter()
        .map(|(leaf, text)| (path(leaf), (*text).to_owned()))
        .collect()
}

fn texts(given: &[(&str, &str)]) -> LeafTexts {
    LeafTexts::parse(pairs(given)).unwrap()
}

/// Build the value of `tree` from the texts and return its JSON.
fn built(tree: &TypeTree, given: &[(&str, &str)]) -> Value {
    ParameterValue::from_texts(tree, &texts(given))
        .unwrap()
        .as_json()
        .clone()
}

/// Build the value of `tree` from the texts and return each fault as path and expectation.
fn faults(tree: &TypeTree, given: &[(&str, &str)]) -> Vec<(String, Expected)> {
    ParameterValue::from_texts(tree, &texts(given))
        .unwrap_err()
        .into_vec()
        .into_iter()
        .map(|fault| {
            let expected = fault.expected().cloned().unwrap();
            (fault.leaf().to_string(), expected)
        })
        .collect()
}

/// The tree of a struct with the members `setpoint` (REAL) and `hold` (TIME).
fn zone_with_hold() -> TypeTree {
    tree(
        vec![struct_row(
            60,
            "ST_Hold",
            &[("setpoint", REAL), ("hold", TIME)],
        )],
        60,
    )
}

#[test]
fn parse_refuses_a_duplicate_path() {
    let error = LeafTexts::parse(pairs(&[("[1]", "1"), ("[1]", "2")])).unwrap_err();
    assert_eq!(error, LeafTextsError::DuplicatePath { path: path("[1]") });
}

#[test]
fn parse_refuses_an_all_elements_segment() {
    let error = LeafTexts::parse(pairs(&[("[*]", "1")])).unwrap_err();
    assert_eq!(error, LeafTextsError::AllElements { path: path("[*]") });
}

#[test]
fn parse_keeps_the_order_of_the_pairs() {
    let texts = texts(&[("[2]", "b"), ("[1]", "a")]);
    let order: Vec<String> = texts.iter().map(|(leaf, _)| leaf.to_string()).collect();
    assert_eq!(order, ["[2]", "[1]"]);
    assert_eq!(texts.len(), 2);
}

#[test]
fn numeric_text_with_a_dot_builds_a_canonical_number() {
    let real = primitive_tree(REAL);
    assert_eq!(built(&real, &[("", "1.50")]).to_string(), "1.5");
    assert_eq!(built(&real, &[("", "1.5e2")]).to_string(), "150");
    assert_eq!(built(&primitive_tree(INT), &[("", "-0")]).to_string(), "0");
    assert_eq!(built(&primitive_tree(INT), &[("", "7")]), json!(7));
}

#[test]
fn numeric_text_without_the_json_number_grammar_expects_a_decimal() {
    let real = primitive_tree(REAL);
    for text in ["1,5", "abc", "", " 5", "+5", ".5", "0x10"] {
        assert_eq!(
            faults(&real, &[("", text)]),
            vec![(String::new(), Expected::Decimal)],
            "{text:?}"
        );
    }
}

#[test]
fn integer_text_without_the_json_number_grammar_expects_an_integer() {
    for (primitive, type_name) in [(INT, Primitive::Int), (USINT, Primitive::Usint)] {
        let tree = primitive_tree(primitive);
        for text in ["", "12,5", "abc", "+5"] {
            assert_eq!(
                faults(&tree, &[("", text)]),
                vec![(String::new(), Expected::Integer { type_name })],
                "{type_name:?} {text:?}"
            );
        }
    }
}

#[test]
fn numeric_text_keeps_the_range_and_integer_rules_of_parse() {
    let int = primitive_tree(INT);
    assert_eq!(
        faults(&int, &[("", "40000")]),
        vec![(
            String::new(),
            Expected::NumberRange {
                type_name: Primitive::Int,
                low: rust_decimal::Decimal::from(-32_768),
                high: rust_decimal::Decimal::from(32_767),
            }
        )]
    );
    assert_eq!(
        faults(&int, &[("", "1.5")]),
        vec![(
            String::new(),
            Expected::Integer {
                type_name: Primitive::Int,
            }
        )]
    );
}

#[test]
fn bool_text_is_true_or_false() {
    let tree = primitive_tree(BOOL);
    assert_eq!(built(&tree, &[("", "true")]), json!(true));
    assert_eq!(built(&tree, &[("", "false")]), json!(false));
    for text in ["TRUE", "1", "yes", ""] {
        assert_eq!(
            faults(&tree, &[("", text)]),
            vec![(String::new(), Expected::Bool)],
            "{text:?}"
        );
    }
}

#[test]
fn string_and_char_texts_are_kept_as_sent() {
    assert_eq!(built(&string_tree(10), &[("", " Zé ")]), json!(" Zé "));
    assert_eq!(built(&wstring_tree(4), &[("", "Łódź")]), json!("Łódź"));
    assert_eq!(built(&primitive_tree(CHAR), &[("", "é")]), json!("é"));
    assert_eq!(built(&primitive_tree(WCHAR), &[("", "Ł")]), json!("Ł"));
    assert_eq!(
        faults(&string_tree(4), &[("", "ABCDE")]),
        vec![(String::new(), Expected::String { length: 4 })]
    );
    assert_eq!(
        faults(&primitive_tree(CHAR), &[("", "ab")]),
        vec![(String::new(), Expected::Char { max: 0xFF })]
    );
}

#[test]
fn time_text_is_an_iec_literal_and_builds_the_count() {
    assert_eq!(
        built(&primitive_tree(TIME), &[("", "T#1m30s")]),
        json!(90_000)
    );
    assert_eq!(
        built(&primitive_tree(DATE), &[("", "D#2026-10-12")]),
        json!(20_738)
    );
    assert_eq!(
        faults(&primitive_tree(TIME), &[("", "90000")]),
        vec![(
            String::new(),
            Expected::Literal {
                type_name: Primitive::Time,
            }
        )]
    );
}

#[test]
fn missing_leaf_expects_a_value_for_the_field() {
    let tree = int_array_tree(&[(1, 8)]);
    let given = [
        ("[1]", "1"),
        ("[2]", "2"),
        ("[3]", "3"),
        ("[4]", "4"),
        ("[5]", "5"),
        ("[6]", "6"),
        ("[7]", "7"),
    ];
    assert_eq!(
        faults(&tree, &given),
        vec![("[8]".to_owned(), Expected::Leaf)]
    );
}

#[test]
fn unknown_path_expects_a_known_leaf() {
    let tree = primitive_tree(INT);
    assert_eq!(
        faults(&tree, &[("", "1"), (".x", "2")]),
        vec![(".x".to_owned(), Expected::KnownLeaf)]
    );
}

#[test]
fn faults_come_in_preorder_and_then_the_unknown_paths() {
    let tree = int_array_tree(&[(1, 3)]);
    let given = [(".zz", "1"), ("[1]", "x"), ("[2]", "2")];
    assert_eq!(
        faults(&tree, &given),
        vec![
            (
                "[1]".to_owned(),
                Expected::Integer {
                    type_name: Primitive::Int,
                },
            ),
            ("[3]".to_owned(), Expected::Leaf),
            (".zz".to_owned(), Expected::KnownLeaf),
        ]
    );
}

#[test]
fn struct_value_from_texts_equals_the_parsed_json() {
    let tree = zone_tree();
    let given = [
        (".temp_sp", "85.5"),
        (".active", "true"),
        (".ramps[1]", "10"),
        (".ramps[2]", "20"),
        (".ramps[3]", "30"),
        (".label", "Zone A"),
    ];
    let value = ParameterValue::from_texts(&tree, &texts(&given)).unwrap();
    assert_eq!(value, ParameterValue::parse(ZONE_JSON, &tree).unwrap());
}

#[test]
fn texts_of_a_value_write_the_canonical_text_and_the_class_of_each_leaf() {
    let tree = zone_tree();
    let value = ParameterValue::parse(ZONE_JSON, &tree).unwrap();
    let leaf = |leaf: &str, class, text: &str| LeafText {
        path: path(leaf),
        class,
        text: text.to_owned(),
    };
    assert_eq!(
        value.texts(&tree).unwrap(),
        vec![
            leaf(".temp_sp", LeafClass::Numeric, "85.5"),
            leaf(".active", LeafClass::Bool, "true"),
            leaf(".ramps[1]", LeafClass::Numeric, "10"),
            leaf(".ramps[2]", LeafClass::Numeric, "20"),
            leaf(".ramps[3]", LeafClass::Numeric, "30"),
            leaf(".label", LeafClass::String, "Zone A"),
        ]
    );
}

#[test]
fn texts_write_a_time_leaf_as_the_canonical_literal_and_round_trip() {
    let tree = zone_with_hold();
    let value = ParameterValue::parse(r#"{"setpoint": 182.50, "hold": "T#90s"}"#, &tree).unwrap();
    let texts = value.texts(&tree).unwrap();
    let written: Vec<(String, String)> = texts
        .iter()
        .map(|leaf| (leaf.text.clone(), format!("{:?}", leaf.class)))
        .collect();
    assert_eq!(
        written,
        [
            ("182.5".to_owned(), "Numeric".to_owned()),
            ("T#1m30s".to_owned(), "Duration".to_owned()),
        ]
    );
    let again = texts
        .into_iter()
        .map(|leaf| (leaf.path, leaf.text))
        .collect();
    let rebuilt = ParameterValue::from_texts(&tree, &LeafTexts::parse(again).unwrap()).unwrap();
    assert_eq!(rebuilt, value);
}

/// The row number of the one primitive of [`primitive_on`].
const ROW: u128 = 1;

/// The tree of one fixed-size primitive on `platform`.
fn primitive_on(platform: PlcPlatform, primitive: Primitive) -> TypeTree {
    let rows = vec![primitive_row(ROW, primitive)];
    TypeTree::resolve(id(ROW), &types_for(machine(), platform, rows)).unwrap()
}

/// Turn the texts of a page into the pairs that a form sends back.
fn texts_of(texts: Vec<LeafText>) -> LeafTexts {
    LeafTexts::parse(
        texts
            .into_iter()
            .map(|leaf| (leaf.path, leaf.text))
            .collect(),
    )
    .unwrap()
}

/// Parse `request`, write its texts, and build the value again from the texts.
///
/// The function asserts the round trip and returns the class and the text of the one leaf.
fn one_leaf_round_trip(tree: &TypeTree, request: &str) -> (LeafClass, String) {
    let value = ParameterValue::parse(request, tree).unwrap();
    let texts = value.texts(tree).unwrap();
    assert_eq!(texts.len(), 1, "{request}");
    let rebuilt = ParameterValue::from_texts(tree, &texts_of(texts.clone())).unwrap();
    assert_eq!(rebuilt, value, "{request}");
    let leaf = texts.into_iter().next().unwrap();
    assert_eq!(leaf.path, path(""), "{request}");
    (leaf.class, leaf.text)
}

/// One case: the platform, the primitive, the request JSON, and the text of the leaf.
///
/// Each platform has the primitive in the matrix of `data-model.md` section 7.
type Case = (PlcPlatform, Primitive, &'static str, &'static str);

/// `CHAR` and `WCHAR` exist on S7 only.
const CHAR_CASES: [Case; 2] = [
    (PlcPlatform::S7_1500, Primitive::Char, r#""é""#, "é"),
    (PlcPlatform::S7_1200, Primitive::Wchar, r#""Ł""#, "Ł"),
];

/// One case per instant primitive. `TOD`, `LDT`, and `DTL` carry a fraction.
const INSTANT_CASES: [Case; 7] = [
    (
        PlcPlatform::S7_300,
        Primitive::Date,
        r#""D#2026-10-12""#,
        "D#2026-10-12",
    ),
    (
        PlcPlatform::TwinCat3,
        Primitive::Ldate,
        r#""LD#2026-10-12""#,
        "LD#2026-10-12",
    ),
    (
        PlcPlatform::S7_1200,
        Primitive::TimeOfDay,
        r#""TOD#12:30:00.500""#,
        "TOD#12:30:00.5",
    ),
    (
        PlcPlatform::CodesysIec,
        Primitive::Ltod,
        r#""LTOD#23:59:59.999999999""#,
        "LTOD#23:59:59.999999999",
    ),
    (
        PlcPlatform::S7_300,
        Primitive::DateAndTime,
        r#""DT#2026-10-12-12:30:00""#,
        "DT#2026-10-12-12:30:00",
    ),
    (
        PlcPlatform::S7_1500,
        Primitive::Ldt,
        r#""LDT#2026-10-12-12:30:00.000000001""#,
        "LDT#2026-10-12-12:30:00.000000001",
    ),
    (
        PlcPlatform::S7_1200,
        Primitive::Dtl,
        r#""DTL#2026-10-12-12:30:00.250""#,
        "DTL#2026-10-12-12:30:00.25",
    ),
];

/// One case per duration primitive. The negative cases use S7-1500, which signs `TIME` and `LTIME`.
const DURATION_CASES: [Case; 4] = [
    (
        PlcPlatform::S7_1500,
        Primitive::Time,
        r#""T#-90s""#,
        "T#-1m30s",
    ),
    (
        PlcPlatform::TwinCat3,
        Primitive::Ltime,
        r#""LT#90m""#,
        "LT#1h30m",
    ),
    (
        PlcPlatform::S7_1500,
        Primitive::Ltime,
        r#""LT#-1d1ns""#,
        "LT#-1d1ns",
    ),
    (
        PlcPlatform::S7_300,
        Primitive::S5Time,
        r#""S5T#12300ms""#,
        "S5T#12s300ms",
    ),
];

/// The numeric primitives that the other tests do not cover, mostly at an end of the range.
const NUMERIC_CASES: [Case; 13] = [
    (PlcPlatform::S7_300, Primitive::Byte, "255", "255"),
    (PlcPlatform::S7_300, Primitive::Word, "65535", "65535"),
    (
        PlcPlatform::S7_300,
        Primitive::Dword,
        "4294967295",
        "4294967295",
    ),
    (
        PlcPlatform::S7_1500,
        Primitive::Lword,
        "18446744073709551615",
        "18446744073709551615",
    ),
    (PlcPlatform::S7_1200, Primitive::Sint, "-128", "-128"),
    (PlcPlatform::S7_1200, Primitive::Usint, "0", "0"),
    (PlcPlatform::S7_1200, Primitive::Uint, "65535", "65535"),
    (
        PlcPlatform::S7_300,
        Primitive::Dint,
        "-2147483648",
        "-2147483648",
    ),
    (
        PlcPlatform::S7_1200,
        Primitive::Udint,
        "4294967295",
        "4294967295",
    ),
    (
        PlcPlatform::S7_1500,
        Primitive::Lint,
        "-9223372036854775808",
        "-9223372036854775808",
    ),
    (
        PlcPlatform::S7_1500,
        Primitive::Ulint,
        "18446744073709551615",
        "18446744073709551615",
    ),
    (
        PlcPlatform::CodesysIec,
        Primitive::Lreal,
        "-1.250e-3",
        "-0.00125",
    ),
    (
        PlcPlatform::CodesysIec,
        Primitive::Lreal,
        "1e27",
        "1000000000000000000000000000",
    ),
];

/// Assert the round trip, the class, and the text of one primitive leaf for each case.
fn assert_cases(class: LeafClass, cases: &[Case]) {
    for &(platform, primitive, request, text) in cases {
        let tree = primitive_on(platform, primitive);
        assert_eq!(
            one_leaf_round_trip(&tree, request),
            (class, text.to_owned()),
            "{primitive:?} on {platform:?}"
        );
    }
}

#[test]
fn texts_of_char_leaves_keep_the_character_and_round_trip() {
    assert_cases(LeafClass::Char, &CHAR_CASES);
}

#[test]
fn texts_of_latin_1_and_wide_strings_keep_the_text_and_round_trip() {
    assert_eq!(
        one_leaf_round_trip(&string_tree(10), r#""Zé""#),
        (LeafClass::String, "Zé".to_owned())
    );
    assert_eq!(
        one_leaf_round_trip(&wstring_tree(4), r#""Łódź""#),
        (LeafClass::String, "Łódź".to_owned())
    );
}

#[test]
fn texts_of_instant_leaves_write_the_canonical_literal_and_round_trip() {
    assert_cases(LeafClass::Instant, &INSTANT_CASES);
}

#[test]
fn texts_of_duration_leaves_write_the_canonical_literal_and_round_trip() {
    assert_cases(LeafClass::Duration, &DURATION_CASES);
}

#[test]
fn texts_of_numeric_leaves_write_the_canonical_decimal_and_round_trip() {
    assert_cases(LeafClass::Numeric, &NUMERIC_CASES);
}

#[test]
fn texts_against_another_tree_give_wrong_tree() {
    let value = ParameterValue::parse(ZONE_JSON, &zone_tree()).unwrap();
    assert_eq!(
        value.texts(&primitive_tree(INT)),
        Err(ValueFault::WrongTree)
    );
}
