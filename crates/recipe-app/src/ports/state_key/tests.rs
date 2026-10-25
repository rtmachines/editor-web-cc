//! The key texts and the value texts of the three state keys (`data-model.md` section 3.17,
//! ADR-0073 (i)).

use recipe_domain::timestamp::Timestamp;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

use super::{StateValue, TimeStateKey, parse_chain_seq_text};
use crate::ports::ChainSeq;

fn instant(seconds: i64) -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

/// The text of a time value. Both time variants share one form.
fn time_text(at: Timestamp) -> Option<String> {
    StateValue::HeartbeatLastAt(at).to_text()
}

/// The text of a `chain_seq` value.
fn seq_text(number: i64) -> Option<String> {
    StateValue::AuditLastCheckSeq(ChainSeq::from(number)).to_text()
}

/// The first instant of `year` in UTC, which can lie outside the range of the text form.
fn start_of_year(year: i32) -> Timestamp {
    let date = Date::from_calendar_date(year, Month::January, 1).unwrap();
    Timestamp::from(PrimitiveDateTime::new(date, Time::MIDNIGHT).assume_utc())
}

/// The literal key of a time key. The `match` has no wildcard arm, so a new variant breaks
/// the build.
fn time_key_literal(key: TimeStateKey) -> &'static str {
    match key {
        TimeStateKey::AuditLastCheckAt => "audit.last_check_at",
        TimeStateKey::HeartbeatLastAt => "heartbeat.last_at",
    }
}

/// The literal key of a value. The `match` has no wildcard arm, so a new variant breaks the build.
fn value_literal(value: &StateValue) -> &'static str {
    match value {
        StateValue::AuditLastCheckSeq(_) => "audit.last_check_seq",
        StateValue::AuditLastCheckAt(_) => "audit.last_check_at",
        StateValue::HeartbeatLastAt(_) => "heartbeat.last_at",
    }
}

#[test]
fn each_time_state_key_has_its_literal_key_text() {
    let keys = [
        TimeStateKey::AuditLastCheckAt,
        TimeStateKey::HeartbeatLastAt,
    ];
    for key in keys {
        assert_eq!(key.key(), time_key_literal(key), "key: {key:?}");
    }
    assert_eq!(keys.len(), 2);
}

#[test]
fn each_state_value_has_its_literal_key_text() {
    let values = [
        StateValue::AuditLastCheckSeq(ChainSeq::from(41)),
        StateValue::AuditLastCheckAt(instant(500)),
        StateValue::HeartbeatLastAt(instant(600)),
    ];
    for value in &values {
        assert_eq!(value.key(), value_literal(value), "value: {value:?}");
    }
    assert_eq!(values.len(), 3);
}

#[test]
fn a_time_value_and_its_time_key_name_the_same_row() {
    assert_eq!(
        StateValue::AuditLastCheckAt(instant(1)).key(),
        TimeStateKey::AuditLastCheckAt.key()
    );
    assert_eq!(
        StateValue::HeartbeatLastAt(instant(1)).key(),
        TimeStateKey::HeartbeatLastAt.key()
    );
}

#[test]
fn the_sequence_text_is_decimal_without_sign_or_leading_zero() {
    assert_eq!(seq_text(0).as_deref(), Some("0"));
    assert_eq!(seq_text(12_345).as_deref(), Some("12345"));
    assert_eq!(seq_text(i64::MAX).as_deref(), Some("9223372036854775807"));
}

#[test]
fn a_negative_sequence_has_no_text() {
    assert_eq!(seq_text(-1), None);
    assert_eq!(seq_text(i64::MIN), None);
}

#[test]
fn parse_chain_seq_reads_what_to_text_wrote() {
    for number in [0, 1, 9, 10, 12_345, i64::MAX] {
        let text = seq_text(number).unwrap();
        assert_eq!(
            parse_chain_seq_text(&text),
            Some(ChainSeq::from(number)),
            "number: {number}"
        );
    }
}

#[test]
fn parse_chain_seq_refuses_every_other_form() {
    let bad = [
        "",
        "-1",
        "-0",
        "+1",
        "01",
        "00",
        " 1",
        "1 ",
        "1.0",
        "1e3",
        "x",
        "\u{ff11}",
        "9223372036854775808",
        "99999999999999999999999",
    ];
    for text in bad {
        assert_eq!(parse_chain_seq_text(text), None, "text: {text:?}");
    }
}

#[test]
fn the_time_text_has_the_suffix_z_and_six_fraction_digits() {
    let text = time_text(instant(1_000_000_000));
    assert_eq!(text.as_deref(), Some("2001-09-09T01:46:40.000000Z"));
}

#[test]
fn both_time_variants_have_the_same_text() {
    let at = instant(1_000_000_000);
    assert_eq!(
        StateValue::AuditLastCheckAt(at).to_text(),
        StateValue::HeartbeatLastAt(at).to_text()
    );
}

#[test]
fn the_time_text_cuts_nanoseconds_to_microseconds() {
    let at = OffsetDateTime::from_unix_timestamp_nanos(1_000_000_000_123_456_789).unwrap();
    let text = time_text(Timestamp::from(at));
    assert_eq!(text.as_deref(), Some("2001-09-09T01:46:40.123456Z"));
}

#[test]
fn the_time_text_converts_another_offset_to_utc() {
    let offset = UtcOffset::from_hms(2, 0, 0).unwrap();
    let at = OffsetDateTime::from_unix_timestamp(1_000_000_000)
        .unwrap()
        .to_offset(offset);
    let text = time_text(Timestamp::from(at));
    assert_eq!(text.as_deref(), Some("2001-09-09T01:46:40.000000Z"));
}

#[test]
fn the_year_zero_has_a_text() {
    assert_eq!(
        time_text(start_of_year(0)).as_deref(),
        Some("0000-01-01T00:00:00.000000Z")
    );
}

/// The old memory copy gave `-001-01-01T00:00:00.000000Z` for this instant. Its own parser then
/// refused that text, because the text has 28 bytes.
#[test]
fn the_year_minus_one_has_no_text() {
    assert_eq!(time_text(start_of_year(-1)), None);
}

#[test]
fn the_year_10000_in_utc_has_no_text() {
    let offset = UtcOffset::from_hms(-2, 0, 0).unwrap();
    let last = PrimitiveDateTime::new(Date::MAX, Time::MAX).assume_offset(offset);
    assert_eq!(time_text(Timestamp::from(last)), None);
}

#[test]
fn parse_reads_the_example_of_the_adr() {
    let parsed = TimeStateKey::parse_text("2026-10-23T02:00:00.000000Z");
    assert_eq!(parsed, Some(instant(1_792_720_800)));
}

#[test]
fn parse_reads_what_to_text_wrote() {
    let at = OffsetDateTime::from_unix_timestamp_nanos(1_000_000_000_123_456_000).unwrap();
    let wanted = Timestamp::from(at);
    let text = time_text(wanted).unwrap();
    assert_eq!(TimeStateKey::parse_text(&text), Some(wanted));
}

#[test]
fn parse_reads_the_year_zero_and_the_year_9999() {
    for year in [0, 9999] {
        let wanted = start_of_year(year);
        let text = time_text(wanted).unwrap();
        assert_eq!(
            TimeStateKey::parse_text(&text),
            Some(wanted),
            "year: {year}"
        );
    }
}

#[test]
fn parse_refuses_every_other_form() {
    let bad = [
        "",
        "2026-10-23T02:00:00Z",
        "2026-10-23T02:00:00.00000Z",
        "2026-10-23T02:00:00.0000000Z",
        "2026-10-23T02:00:00.000000+00:00",
        "2026-10-23T02:00:00.000000z",
        "2026-10-23 02:00:00.000000Z",
        "2026/10/23T02:00:00.000000Z",
        "2026-13-23T02:00:00.000000Z",
        "2026-10-32T02:00:00.000000Z",
        "2026-02-30T02:00:00.000000Z",
        "2026-10-23T24:00:00.000000Z",
        "2026-10-23T02:60:00.000000Z",
        "2026-10-23T02:00:60.000000Z",
        "2026-10-23T02:00:00.00000aZ",
        "2026-10-23T02:00:00.000000Z ",
        "+026-10-23T02:00:00.000000Z",
        "-001-01-01T00:00:00.000000Z",
        "2026-10-2\u{ff13}T02:00:00.000000Z",
        "12345",
    ];
    for text in bad {
        assert_eq!(TimeStateKey::parse_text(text), None, "text: {text:?}");
    }
}
