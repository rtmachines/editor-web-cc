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
