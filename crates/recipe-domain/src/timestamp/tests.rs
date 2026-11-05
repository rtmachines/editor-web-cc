use super::*;
use time::OffsetDateTime;

#[test]
fn timestamp_round_trips_an_offset_date_time() {
    let instant = OffsetDateTime::UNIX_EPOCH;
    assert_eq!(Timestamp::from(instant).as_offset_date_time(), &instant);
}
