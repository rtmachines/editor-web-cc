use super::{LimitInput, LimitList, LimitListReason};
use crate::plant::LeafPath;

fn row(path: &str) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: None,
        max: Some("10".to_owned()),
        max_length: None,
    }
}

fn empty_row(path: &str) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: None,
        max: None,
        max_length: None,
    }
}

#[test]
fn two_rows_for_one_leaf_path_are_a_duplicate() {
    let rows = vec![row(".setpoint"), row(".setpoint")];
    assert_eq!(LimitList::parse(rows), Err(LimitListReason::DuplicatePath));
}

#[test]
fn all_elements_and_one_element_of_one_level_are_rejected() {
    let rows = vec![row(".ramps[*]"), row(".ramps[2]")];
    assert_eq!(LimitList::parse(rows), Err(LimitListReason::StarAndIndex));
}

#[test]
fn index_row_before_all_elements_row_is_rejected() {
    let rows = vec![row(".ramps[2]"), row(".ramps[*]")];
    assert_eq!(LimitList::parse(rows), Err(LimitListReason::StarAndIndex));
}

#[test]
fn all_elements_and_index_below_a_member_are_rejected() {
    let rows = vec![row(".zones[*].a"), row(".zones[2].b")];
    assert_eq!(LimitList::parse(rows), Err(LimitListReason::StarAndIndex));
}

#[test]
fn different_prefixes_with_all_and_index_are_allowed() {
    let rows = vec![row(".a[*]"), row(".b[2]")];
    assert!(LimitList::parse(rows).is_ok());
}

#[test]
fn two_all_elements_rows_at_one_level_are_allowed() {
    let rows = vec![row(".zones[*].a"), row(".zones[*].b")];
    assert!(LimitList::parse(rows).is_ok());
}

#[test]
fn row_without_a_field_is_empty() {
    let rows = vec![empty_row(".setpoint")];
    assert_eq!(LimitList::parse(rows), Err(LimitListReason::EmptyRow));
}

#[test]
fn empty_row_is_checked_before_duplicate_path() {
    let rows = vec![row(".setpoint"), row(".setpoint"), empty_row(".other")];
    assert_eq!(LimitList::parse(rows), Err(LimitListReason::EmptyRow));
}

#[test]
fn empty_list_is_valid() {
    let list = LimitList::parse(Vec::new()).unwrap();
    assert!(list.rows().is_empty());
}

#[test]
fn all_elements_at_root_with_index_at_root_is_rejected() {
    let rows = vec![row("[*]"), row("[1,2]")];
    assert_eq!(LimitList::parse(rows), Err(LimitListReason::StarAndIndex));
}

#[test]
fn valid_list_keeps_its_rows_in_order() {
    let rows = vec![row(".a"), row(".b")];
    let list: LimitList = LimitList::parse(rows.clone()).unwrap();
    assert_eq!(list.rows(), rows.as_slice());
    assert_eq!(list.into_rows(), rows);
}

// ADR-0025: a reason has no code. The three reasons give three distinct lower-case fragments
// without a final period.
#[test]
fn adr_0025_the_reason_texts_are_distinct_lower_case_fragments_without_a_final_period() {
    let reasons = [
        LimitListReason::DuplicatePath,
        LimitListReason::StarAndIndex,
        LimitListReason::EmptyRow,
    ];
    let texts: Vec<String> = reasons.iter().map(ToString::to_string).collect();
    for text in &texts {
        assert!(
            text.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
            "{text}"
        );
        assert!(!text.ends_with('.'), "{text}");
    }
    let distinct: std::collections::BTreeSet<&String> = texts.iter().collect();
    assert_eq!(distinct.len(), reasons.len());
}
