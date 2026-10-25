//! Tests of the catch-up rule (design D4 step 9, ADR-0060).

use std::time::Duration;

use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::needs_catchup;

const AGE: Duration = Duration::from_secs(24 * 60 * 60);

fn at(seconds: i64) -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

#[test]
fn a_missing_check_needs_the_catchup() {
    assert!(needs_catchup(None, at(1_000_000), AGE));
}

#[test]
fn a_check_younger_than_the_age_needs_no_catchup() {
    let now = 1_000_000;
    assert!(!needs_catchup(Some(at(now - 3_600)), at(now), AGE));
}

#[test]
fn a_check_of_exactly_the_age_needs_no_catchup() {
    let now = 1_000_000;
    assert!(!needs_catchup(Some(at(now - 86_400)), at(now), AGE));
}

#[test]
fn a_check_older_than_the_age_needs_the_catchup() {
    let now = 1_000_000;
    assert!(needs_catchup(Some(at(now - 86_401)), at(now), AGE));
}

#[test]
fn a_check_in_the_future_needs_no_catchup() {
    let now = 1_000_000;
    assert!(!needs_catchup(Some(at(now + 60)), at(now), AGE));
}
