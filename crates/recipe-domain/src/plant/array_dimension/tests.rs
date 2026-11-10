use super::*;

fn dimension(lower: i32, upper: i32) -> ArrayDimension {
    ArrayDimension::parse(lower, upper).expect("test dimension is valid")
}

#[test]
fn reversed_bounds_fail_with_order() {
    assert_eq!(
        ArrayDimension::parse(5, 1),
        Err(ArrayDimensionReason::Order)
    );
}

#[test]
fn span_of_65536_fails_with_span() {
    assert_eq!(
        ArrayDimension::parse(0, 65_536),
        Err(ArrayDimensionReason::Span)
    );
}

#[test]
fn span_of_65535_passes_with_65536_elements() {
    let parsed = ArrayDimension::parse(0, 65_535);
    assert_eq!(parsed.map(ArrayDimension::count), Ok(65_536));
}

#[test]
fn negative_span_of_65535_passes() {
    let parsed = ArrayDimension::parse(-2_147_483_648, -2_147_418_113);
    assert_eq!(parsed.map(ArrayDimension::count), Ok(65_536));
}

#[test]
fn full_i32_range_fails_with_span_without_overflow() {
    assert_eq!(
        ArrayDimension::parse(-2_147_483_648, 2_147_483_647),
        Err(ArrayDimensionReason::Span)
    );
}

#[test]
fn single_element_dimension_has_one_element() {
    assert_eq!(
        ArrayDimension::parse(7, 7).map(ArrayDimension::count),
        Ok(1)
    );
}

#[test]
fn bounds_are_returned_unchanged() {
    let parsed = ArrayDimension::parse(1, 8);
    assert_eq!(parsed.map(ArrayDimension::lower), Ok(1));
    assert_eq!(parsed.map(ArrayDimension::upper), Ok(8));
}

#[test]
fn dimension_displays_its_bounds() {
    assert_eq!(
        ArrayDimension::parse(1, 8).map(|d| d.to_string()),
        Ok("1..8".to_owned())
    );
}

#[test]
fn one_to_three_dimensions_pass() {
    for count in 1..=3 {
        let list = vec![dimension(1, 2); count];
        assert!(ArrayDimensions::parse(list).is_ok(), "{count}");
    }
}

#[test]
fn zero_dimensions_fail_with_count() {
    assert_eq!(
        ArrayDimensions::parse(Vec::new()),
        Err(ArrayDimensionReason::Count)
    );
}

#[test]
fn four_dimensions_fail_with_count() {
    let list = vec![dimension(1, 2); 4];
    assert_eq!(
        ArrayDimensions::parse(list),
        Err(ArrayDimensionReason::Count)
    );
}

#[test]
fn two_dimensions_have_six_elements() {
    let list = vec![dimension(1, 2), dimension(1, 3)];
    let dimensions = ArrayDimensions::parse(list);
    assert_eq!(
        dimensions.as_ref().map(ArrayDimensions::element_count),
        Ok(Some(6))
    );
    assert_eq!(dimensions.as_ref().map(ArrayDimensions::len), Ok(2));
}

#[test]
fn three_maximal_dimensions_have_two_to_the_48_elements() {
    let list = vec![dimension(0, 65_535); 3];
    let dimensions = ArrayDimensions::parse(list);
    assert_eq!(
        dimensions.as_ref().map(ArrayDimensions::element_count),
        Ok(Some(281_474_976_710_656))
    );
}

#[test]
fn dimensions_display_with_commas() {
    let list = vec![dimension(1, 2), dimension(1, 3)];
    let dimensions = ArrayDimensions::parse(list);
    assert_eq!(
        dimensions.map(|d| d.to_string()),
        Ok("1..2,1..3".to_owned())
    );
}

#[test]
fn iter_yields_the_dimensions_in_order() {
    let list = vec![dimension(1, 2), dimension(5, 9)];
    let dimensions = ArrayDimensions::parse(list.clone());
    let yielded: Vec<ArrayDimension> = dimensions
        .as_ref()
        .map_or(Vec::new(), |d| d.iter().copied().collect());
    assert_eq!(yielded, list);
}

// domain-model.md section 2.1: `upper - lower < 65536` accepts 65536 elements at most.
#[test]
fn span_text_names_the_largest_accepted_element_count() {
    let largest = ArrayDimension::parse(0, 65_535).map(ArrayDimension::count);
    assert_eq!(largest, Ok(65_536));
    assert_eq!(
        ArrayDimension::parse(0, 65_536),
        Err(ArrayDimensionReason::Span)
    );
    let expected = format!("more than {}", largest.unwrap_or_default());
    assert!(
        ArrayDimensionReason::Span.to_string().contains(&expected),
        "{}",
        ArrayDimensionReason::Span
    );
}

fn assert_fragment(text: &str) {
    assert!(
        text.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
        "{text}"
    );
    assert!(!text.ends_with('.'), "{text}");
}

// A38 and domain-model.md section 2.1: an array has 1 to 3 dimensions.
#[test]
fn count_text_names_the_largest_dimension_count() {
    let text = ArrayDimensionReason::Count.to_string();
    assert_eq!(
        ArrayDimensions::parse(vec![dimension(0, 1); 3]).map(|d| d.len()),
        Ok(3)
    );
    assert_eq!(
        ArrayDimensions::parse(vec![dimension(0, 1); 4]),
        Err(ArrayDimensionReason::Count)
    );
    assert!(text.contains("1 to 3"), "{text}");
}

// domain-model.md section 2.1: the three reasons give three distinct fragments.
#[test]
fn reason_texts_are_distinct_lower_case_fragments_without_a_final_period() {
    let reasons = [
        ArrayDimensionReason::Order,
        ArrayDimensionReason::Span,
        ArrayDimensionReason::Count,
    ];
    let texts: Vec<String> = reasons.iter().map(ToString::to_string).collect();
    for text in &texts {
        assert_fragment(text);
    }
    let distinct: std::collections::BTreeSet<&String> = texts.iter().collect();
    assert_eq!(distinct.len(), reasons.len());
}

// A38: every parsed list has 1 to 3 dimensions, so it is never empty.
mod parsed_lists {
    use proptest::prelude::*;

    use super::*;

    fn bounds() -> impl Strategy<Value = (i32, i32)> {
        (-70_000_i32..70_000, 0_i32..65_536)
            .prop_map(|(lower, span)| (lower, lower.saturating_add(span)))
    }

    proptest! {
        #[test]
        fn a_parsed_list_is_never_empty(pairs in prop::collection::vec(bounds(), 1..=3)) {
            let list: Vec<ArrayDimension> = pairs
                .into_iter()
                .map(|(lower, upper)| dimension(lower, upper))
                .collect();
            let dims = ArrayDimensions::parse(list).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert!(!dims.is_empty());
            prop_assert!((1..=3).contains(&dims.len()));
        }
    }
}
