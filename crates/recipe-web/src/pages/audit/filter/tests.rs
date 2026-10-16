//! Tests of the parsers of the audit filter values (ADR-0051 (d), design D6).

use super::{cursor_url, present, time_range, uuid};
use crate::pages::recipes::fields::FieldLines;

/// Return the number of bad lines that `time_range` adds for the two fields.
fn bad_lines(from: Option<&str>, to: Option<&str>) -> usize {
    let mut lines = FieldLines::default();
    let _range = time_range(&mut lines, from, to);
    lines.into_lines().map_or(0, |lines| lines.iter().count())
}

#[test]
fn filter_a_day_range_ends_at_the_start_of_the_next_day() {
    let mut lines = FieldLines::default();
    let range = time_range(&mut lines, Some("2026-10-16"), Some("2026-10-16"));
    assert!(lines.into_lines().is_none());
    let from = range.from.unwrap().as_offset_date_time().unix_timestamp();
    let until = range.until.unwrap().as_offset_date_time().unix_timestamp();
    assert_eq!(until.checked_sub(from), Some(86_400));
    assert_eq!(from.checked_rem(86_400), Some(0));
}

#[test]
fn filter_the_last_day_of_the_calendar_sets_no_upper_bound() {
    let mut lines = FieldLines::default();
    let range = time_range(&mut lines, None, Some("9999-12-31"));
    assert!(lines.into_lines().is_none());
    assert!(range.until.is_none());
    assert!(range.from.is_none());
}

#[test]
fn filter_a_bad_date_adds_one_line_and_an_empty_field_none() {
    for bad in [
        "2026-13-01",
        "2026-02-30",
        "2026-1-1",
        "26-01-01",
        "2026/01/01",
        "0000-01-01",
        "2026-01-0a",
        " 2026-01-01",
        "２０２６-01-01",
    ] {
        assert_eq!(bad_lines(Some(bad), None), 1, "{bad}");
        assert_eq!(bad_lines(None, Some(bad)), 1, "{bad}");
    }
    assert_eq!(bad_lines(Some(""), Some("")), 0);
    assert_eq!(bad_lines(Some("2026-13-01"), Some("2026-13-02")), 2);
}

#[test]
fn filter_a_range_with_to_before_from_adds_a_line_for_to() {
    assert_eq!(bad_lines(Some("2026-10-02"), Some("2026-10-01")), 1);
    assert_eq!(bad_lines(Some("2026-10-01"), Some("2026-10-01")), 0);
}

#[test]
fn filter_present_drops_an_empty_text() {
    assert_eq!(present(Some("")), None);
    assert_eq!(present(None), None);
    assert_eq!(present(Some("x")), Some("x"));
}

#[test]
fn filter_uuid_takes_the_hyphenated_form_only() {
    assert!(uuid("aggregate", "12345678-0000-0000-0000-000000000000").is_ok());
    for bad in [
        "",
        "12345678000000000000000000000000",
        "{12345678-0000-0000-0000-000000000000}",
    ] {
        assert!(uuid("aggregate", bad).is_err(), "{bad}");
    }
}

#[test]
fn filter_cursor_url_encodes_the_values_and_puts_the_cursor_last() {
    let url = cursor_url(
        "/audit",
        &[("actor", "a b"), ("action", ""), ("from", "2026-10-16")],
        Some(7),
    );
    assert_eq!(url, "/audit?actor=a%20b&from=2026-10-16&before=7");
    assert_eq!(cursor_url("/audit", &[("actor", "")], None), "/audit");
}
