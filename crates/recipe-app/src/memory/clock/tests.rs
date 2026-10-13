use time::{Duration, OffsetDateTime};

use super::MemoryClock;
use crate::ports::Clock;
use recipe_domain::timestamp::Timestamp;

#[test]
fn the_clock_returns_its_start_time() {
    let clock = MemoryClock::new(OffsetDateTime::UNIX_EPOCH);
    assert_eq!(clock.now(), Timestamp::from(OffsetDateTime::UNIX_EPOCH));
}

#[test]
fn set_replaces_the_time() {
    let clock = MemoryClock::new(OffsetDateTime::UNIX_EPOCH);
    let later = Timestamp::from(OffsetDateTime::UNIX_EPOCH + Duration::hours(5));
    clock.set(later);
    assert_eq!(clock.now(), later);
}

#[test]
fn advance_moves_the_time_forward() {
    let clock = MemoryClock::new(OffsetDateTime::UNIX_EPOCH);
    clock.advance(Duration::seconds(90));
    clock.advance(Duration::seconds(30));
    let expected = Timestamp::from(OffsetDateTime::UNIX_EPOCH + Duration::seconds(120));
    assert_eq!(clock.now(), expected);
}

#[test]
fn a_clone_shares_the_time() {
    let clock = MemoryClock::new(OffsetDateTime::UNIX_EPOCH);
    let copy = clock.clone();
    clock.advance(Duration::minutes(1));
    assert_eq!(copy.now(), clock.now());
}

#[test]
fn the_default_clock_starts_at_a_fixed_time() {
    assert_eq!(MemoryClock::default().now(), MemoryClock::default().now());
}
