use super::*;
use proptest::prelude::*;

fn member(name: &str) -> LeafSegment {
    LeafSegment::Member(MemberName::parse(name).expect("valid member name"))
}

fn assert_round_trip(text: &str) {
    let parsed = LeafPath::parse(text);
    assert!(parsed.is_ok(), "{text} must parse");
    assert_eq!(parsed.map(|path| path.to_string()), Ok(text.to_owned()));
}

fn reason_of(text: &str) -> Option<LeafPathReason> {
    LeafPath::parse(text).err()
}

#[test]
fn leaf_path_examples_round_trip() {
    for text in [
        "",
        ".setpoint",
        "[3]",
        "[1,2]",
        "[-1,0,7]",
        ".zones[2].temp_max",
        ".zones[*].temp_max",
        "[*]",
    ] {
        assert_round_trip(text);
    }
}

#[test]
fn leaf_path_segments_follow_the_text() {
    let parsed = LeafPath::parse(".zones[2].temp_max").map(|p| p.segments().to_vec());
    assert_eq!(
        parsed,
        Ok(vec![
            member("zones"),
            LeafSegment::Index(vec![2]),
            member("temp_max"),
        ])
    );
}

#[test]
fn leaf_path_empty_text_is_empty_path() {
    assert_eq!(LeafPath::parse("").map(|p| p.is_empty()), Ok(true));
    assert_eq!(LeafPath::parse("[3]").map(|p| p.is_empty()), Ok(false));
}

#[test]
fn leaf_path_four_indexes_is_too_many_indexes() {
    assert_eq!(reason_of("[1,2,3,4]"), Some(LeafPathReason::TooManyIndexes));
}

#[test]
fn leaf_path_malformed_texts_are_grammar_errors() {
    for text in [
        "[*,1]",
        "[1,*]",
        "[]",
        "[1,]",
        ".",
        "..a",
        ".1a",
        "zones",
        "[ 1]",
        "[+1]",
        "[12345678901]",
    ] {
        assert_eq!(reason_of(text), Some(LeafPathReason::Grammar), "{text}");
    }
}

#[test]
fn leaf_path_index_above_integer_max_is_out_of_range() {
    assert_eq!(
        reason_of("[2147483648]"),
        Some(LeafPathReason::IndexOutOfRange)
    );
}

#[test]
fn leaf_path_index_below_integer_min_is_out_of_range() {
    assert_eq!(
        reason_of("[-2147483649]"),
        Some(LeafPathReason::IndexOutOfRange)
    );
}

#[test]
fn leaf_path_integer_limits_pass() {
    assert_round_trip("[2147483647]");
    assert_round_trip("[-2147483648]");
}

#[test]
fn leaf_path_leading_zero_displays_canonical_number() {
    assert_eq!(
        LeafPath::parse("[007]").map(|p| p.to_string()),
        Ok("[7]".to_owned())
    );
}

#[test]
fn leaf_path_negative_zero_displays_as_zero() {
    assert_eq!(
        LeafPath::parse("[-0]").map(|p| p.to_string()),
        Ok("[0]".to_owned())
    );
}

fn member_text() -> impl Strategy<Value = String> {
    "[A-Za-z_][A-Za-z0-9_]{0,10}".prop_map(|name| format!(".{name}"))
}

fn index_text() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<i32>(), 1..=3).prop_map(|values| {
        let joined: Vec<String> = values.iter().map(i32::to_string).collect();
        format!("[{}]", joined.join(","))
    })
}

fn segment_text() -> impl Strategy<Value = String> {
    prop_oneof![member_text(), index_text(), Just("[*]".to_owned()),]
}

proptest! {
    #[test]
    fn leaf_path_generated_text_round_trips(
        segments in prop::collection::vec(segment_text(), 0..=5),
    ) {
        let text = segments.concat();
        match LeafPath::parse(&text) {
            Ok(parsed) => prop_assert_eq!(parsed.to_string(), text),
            Err(reason) => prop_assert!(false, "{text} failed: {reason}"),
        }
    }
}
