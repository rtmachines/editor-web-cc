use std::thread;
use std::time::Duration;

use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::ReadinessState;

fn at(seconds: i64) -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

fn assert_shareable<T: Clone + Send + Sync + 'static>() {}

#[test]
fn state_is_cheap_to_clone_and_shareable_between_threads() {
    assert_shareable::<ReadinessState>();
}

#[test]
fn a_new_state_is_not_ready_since_the_start() {
    let state = ReadinessState::new(at(100));
    assert!(!state.is_ready());
    assert_eq!(state.not_ready_for(at(190)), Some(Duration::from_secs(90)));
}

#[test]
fn mark_ready_ends_the_not_ready_time() {
    let state = ReadinessState::new(at(100));
    state.mark_ready();
    assert!(state.is_ready());
    assert_eq!(state.not_ready_for(at(1000)), None);
}

#[test]
fn a_second_not_ready_mark_keeps_the_first_start() {
    let state = ReadinessState::new(at(0));
    state.mark_ready();
    state.mark_not_ready(at(200));
    state.mark_not_ready(at(260));
    assert!(!state.is_ready());
    assert_eq!(state.not_ready_for(at(300)), Some(Duration::from_secs(100)));
}

#[test]
fn a_new_outage_after_a_recovery_starts_a_new_count() {
    let state = ReadinessState::new(at(0));
    state.mark_ready();
    state.mark_not_ready(at(10));
    state.mark_ready();
    state.mark_not_ready(at(500));
    assert_eq!(state.not_ready_for(at(530)), Some(Duration::from_secs(30)));
}

#[test]
fn a_clock_that_went_back_gives_zero_and_not_a_panic() {
    let state = ReadinessState::new(at(100));
    assert_eq!(state.not_ready_for(at(40)), Some(Duration::ZERO));
}

#[test]
fn clones_share_one_state() {
    let first = ReadinessState::new(at(0));
    let second = first.clone();
    first.mark_ready();
    assert!(second.is_ready());
    second.mark_not_ready(at(5));
    assert!(!first.is_ready());
}

#[test]
fn a_reader_in_another_thread_sees_the_ready_mark() {
    let state = ReadinessState::new(at(0));
    let writer = state.clone();
    thread::spawn(move || writer.mark_ready()).join().unwrap();
    assert!(state.is_ready());
}
