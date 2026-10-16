//! Tests of the UTC day and of the range of two days (ADR-0051 (d), design D6).
//!
//! The expected instants are Unix times that a calendar gives, not values that the code computes.

use time::OffsetDateTime;

use super::day::{DayRangeReason, DayReason, UtcDay, day_range};
use crate::ports::TimeRange;
use recipe_domain::timestamp::Timestamp;

/// 2000-01-01T00:00:00Z.
const Y2000: i64 = 946_684_800;
/// 2000-01-02T00:00:00Z.
const Y2000_NEXT_DAY: i64 = 946_771_200;
/// 9999-12-31T00:00:00Z.
const LAST_DAY: i64 = 253_402_214_400;
/// 0001-01-01T00:00:00Z.
const FIRST_DAY: i64 = -62_135_596_800;
/// 2024-02-29T00:00:00Z.
const LEAP_DAY: i64 = 1_709_164_800;

/// Return the instant `seconds` after the Unix epoch.
fn at(seconds: i64) -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

/// Parse `text` as a day. The text must be valid.
fn day(text: &str) -> UtcDay {
    UtcDay::parse(text).unwrap()
}

#[test]
fn utc_day_accepts_a_date_of_the_years_1_to_9999() {
    for text in [
        "2026-10-16",
        "0001-01-01",
        "9999-12-31",
        "2000-02-29",
        "2024-02-29",
        "2026-12-31",
        "2026-01-31",
    ] {
        assert!(UtcDay::parse(text).is_ok(), "{text}");
    }
}

#[test]
fn utc_day_refuses_text_that_is_not_the_grammar_year_month_day() {
    for text in [
        "",
        "2026-1-1",
        "2026-10-6",
        "26-10-16",
        "2026/10/16",
        "2026.10.16",
        "20261016",
        "2026-10-16 ",
        " 2026-10-16",
        "2026-10-166",
        "2026-10-16T00:00:00Z",
        "+026-10-16",
        "-001-10-16",
        "2026-+1-16",
        "abcd-ef-gh",
        "2026-１0-16",
        "２０２６-10-16",
    ] {
        assert_eq!(UtcDay::parse(text), Err(DayReason::Grammar), "{text:?}");
    }
}

#[test]
fn utc_day_refuses_a_day_that_the_calendar_does_not_have() {
    for text in [
        "0000-01-01",
        "2026-00-10",
        "2026-13-01",
        "2026-10-00",
        "2026-10-32",
        "2026-02-30",
        "2026-04-31",
        "2023-02-29",
        "1900-02-29",
    ] {
        assert_eq!(UtcDay::parse(text), Err(DayReason::Calendar), "{text:?}");
    }
}

#[test]
fn utc_day_orders_the_days_by_the_calendar() {
    assert!(day("2026-10-15") < day("2026-10-16"));
    assert!(day("2026-09-30") < day("2026-10-01"));
    assert!(day("2025-12-31") < day("2026-01-01"));
    assert_eq!(day("2026-10-16"), day("2026-10-16"));
}

#[test]
fn day_range_without_a_day_has_no_bound() {
    assert_eq!(day_range(None, None), Ok(TimeRange::default()));
}

#[test]
fn day_range_from_starts_at_the_start_of_its_day_in_utc() {
    let range = day_range(Some(day("2000-01-01")), None).unwrap();
    assert_eq!(range.from, Some(at(Y2000)));
    assert_eq!(range.until, None);
}

#[test]
fn day_range_to_ends_at_the_start_of_the_next_day() {
    let range = day_range(None, Some(day("2000-01-01"))).unwrap();
    assert_eq!(range.from, None);
    assert_eq!(range.until, Some(at(Y2000_NEXT_DAY)));
}

#[test]
fn day_range_holds_both_days_and_splits_at_midnight_utc() {
    let range = day_range(Some(day("2000-01-01")), Some(day("2000-01-01"))).unwrap();
    assert_eq!(range.from, Some(at(Y2000)));
    assert_eq!(range.until, Some(at(Y2000_NEXT_DAY)));
    assert!(!range.contains(at(Y2000.checked_sub(1).unwrap())));
    assert!(range.contains(at(Y2000)));
    assert!(range.contains(at(Y2000_NEXT_DAY.checked_sub(1).unwrap())));
    assert!(!range.contains(at(Y2000_NEXT_DAY)));
}

#[test]
fn day_range_to_the_last_day_of_the_year_9999_sets_no_upper_bound() {
    let range = day_range(None, Some(day("9999-12-31"))).unwrap();
    assert_eq!(range.until, None);
    let both = day_range(Some(day("9999-12-30")), Some(day("9999-12-31"))).unwrap();
    assert_eq!(both.from, Some(at(LAST_DAY.checked_sub(86_400).unwrap())));
    assert_eq!(both.until, None);
    assert!(both.contains(at(LAST_DAY.checked_add(86_399).unwrap())));
}

#[test]
fn day_range_from_the_first_day_and_to_a_leap_day_keep_their_instants() {
    let range = day_range(Some(day("0001-01-01")), Some(day("2024-02-29"))).unwrap();
    assert_eq!(range.from, Some(at(FIRST_DAY)));
    assert_eq!(range.until, Some(at(LEAP_DAY.checked_add(86_400).unwrap())));
}

#[test]
fn day_range_refuses_a_day_to_before_a_day_from() {
    let result = day_range(Some(day("2026-10-02")), Some(day("2026-10-01")));
    assert_eq!(result, Err(DayRangeReason::ToBeforeFrom));
    let across_year = day_range(Some(day("2026-01-01")), Some(day("2025-12-31")));
    assert_eq!(across_year, Err(DayRangeReason::ToBeforeFrom));
}

#[test]
fn day_range_accepts_the_same_day_for_from_and_to() {
    assert!(day_range(Some(day("2026-10-16")), Some(day("2026-10-16"))).is_ok());
}
