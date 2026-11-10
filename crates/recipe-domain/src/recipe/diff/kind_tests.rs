//! Tests of the query text of a diff section (ADR-0050 (e)).

use crate::recipe::{DiffKind, DiffKindReason};

#[test]
fn diff_kind_round_trips_through_its_query_text() {
    let texts: Vec<&str> = DiffKind::ALL.iter().map(|kind| kind.as_str()).collect();
    assert_eq!(
        texts,
        [
            "limits",
            "values",
            "definitions",
            "translations",
            "added",
            "removed"
        ]
    );
    for kind in DiffKind::ALL {
        assert_eq!(DiffKind::parse(kind.as_str()), Ok(kind));
    }
}

#[test]
fn diff_kind_rejects_any_other_text() {
    for text in [
        "", "Limits", "limit", " values", "values ", "all", "summary",
    ] {
        assert_eq!(
            DiffKind::parse(text),
            Err(DiffKindReason::Unknown),
            "{text:?}"
        );
    }
}

// ADR-0050 (e): the reason names a section of a diff, and the enum has no code.
#[test]
fn unknown_kind_text_names_a_section_as_a_lower_case_fragment() {
    assert_eq!(DiffKind::ALL.len(), 6);
    let text = DiffKindReason::Unknown.to_string();
    assert!(
        text.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
        "{text}"
    );
    assert!(!text.ends_with('.'), "{text}");
    assert!(text.contains("section"), "{text}");
}
