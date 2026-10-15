//! Tests of the reader of the UDT form body (task 8.2, design D8).

use super::form_rows::{SentForm, read};
use crate::error::{FieldReason, WebError};

/// Build the pairs of a body from borrowed texts.
fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

/// Read the body made of `items`.
fn read_of(items: &[(&str, &str)]) -> Result<SentForm, WebError> {
    read(&pairs(items))
}

/// Return true when the result is the field error `field` with `reason`.
fn is_bad(result: &Result<SentForm, WebError>, field: &str, reason: FieldReason) -> bool {
    matches!(
        result,
        Err(WebError::BadRequest { field: found, reason: why }) if *found == field && *why == reason
    )
}

#[test]
fn form_rows_read_the_name_the_rows_and_ignore_the_token() {
    let form = read_of(&[
        ("csrf_token", "abc"),
        ("name", "ST_Zone"),
        ("members[0].name", "temp_max"),
        ("members[0].type", "REAL"),
    ])
    .unwrap();
    assert_eq!(form.name.as_deref(), Some("ST_Zone"));
    assert!(!form.add);
    assert_eq!(form.rows.len(), 1);
    assert_eq!(form.rows[0].name, "temp_max");
    assert_eq!(form.rows[0].type_text, "REAL");
}

#[test]
fn form_rows_of_an_empty_body_are_empty() {
    let form = read_of(&[]).unwrap();
    assert_eq!(form.name, None);
    assert!(!form.add);
    assert!(form.rows.is_empty());
}

#[test]
fn form_rows_keep_the_order_of_the_index_and_do_not_depend_on_the_order_of_the_keys() {
    let form = read_of(&[
        ("members[3].type", "T3"),
        ("members[1].name", "a"),
        ("members[3].name", "c"),
        ("members[1].type", "T1"),
        ("members[10].name", "d"),
        ("members[10].type", "T10"),
        ("members[2].name", "b"),
        ("members[2].type", "T2"),
    ])
    .unwrap();
    let names: Vec<&str> = form.rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["a", "b", "c", "d"]);
    let types: Vec<&str> = form.rows.iter().map(|row| row.type_text.as_str()).collect();
    assert_eq!(types, ["T1", "T2", "T3", "T10"]);
}

#[test]
fn form_rows_give_an_empty_text_to_a_field_that_the_body_lacks() {
    let form = read_of(&[("members[0].name", "speed"), ("members[1].type", "INT")]).unwrap();
    assert_eq!(form.rows.len(), 2);
    assert_eq!(form.rows[0].type_text, "");
    assert_eq!(form.rows[1].name, "");
}

#[test]
fn form_rows_keep_the_text_of_a_value_as_it_is() {
    let form = read_of(&[
        ("name", "  Zażółć <b> "),
        ("members[0].name", ""),
        ("members[0].type", " ARRAY "),
    ])
    .unwrap();
    assert_eq!(form.name.as_deref(), Some("  Zażółć <b> "));
    assert_eq!(form.rows[0].name, "");
    assert_eq!(form.rows[0].type_text, " ARRAY ");
}

#[test]
fn form_rows_read_the_flag_add() {
    assert!(read_of(&[("add", "1")]).unwrap().add);
    assert!(!read_of(&[("name", "ST_A")]).unwrap().add);
    for value in ["", "0", "2", "true", "01", " 1"] {
        let result = read_of(&[("add", value)]);
        assert!(is_bad(&result, "add", FieldReason::Flag), "{value:?}");
    }
}

#[test]
fn form_rows_refuse_a_key_that_the_form_does_not_have() {
    let keys = [
        "foo",
        "Name",
        "csrf_pre",
        "next",
        "members",
        "members[0]",
        "members[0].",
        "members[0].other",
        "members[0].name.x",
        "members[0]name",
        "members[0].Name",
        "Members[0].name",
        "members[ 0].name",
        "members[+1].name",
        "members[-1].name",
        "members[a].name",
        "members[].name",
        "members[1.5].name",
        "members[0",
        "members0].name",
        "xmembers[0].name",
    ];
    for key in keys {
        let result = read_of(&[(key, "x")]);
        assert!(is_bad(&result, "form", FieldReason::Form), "{key}");
    }
}

#[test]
fn form_rows_refuse_an_index_with_a_leading_zero_or_too_many_digits() {
    for index in ["00", "01", "007", "0001", "10000", "123456", "1e1"] {
        let key = format!("members[{index}].name");
        let result = read_of(&[(key.as_str(), "x")]);
        assert!(is_bad(&result, "form", FieldReason::Form), "{index}");
    }
}

#[test]
fn form_rows_accept_the_index_zero_and_up_to_1023() {
    for index in ["0", "9", "10", "99", "100", "999", "1000", "1023"] {
        let key = format!("members[{index}].name");
        let form = read_of(&[(key.as_str(), "x")]).unwrap();
        assert_eq!(form.rows.len(), 1, "{index}");
    }
}

#[test]
fn form_rows_refuse_an_index_above_the_maximum_of_the_member_count() {
    for index in ["1024", "1025", "5000", "9999"] {
        let key = format!("members[{index}].type");
        let result = read_of(&[(key.as_str(), "x")]);
        assert!(
            is_bad(&result, "members", FieldReason::MemberCount),
            "{index}"
        );
    }
}

#[test]
fn form_rows_hold_exactly_1024_rows_at_the_limit() {
    let mut items: Vec<(String, String)> = Vec::new();
    for index in 0..1024 {
        items.push((format!("members[{index}].name"), format!("m{index}")));
        items.push((format!("members[{index}].type"), "INT".to_owned()));
    }
    let form = read(&items).unwrap();
    assert_eq!(form.rows.len(), 1024);
    assert_eq!(form.rows[1023].name, "m1023");
    assert_eq!(form.rows[0].name, "m0");
}

#[test]
fn form_rows_refuse_two_keys_for_one_field() {
    let repeated = [
        [("members[1].name", "a"), ("members[1].name", "b")],
        [("members[1].type", "a"), ("members[1].type", "b")],
        [("name", "ST_A"), ("name", "ST_B")],
        [("add", "1"), ("add", "1")],
        [("csrf_token", "a"), ("csrf_token", "b")],
    ];
    for items in repeated {
        let result = read_of(&items);
        assert!(is_bad(&result, "form", FieldReason::Form), "{items:?}");
    }
}

#[test]
fn form_rows_accept_the_name_and_the_type_of_one_row_once_each() {
    let form = read_of(&[("members[1].name", "a"), ("members[1].type", "INT")]).unwrap();
    assert_eq!(form.rows.len(), 1);
}
