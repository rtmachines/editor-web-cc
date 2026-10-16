//! Tests of the reader of the fields of a parameter form (task 10.1).
//!
//! The rules come from ADR-0039 (a), (b), (e), (f), (h), and (k), and from design D6 and D11.

use std::cell::Cell;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{LeafPath, Primitive};
use recipe_domain::recipe::{MemberValue, ParameterKey};

use super::fields::{FieldSet, SentFields, form_pairs, max_pairs, read, read_body, read_pairs};
use super::tree_kit::{line_tree, primitive_tree};
use crate::error::{FieldReason, WebError};

/// The key of a parameter in the rebind form.
const KEY: &str = "7357a000-0000-0000-0000-000000000001";

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn read_set(list: &[(&str, &str)], set: FieldSet) -> Result<SentFields, WebError> {
    read(&pairs(list), set)
}

fn read_form(list: &[(&str, &str)]) -> Result<SentFields, WebError> {
    read_set(list, FieldSet::Parameter)
}

fn path(text: &str) -> LeafPath {
    LeafPath::parse(text).unwrap()
}

/// Assert that the answer is one line `E-WEB-008` of `field` with `reason`.
fn assert_bad(result: Result<SentFields, WebError>, field: &str, reason: FieldReason) {
    let error = result.err().unwrap();
    assert_eq!(error.code(), ErrorCode::WEB_008, "{error}");
    assert!(
        matches!(error, WebError::BadRequest { field: name, reason: why } if name == field && why == reason),
        "{error:?}"
    );
}

#[test]
fn fields_value_fields_give_the_leaf_paths_in_the_order_of_the_form() {
    let sent = read_form(&[
        ("csrf_token", "t"),
        ("row_version", "7"),
        ("value[2]", " 20 "),
        ("value[1]", "10"),
        ("value.zone.hold", "T#1s"),
    ])
    .unwrap();
    let got: Vec<(String, &str)> = sent
        .values()
        .iter()
        .map(|(leaf, text)| (leaf.to_string(), text.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("[2]".to_owned(), " 20 "),
            ("[1]".to_owned(), "10"),
            (".zone.hold".to_owned(), "T#1s")
        ]
    );
    assert_eq!(sent.text("row_version"), Some("7"));
}

#[test]
fn fields_the_session_token_is_not_kept_and_not_in_the_debug_text() {
    let sent = read_form(&[("csrf_token", "tok-zz-qq"), ("tag", "A")]).unwrap();
    assert!(!format!("{sent:?}").contains("tok-zz-qq"));
    assert_eq!(sent.text("csrf_token"), None);
    assert_bad(
        read_form(&[("csrf_token", "a"), ("csrf_token", "b")]),
        "form",
        FieldReason::Form,
    );
}

#[test]
fn fields_the_field_value_alone_is_the_value_of_a_primitive() {
    let sent = read_form(&[("value", "1200")]).unwrap();
    assert_eq!(sent.values(), [(path(""), "1200".to_owned())]);
}

#[test]
fn fields_a_key_outside_the_fields_of_the_route_is_a_form_error() {
    for key in ["foo", "values[1]", "Value", "limits", "new_member_values"] {
        assert_bad(read_form(&[(key, "1")]), "form", FieldReason::Form);
    }
}

#[test]
fn fields_a_malformed_value_key_is_a_value_key_error() {
    for key in [
        "value.",
        "value.1a",
        "value[x]",
        "value[1,2,3,4]",
        "value[]",
        "valuex",
    ] {
        let result = read_form(&[(key, "1")]);
        let error = result.err().unwrap();
        assert!(
            matches!(
                error,
                WebError::BadRequest {
                    reason: FieldReason::ValueKey | FieldReason::Form,
                    ..
                }
            ),
            "{key}: {error:?}"
        );
    }
    assert_bad(
        read_form(&[("value[x]", "1")]),
        "form",
        FieldReason::ValueKey,
    );
}

#[test]
fn fields_a_value_key_with_every_element_is_a_value_key_error() {
    assert_bad(
        read_form(&[("value[*]", "1")]),
        "form",
        FieldReason::ValueKey,
    );
    assert_bad(
        read_form(&[("value.zones[*].hold", "T#1s")]),
        "form",
        FieldReason::ValueKey,
    );
}

#[test]
fn fields_a_value_key_twice_is_a_value_key_error() {
    assert_bad(
        read_form(&[("value[1]", "1"), ("value[1]", "2")]),
        "form",
        FieldReason::ValueKey,
    );
    assert_bad(
        read_form(&[("value[1]", "1"), ("value[01]", "2")]),
        "form",
        FieldReason::ValueKey,
    );
}

#[test]
fn fields_a_plain_field_twice_is_a_form_error() {
    assert_bad(
        read_form(&[("tag", "A"), ("tag", "B")]),
        "form",
        FieldReason::Form,
    );
}

#[test]
fn fields_a_leaf_path_with_more_segments_than_the_depth_limit_is_a_value_key_error() {
    let deepest = format!("value{}", ".a".repeat(32));
    assert!(read_form(&[(deepest.as_str(), "1")]).is_ok());
    let deeper = format!("value{}", ".a".repeat(33));
    assert_bad(
        read_form(&[(deeper.as_str(), "1")]),
        "form",
        FieldReason::ValueKey,
    );
}

#[test]
fn fields_a_key_of_one_mib_gives_one_line_without_the_key() {
    let segments = format!("value{}", ".a".repeat(512 * 1024));
    let name = format!("value.{}", "a".repeat(1024 * 1024));
    for key in [segments, name] {
        let error = read_form(&[(key.as_str(), "1")]).err().unwrap();
        assert_eq!(error.code(), ErrorCode::WEB_008);
        assert!(error.to_string().len() < 300, "{}", error.to_string().len());
    }
}

#[test]
fn fields_a_flood_of_pairs_stops_at_the_first_pair_above_the_limit() {
    // Five times `budget.leaves_per_parameter` (16 384) and 64 (ADR-0039 (b)).
    assert_eq!(max_pairs(), 81_984);
    let mut flood = pairs(&[("value", "1")]);
    flood.extend((0..100_000).map(|index| (format!("value[{index}]"), "1".to_owned())));
    let consumed = Cell::new(0_usize);
    let result = read(
        flood
            .iter()
            .inspect(|_pair| consumed.set(consumed.get().checked_add(1).unwrap())),
        FieldSet::Parameter,
    );
    assert_bad(result, "form", FieldReason::Form);
    assert_eq!(consumed.get(), max_pairs().checked_add(1).unwrap());
}

#[test]
fn fields_the_body_reader_takes_no_pair_after_the_first_pair_above_the_limit() {
    // ADR-0039 (b), spec "Flood of pairs": the reader counts the pairs while it reads them.
    let flood: String = (0..100_000)
        .map(|index| format!("&value%5B{index}%5D=1"))
        .collect();
    let body = format!("value=1{flood}");
    let consumed = Cell::new(0_usize);
    let pairs = form_pairs(body.as_bytes())
        .inspect(|_pair| consumed.set(consumed.get().checked_add(1).unwrap()));
    assert_bad(
        read_pairs(pairs, FieldSet::Parameter),
        "form",
        FieldReason::Form,
    );
    assert_eq!(consumed.get(), max_pairs().checked_add(1).unwrap());
}

#[test]
fn fields_the_body_reader_decodes_the_pairs_as_a_browser_encodes_them() {
    let body = b"csrf_token=t&&row_version=7&value%5B1%5D=+1%2B2&value.a=%E2%82%AC&value.b&value.c=%zz&value.d=%FF&";
    let sent = read_body(body, FieldSet::Value).unwrap();
    let got: Vec<(String, &str)> = sent
        .values()
        .iter()
        .map(|(leaf, text)| (leaf.to_string(), text.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("[1]".to_owned(), " 1+2"),
            (".a".to_owned(), "\u{20ac}"),
            (".b".to_owned(), ""),
            (".c".to_owned(), "%zz"),
            (".d".to_owned(), "\u{fffd}"),
        ]
    );
    assert_eq!(sent.text("row_version"), Some("7"));
}

#[test]
fn fields_the_body_reader_refuses_an_unknown_key_like_the_pair_reader() {
    assert_bad(
        read_body(b"row_version=1&unknown=1", FieldSet::Value),
        "form",
        FieldReason::Form,
    );
}

#[test]
fn fields_the_limit_of_pairs_itself_passes() {
    let list: Vec<(String, String)> = (0..max_pairs())
        .map(|index| (format!("value[{index}]"), "1".to_owned()))
        .collect();
    assert_eq!(
        read(&list, FieldSet::Parameter).unwrap().values().len(),
        max_pairs()
    );
}

#[test]
fn fields_limit_rows_come_in_the_order_of_their_index() {
    let sent = read_form(&[
        ("limits[1].leaf_path", "[*]"),
        ("limits[1].min", "0"),
        ("limits[1].max", "3000"),
        ("limits[0].leaf_path", ".name"),
        ("limits[0].max_length", "8"),
    ])
    .unwrap();
    let rows: Vec<(usize, String, &str, &str, &str)> = sent
        .limits()
        .iter()
        .map(|row| {
            (
                row.index,
                row.leaf_path.to_string(),
                row.min.as_str(),
                row.max.as_str(),
                row.max_length.as_str(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (0, ".name".to_owned(), "", "", "8"),
            (1, "[*]".to_owned(), "0", "3000", "")
        ]
    );
}

#[test]
fn fields_a_malformed_limit_key_is_a_limit_row_error() {
    for key in [
        "limits[01].min",
        "limits[x].min",
        "limits[0].foo",
        "limits[0]min",
        "limits[].min",
        "limits[1234567].min",
    ] {
        assert_bad(read_form(&[(key, "1")]), "form", FieldReason::LimitRow);
    }
}

#[test]
fn fields_a_limit_field_twice_is_a_limit_row_error() {
    assert_bad(
        read_form(&[("limits[0].min", "1"), ("limits[0].min", "2")]),
        "form",
        FieldReason::LimitRow,
    );
}

#[test]
fn fields_a_limit_row_with_a_bound_and_no_path_is_a_limit_row_error() {
    assert_bad(
        read_form(&[("limits[0].min", "1")]),
        "form",
        FieldReason::LimitRow,
    );
    assert_bad(
        read_form(&[("limits[0].leaf_path", "x.y"), ("limits[0].min", "1")]),
        "form",
        FieldReason::LimitRow,
    );
}

#[test]
fn fields_a_blank_limit_row_is_not_a_limit() {
    let sent = read_form(&[
        ("limits[0].leaf_path", "[1]"),
        ("limits[0].min", " "),
        ("limits[0].max", ""),
        ("limits[1].leaf_path", "[2]"),
        ("limits[1].min", " 5 "),
        ("limits[2].min", ""),
    ])
    .unwrap();
    let list = sent.limit_list().unwrap();
    let rows: Vec<(String, Option<&str>, Option<&str>)> = list
        .rows()
        .iter()
        .map(|row| {
            (
                row.leaf_path.to_string(),
                row.min.as_deref(),
                row.max.as_deref(),
            )
        })
        .collect();
    assert_eq!(rows, [("[2]".to_owned(), Some("5"), None)]);
}

#[test]
fn fields_every_bad_maximum_length_gets_a_line_under_its_input() {
    let sent = read_form(&[
        ("limits[0].leaf_path", ".a"),
        ("limits[0].max_length", "x"),
        ("limits[1].leaf_path", ".b"),
        ("limits[1].max_length", "16383"),
        ("limits[2].leaf_path", ".c"),
        ("limits[2].max_length", " 16382 "),
        ("limits[3].leaf_path", ".d"),
        ("limits[3].max_length", "0"),
    ])
    .unwrap();
    let lines = sent.limit_list().err().unwrap();
    let targets: Vec<Option<&str>> = lines.iter().map(|line| line.target()).collect();
    assert_eq!(
        targets,
        [
            Some("limits-0-max_length"),
            Some("limits-1-max_length"),
            Some("limits-3-max_length")
        ]
    );
    assert!(lines.iter().all(|line| line.code() == ErrorCode::WEB_008));
}

#[test]
fn fields_two_limit_rows_of_one_level_with_every_element_and_an_index_are_a_line() {
    let sent = read_form(&[
        ("limits[0].leaf_path", "[*]"),
        ("limits[0].min", "1"),
        ("limits[1].leaf_path", "[2]"),
        ("limits[1].min", "1"),
    ])
    .unwrap();
    let lines = sent.limit_list().err().unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines.first().unwrap().target(), None);
}

#[test]
fn fields_the_button_show_value_fields_sends_one() {
    assert!(read_form(&[("fields", "1")]).unwrap().wants_fields());
    assert!(!read_form(&[]).unwrap().wants_fields());
    assert_bad(read_form(&[("fields", "2")]), "form", FieldReason::Flag);
}

#[test]
fn fields_the_limit_mode_names_one_array_level() {
    for level in ["[*]", ".zones[*]", "[*].ramps[*]", "[1].ramps[*]"] {
        let sent = read_form(&[("limit_mode", level)]).unwrap();
        assert_eq!(sent.limit_mode(), Some(&path(level)), "{level}");
    }
    assert_eq!(read_form(&[]).unwrap().limit_mode(), None);
    for bad in ["", "[1]", ".zones", "x", "[*].a"] {
        assert_bad(
            read_form(&[("limit_mode", bad)]),
            "limit_mode",
            FieldReason::Reload,
        );
    }
}

#[test]
fn fields_the_value_type_is_a_type_expression() {
    let sent = read_form(&[("value_type", "ARRAY[1..8] OF INT")]).unwrap();
    assert_eq!(
        sent.value_type().map(ToString::to_string),
        Some("ARRAY[1..8] OF INT".to_owned())
    );
    assert_bad(
        read_form(&[("value_type", "ARRAY[")]),
        "value_type",
        FieldReason::Reload,
    );
}

#[test]
fn fields_each_route_takes_only_its_own_fields() {
    let value = [("csrf_token", "t"), ("row_version", "1"), ("value", "1")];
    assert!(read_set(&value, FieldSet::Value).is_ok());
    assert_bad(
        read_set(&[("tag", "A")], FieldSet::Value),
        "form",
        FieldReason::Form,
    );
    assert_bad(
        read_set(&[("value", "1")], FieldSet::Limits),
        "form",
        FieldReason::Form,
    );
    assert!(read_set(&[("limits[0].leaf_path", "")], FieldSet::Limits).is_ok());
    assert_bad(
        read_set(&[("limits[0].min", "1")], FieldSet::Value),
        "form",
        FieldReason::Form,
    );
    assert_bad(
        read_set(&[("value", "1")], FieldSet::Rebind),
        "form",
        FieldReason::Form,
    );
    assert_bad(
        read_set(&[("fields", "1")], FieldSet::Value),
        "form",
        FieldReason::Form,
    );
}

#[test]
fn fields_new_member_values_name_the_key_and_the_leaf() {
    let first = format!("new_member_values[{KEY}].temp_min");
    let second = format!("new_member_values[{KEY}][2].hold");
    let sent = read_set(
        &[
            ("from_type_id", "a"),
            ("to_type_id", "b"),
            (first.as_str(), " 5 "),
            (second.as_str(), "T#1s"),
        ],
        FieldSet::Rebind,
    )
    .unwrap();
    let key = ParameterKey::parse(KEY).unwrap();
    let values = sent.member_values(&|_key| None);
    assert_eq!(
        values.get(&(key, path(".temp_min"))),
        Some(&MemberValue::Text(" 5 ".to_owned()))
    );
    assert_eq!(values.len(), 2);
    assert_eq!(sent.text("from_type_id"), Some("a"));
}

#[test]
fn fields_a_malformed_new_member_key_is_a_value_key_error() {
    let star = format!("new_member_values[{KEY}][*].a");
    let twice = format!("new_member_values[{KEY}].a");
    for key in [
        "new_member_values[x].a".to_owned(),
        "new_member_values[].a".to_owned(),
        format!("new_member_values[{KEY}].1"),
        format!("new_member_values[{KEY}"),
        star,
    ] {
        assert_bad(
            read_set(&[(key.as_str(), "1")], FieldSet::Rebind),
            "form",
            FieldReason::ValueKey,
        );
    }
    assert_bad(
        read_set(
            &[(twice.as_str(), "1"), (twice.as_str(), "2")],
            FieldSet::Rebind,
        ),
        "form",
        FieldReason::ValueKey,
    );
    assert_bad(
        read_set(&[("new_member_values.a", "1")], FieldSet::Rebind),
        "form",
        FieldReason::Form,
    );
}

#[test]
fn fields_texts_of_numbers_and_times_are_trimmed_and_strings_are_kept() {
    let tree = line_tree();
    let sent = read_form(&[
        ("value.name", "  Linia A  "),
        ("value.speed", " 1200 "),
        ("value.zone.setpoint", "\t1.5\n"),
        ("value.zone.hold", " T#1s "),
        ("value.zone.enabled", " true "),
        ("value.unknown", " 7 "),
    ])
    .unwrap();
    let texts = sent.leaf_texts(&tree).unwrap();
    let got: Vec<(String, &str)> = texts
        .iter()
        .map(|(leaf, text)| (leaf.to_string(), text))
        .collect();
    assert_eq!(
        got,
        [
            (".name".to_owned(), "  Linia A  "),
            (".speed".to_owned(), "1200"),
            (".zone.setpoint".to_owned(), "1.5"),
            (".zone.hold".to_owned(), "T#1s"),
            (".zone.enabled".to_owned(), "true"),
            (".unknown".to_owned(), " 7 ")
        ]
    );
}

#[test]
fn fields_a_char_text_is_kept_as_sent() {
    let tree = primitive_tree(Primitive::Char);
    let sent = read_form(&[("value", " ")]).unwrap();
    let texts = sent.leaf_texts(&tree).unwrap();
    assert_eq!(texts.iter().next().map(|(_leaf, text)| text), Some(" "));
}

#[test]
fn fields_new_member_values_are_trimmed_by_the_class_of_their_leaf() {
    let tree = line_tree();
    let name = format!("new_member_values[{KEY}].name");
    let speed = format!("new_member_values[{KEY}].speed");
    let sent = read_set(
        &[(name.as_str(), " A "), (speed.as_str(), " 7 ")],
        FieldSet::Rebind,
    )
    .unwrap();
    let key = ParameterKey::parse(KEY).unwrap();
    let values = sent.member_values(&|_key| Some(&tree));
    assert_eq!(
        values.get(&(key, path(".name"))),
        Some(&MemberValue::Text(" A ".to_owned()))
    );
    assert_eq!(
        values.get(&(key, path(".speed"))),
        Some(&MemberValue::Text("7".to_owned()))
    );
}
