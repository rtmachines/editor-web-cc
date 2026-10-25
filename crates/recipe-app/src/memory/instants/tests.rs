use std::time::Duration;

use recipe_domain::timestamp::Timestamp;
use time::{Date, PrimitiveDateTime, Time};

use super::earlier;

fn unix(seconds: i64) -> Timestamp {
    Timestamp::from(time::OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

#[test]
fn earlier_subtracts_the_span() {
    assert_eq!(earlier(unix(1_000), Duration::from_secs(400)), unix(600));
}

#[test]
fn earlier_stops_at_the_smallest_instant_on_overflow() {
    let smallest = PrimitiveDateTime::new(Date::MIN, Time::MIDNIGHT).assume_utc();
    let result = earlier(Timestamp::from(smallest), Duration::from_secs(1));
    assert_eq!(result, Timestamp::from(smallest));
}
