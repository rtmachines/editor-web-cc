use std::collections::BTreeMap;

use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};

use crate::error_codes::ErrorCode;
use crate::export::{DocumentFacts, ExportError, ExportStatus, SCHEMA_VERSION, Sha256, utc_text};
use crate::identity::Login;
use crate::text::Text;
use crate::timestamp::Timestamp;

/// Build an instant from its local date, local time, nanosecond, and UTC offset in hours.
fn instant(
    (year, month, day): (i32, Month, u8),
    (hour, minute, second): (u8, u8, u8),
    nanosecond: u32,
    offset_hours: i8,
) -> Timestamp {
    let date = Date::from_calendar_date(year, month, day).unwrap();
    let time = Time::from_hms_nano(hour, minute, second, nanosecond).unwrap();
    let offset = UtcOffset::from_hms(offset_hours, 0, 0).unwrap();
    Timestamp::from(PrimitiveDateTime::new(date, time).assume_offset(offset))
}

fn facts(status: ExportStatus) -> DocumentFacts {
    let at = instant((2026, Month::October, 11), (14, 2, 10), 0, 0);
    DocumentFacts {
        exported_at: at,
        exported_by: Login::parse("mhoffman").unwrap(),
        status,
        approved_at: at,
        approved_by: Login::parse("anowak").unwrap(),
        machine_name: Text::parse("Filler 2").unwrap(),
        recipe_name: Text::parse("Standard bottle").unwrap(),
        content_sha256: Sha256::from([0_u8; 32]),
        names: BTreeMap::new(),
    }
}

#[test]
fn utc_text_converts_an_offset_of_plus_two_hours_to_utc() {
    let local = instant((2026, Month::October, 11), (16, 2, 10), 0, 2);

    assert_eq!(utc_text(local).unwrap(), "2026-10-11T14:02:10Z");
}

#[test]
fn utc_text_drops_the_fraction_of_a_second() {
    let half = instant((2026, Month::October, 11), (14, 2, 10), 500_000_000, 0);
    let last = instant((2026, Month::December, 31), (23, 59, 59), 999_999_999, 0);

    assert_eq!(utc_text(half).unwrap(), "2026-10-11T14:02:10Z");
    assert_eq!(utc_text(last).unwrap(), "2026-12-31T23:59:59Z");
}

/// Catches a range `0..MAX_YEAR` in place of `0..=MAX_YEAR`: the last instant of the year 9999 fails.
#[test]
fn utc_text_writes_four_digits_at_both_ends_of_the_year_range() {
    let first = instant((0, Month::January, 1), (0, 0, 0), 0, 0);
    let last = instant((9999, Month::December, 31), (23, 59, 59), 0, 0);

    assert_eq!(utc_text(first).unwrap(), "0000-01-01T00:00:00Z");
    assert_eq!(utc_text(last).unwrap(), "9999-12-31T23:59:59Z");
}

/// Catches a swap of `checked_to_offset` to `to_offset`: the UTC instant of the last input is in
/// the year 10000, outside the range of `time`, and `to_offset` panics there.
#[test]
fn utc_text_rejects_a_year_outside_0000_to_9999() {
    let negative = instant((-1, Month::June, 1), (12, 0, 0), 0, 0);
    let before_zero_in_utc = instant((0, Month::January, 1), (1, 0, 0), 0, 2);
    let after_9999_in_utc = instant((9999, Month::December, 31), (23, 0, 0), 0, -2);

    for input in [negative, before_zero_in_utc, after_9999_in_utc] {
        let error = utc_text(input).unwrap_err();
        assert!(matches!(error, ExportError::Serialize(_)), "{input:?}");
        assert_eq!(error.code(), ErrorCode::EXP_004);
    }
}

#[test]
fn export_status_serializes_as_its_name() {
    assert_eq!(
        serde_json::to_string(&ExportStatus::Approved).unwrap(),
        r#""Approved""#
    );
    assert_eq!(
        serde_json::to_string(&ExportStatus::Retired).unwrap(),
        r#""Retired""#
    );
}

#[test]
fn only_an_approved_document_is_loadable() {
    assert!(facts(ExportStatus::Approved).loadable());
    assert!(!facts(ExportStatus::Retired).loadable());
}

#[test]
fn the_schema_version_is_one() {
    assert_eq!(SCHEMA_VERSION, "1");
}
