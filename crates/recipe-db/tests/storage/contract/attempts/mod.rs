//! Contract cases of `LoginAttemptStore` and `AttemptRepository` (task 5.6 of
//! `identity-and-sessions`).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The cases that
//! need PostgreSQL carry the prefix `postgres_only`. The attempts with a fixed age come from
//! `Harness::seed_attempts`.

mod postgres;
mod repository;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use recipe_app::ports::{AttemptKey, AttemptOutcome, AttemptRow};
use recipe_domain::identity::{ThrottlePolicy, TypedLogin};
use recipe_domain::timestamp::Timestamp;

use crate::contract::harness::{Harness, begin};

/// The address of the cases.
const FIRST: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
/// A second address.
const SECOND: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
/// An IPv6 address, to prove the round trip through `inet`.
const SIXTEEN: IpAddr = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));

fn typed(text: &str) -> TypedLogin {
    TypedLogin::parse(text).unwrap()
}

fn key(login: &str, client_ip: IpAddr) -> AttemptKey {
    AttemptKey {
        login: typed(login),
        client_ip,
    }
}

/// The seconds from `earlier` to `later`.
fn seconds_between(earlier: Timestamp, later: Timestamp) -> i64 {
    later
        .as_offset_date_time()
        .unix_timestamp()
        .checked_sub(earlier.as_offset_date_time().unix_timestamp())
        .unwrap()
}

async fn begin_attempt(
    h: &dyn Harness,
    key: &AttemptKey,
    policy: &ThrottlePolicy,
) -> AttemptOutcome {
    h.login_attempts().begin_attempt(key, policy).await.unwrap()
}

/// Read and remove the rows of a login in a transaction that rolls back.
async fn rows_of(h: &dyn Harness, login: &str) -> Vec<AttemptRow> {
    let mut tx = begin(h).await;
    let deleted = tx
        .attempts()
        .delete_for_login(&typed(login), 1000)
        .await
        .unwrap();
    assert_eq!(deleted.count, u64::try_from(deleted.newest.len()).unwrap());
    deleted.newest
}

async fn the_first_attempt_counts_nothing_and_inserts_one_failure(h: &dyn Harness) {
    let outcome = begin_attempt(h, &key("anna", FIRST), &ThrottlePolicy::default()).await;

    assert_eq!(outcome.count.pair_failures, 0);
    assert_eq!(outcome.count.last_pair_failure, None);
    assert_eq!(outcome.count.address_failures, 0);
    assert_eq!(outcome.count.oldest_address_failure, None);
    let rows = rows_of(h, "anna").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(Some(rows[0].id), outcome.attempt);
    assert_eq!(rows[0].login, typed("anna"));
    assert_eq!(rows[0].client_ip, FIRST);
    assert!(!rows[0].succeeded);
}

async fn the_second_attempt_counts_the_first(h: &dyn Harness) {
    let policy = ThrottlePolicy::default();
    let first = begin_attempt(h, &key("anna", FIRST), &policy).await;

    let second = begin_attempt(h, &key("anna", FIRST), &policy).await;

    assert_eq!(second.count.pair_failures, 1);
    assert_eq!(second.count.address_failures, 1);
    assert!(second.count.last_pair_failure.is_some());
    assert_eq!(
        second.count.last_pair_failure,
        second.count.oldest_address_failure
    );
    assert_ne!(first.attempt, second.attempt);
    assert_eq!(rows_of(h, "anna").await.len(), 2);
}

async fn the_counts_cover_the_pair_and_the_address(h: &dyn Harness) {
    h.seed_attempts(&typed("anna"), FIRST, &[100, 50], false)
        .await;
    h.seed_attempts(&typed("bob"), FIRST, &[80], false).await;
    h.seed_attempts(&typed("anna"), SECOND, &[70], false).await;

    let outcome = begin_attempt(h, &key("anna", FIRST), &ThrottlePolicy::default()).await;

    assert_eq!(outcome.count.pair_failures, 2);
    assert_eq!(outcome.count.address_failures, 3);
    let last = outcome.count.last_pair_failure.unwrap();
    let oldest = outcome.count.oldest_address_failure.unwrap();
    // The seeds ran before T1, so the database adds a little time to the age.
    assert!((50..60).contains(&seconds_between(last, outcome.count.now)));
    assert!((100..110).contains(&seconds_between(oldest, outcome.count.now)));
}

async fn attempts_outside_the_window_and_successes_do_not_count(h: &dyn Harness) {
    let policy = ThrottlePolicy::default();
    let outside = policy.window.as_secs().checked_add(1).unwrap();
    h.seed_attempts(&typed("anna"), FIRST, &[outside, 2 * 3600], false)
        .await;
    h.seed_attempts(&typed("anna"), FIRST, &[10], true).await;

    let outcome = begin_attempt(h, &key("anna", FIRST), &policy).await;

    assert_eq!(outcome.count.pair_failures, 0);
    assert_eq!(outcome.count.address_failures, 0);
    assert_eq!(outcome.count.last_pair_failure, None);
    assert_eq!(outcome.count.oldest_address_failure, None);
}

async fn a_reached_budget_inserts_no_row(h: &dyn Harness) {
    let policy = ThrottlePolicy {
        address_budget: 2,
        ..ThrottlePolicy::default()
    };
    h.seed_attempts(&typed("bob"), FIRST, &[30, 20], false)
        .await;
    h.seed_attempts(&typed("carl"), SECOND, &[10], false).await;

    let outcome = begin_attempt(h, &key("anna", FIRST), &policy).await;
    let other_address = begin_attempt(h, &key("anna", SECOND), &policy).await;

    assert_eq!(outcome.count.address_failures, 2);
    assert_eq!(outcome.attempt, None);
    assert_eq!(rows_of(h, "anna").await.len(), 1);
    assert_eq!(other_address.count.address_failures, 1);
    assert!(other_address.attempt.is_some());
}

async fn an_ipv6_address_round_trips(h: &dyn Harness) {
    h.seed_attempts(&typed("anna"), SIXTEEN, &[30], false).await;
    let policy = ThrottlePolicy::default();

    let outcome = begin_attempt(h, &key("anna", SIXTEEN), &policy).await;

    assert_eq!(outcome.count.pair_failures, 1);
    let rows = rows_of(h, "anna").await;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.client_ip == SIXTEEN));
}

async fn a_mapped_ipv4_address_counts_as_its_ipv4_form(h: &dyn Harness) {
    let policy = ThrottlePolicy::default();
    let mapped = IpAddr::V6(Ipv4Addr::new(10, 0, 0, 5).to_ipv6_mapped());
    let plain = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
    begin_attempt(h, &key("anna", mapped), &policy).await;

    let second = begin_attempt(h, &key("anna", plain), &policy).await;

    assert_eq!(second.count.pair_failures, 1);
    assert_eq!(second.count.address_failures, 1);
    let rows = rows_of(h, "anna").await;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.client_ip == plain));
}

contract_case!(
    a_mapped_ipv4_address_counts_as_its_ipv4_form,
    a_mapped_ipv4_address_counts_as_its_ipv4_form_memory,
    a_mapped_ipv4_address_counts_as_its_ipv4_form_postgres
);
contract_case!(
    the_first_attempt_counts_nothing_and_inserts_one_failure,
    the_first_attempt_counts_nothing_and_inserts_one_failure_memory,
    the_first_attempt_counts_nothing_and_inserts_one_failure_postgres
);
contract_case!(
    the_second_attempt_counts_the_first,
    the_second_attempt_counts_the_first_memory,
    the_second_attempt_counts_the_first_postgres
);
contract_case!(
    the_counts_cover_the_pair_and_the_address,
    the_counts_cover_the_pair_and_the_address_memory,
    the_counts_cover_the_pair_and_the_address_postgres
);
contract_case!(
    attempts_outside_the_window_and_successes_do_not_count,
    attempts_outside_the_window_and_successes_do_not_count_memory,
    attempts_outside_the_window_and_successes_do_not_count_postgres
);
contract_case!(
    a_reached_budget_inserts_no_row,
    a_reached_budget_inserts_no_row_memory,
    a_reached_budget_inserts_no_row_postgres
);
contract_case!(
    an_ipv6_address_round_trips,
    an_ipv6_address_round_trips_memory,
    an_ipv6_address_round_trips_postgres
);
