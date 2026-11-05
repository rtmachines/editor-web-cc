use super::*;
use proptest::prelude::*;

fn parse_ok(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

/// Return the rejected text when `text` gives `InvalidUnitPath`.
fn invalid_text(text: &str) -> Option<String> {
    match UnitPath::parse(text) {
        Err(PlantError::InvalidUnitPath { text }) => Some(text),
        _ => None,
    }
}

#[test]
fn valid_paths_pass_and_display_equals_the_input() {
    for text in ["U1", "U64", "U1.2.4.5.6", "U64.64.64.64.64", "U1.2.4"] {
        assert_eq!(parse_ok(text).to_string(), text);
    }
}

#[test]
fn segment_above_64_gives_invalid_text() {
    assert_eq!(invalid_text("U65"), Some("U65".to_owned()));
    assert_eq!(invalid_text("U1.65"), Some("U1.65".to_owned()));
}

#[test]
fn malformed_texts_give_invalid_text() {
    for text in [
        "U0", "U01", "U1..2", "U1.", "1.2", "u1", "U", "U1.2a", "U 1", "",
    ] {
        assert_eq!(invalid_text(text), Some(text.to_owned()));
    }
}

#[test]
fn six_valid_segments_give_depth_error() {
    assert!(matches!(
        UnitPath::parse("U1.2.3.4.5.6"),
        Err(PlantError::UnitDepthExceeded { path }) if path == "U1.2.3.4.5.6"
    ));
}

#[test]
fn six_segments_with_a_bad_last_segment_give_invalid_text() {
    assert_eq!(
        invalid_text("U1.2.3.4.5.65"),
        Some("U1.2.3.4.5.65".to_owned())
    );
}

#[test]
fn parent_of_three_segments_is_two_segments() {
    assert_eq!(parse_ok("U1.2.4").parent(), Some(parse_ok("U1.2")));
}

#[test]
fn parent_of_one_segment_is_none() {
    assert_eq!(parse_ok("U1").parent(), None);
}

#[test]
fn depth_counts_the_segments() {
    assert_eq!(parse_ok("U1").depth(), 1);
    assert_eq!(parse_ok("U1.2.3.4.5").depth(), 5);
}

#[test]
fn segments_iterate_in_order() {
    let values: Vec<u8> = parse_ok("U1.10.64").segments().collect();
    assert_eq!(values, vec![1, 10, 64]);
}

#[test]
fn ancestor_is_a_strict_prefix() {
    let one = parse_ok("U1");
    assert!(one.is_ancestor_of(&parse_ok("U1.2.3")));
    assert!(!one.is_ancestor_of(&parse_ok("U12")));
    assert!(!one.is_ancestor_of(&one));
    assert!(!parse_ok("U1.2").is_ancestor_of(&parse_ok("U1.3.4")));
    assert!(!parse_ok("U1.2.3").is_ancestor_of(&parse_ok("U1.2")));
}

#[test]
fn self_or_descendant_covers_below_itself_and_sibling_cases() {
    let u12 = parse_ok("U1.2");
    assert!(parse_ok("U1.2.4").is_self_or_descendant_of(&u12));
    assert!(u12.is_self_or_descendant_of(&u12));
    assert!(!parse_ok("U12").is_self_or_descendant_of(&parse_ok("U1")));
    assert!(!parse_ok("U1").is_self_or_descendant_of(&u12));
}

#[test]
fn order_is_numeric_per_segment() {
    let mut paths = [
        parse_ok("U10"),
        parse_ok("U2"),
        parse_ok("U1.10"),
        parse_ok("U1.2"),
        parse_ok("U1"),
    ];
    paths.sort();
    let texts: Vec<String> = paths.iter().map(ToString::to_string).collect();
    assert_eq!(texts, vec!["U1", "U1.2", "U1.10", "U2", "U10"]);
}

#[test]
fn first_main_is_u1() {
    assert_eq!(UnitPath::first_main().to_string(), "U1");
    assert_eq!(UnitPath::first_main(), parse_ok("U1"));
}

fn valid_segments(range: std::ops::RangeInclusive<usize>) -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(1u8..=64, range)
}

fn join_text(segments: &[u8]) -> String {
    let joined: Vec<String> = segments.iter().map(ToString::to_string).collect();
    format!("U{}", joined.join("."))
}

/// Segment-wise oracle: every segment of `ancestor` equals the segment at the same place.
fn oracle_self_or_descendant(candidate: &[u8], ancestor: &[u8]) -> bool {
    candidate.len() >= ancestor.len()
        && ancestor
            .iter()
            .zip(candidate.iter())
            .all(|(expected, actual)| expected == actual)
}

proptest! {
    #[test]
    fn self_or_descendant_matches_the_segment_wise_oracle(
        candidate in valid_segments(1..=5),
        ancestor in valid_segments(1..=5),
    ) {
        let candidate_path = UnitPath::parse(&join_text(&candidate))
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        let ancestor_path = UnitPath::parse(&join_text(&ancestor))
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(
            candidate_path.is_self_or_descendant_of(&ancestor_path),
            oracle_self_or_descendant(&candidate, &ancestor)
        );
    }

    #[test]
    fn every_path_is_self_or_descendant_of_its_prefixes(segments in valid_segments(1..=5)) {
        let path = UnitPath::parse(&join_text(&segments))
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        for length in 1..=segments.len() {
            let prefix_segments = segments.iter().take(length).copied().collect::<Vec<u8>>();
            let prefix = UnitPath::parse(&join_text(&prefix_segments))
                .map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert!(path.is_self_or_descendant_of(&prefix));
        }
    }

    #[test]
    fn valid_text_round_trips(segments in valid_segments(1..=5)) {
        let text = join_text(&segments);
        let path = UnitPath::parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(path.to_string(), text);
        prop_assert_eq!(usize::from(path.depth()), segments.len());
    }

    #[test]
    fn six_to_eight_valid_segments_give_depth_error(segments in valid_segments(6..=8)) {
        let text = join_text(&segments);
        let depth_error = matches!(
            UnitPath::parse(&text),
            Err(PlantError::UnitDepthExceeded { .. })
        );
        prop_assert!(depth_error);
    }
}
