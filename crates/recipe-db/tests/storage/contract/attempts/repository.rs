//! Contract cases of `AttemptRepository`.

use std::time::Duration;

use recipe_app::ports::AttemptId;
use recipe_domain::identity::ThrottlePolicy;

use super::{FIRST, SECOND, begin_attempt, key, rows_of, seconds_between, typed};
use crate::contract::harness::{Harness, begin};

/// The window of `GetUser`: 15 minutes.
const WINDOW: Duration = Duration::from_secs(15 * 60);

async fn mark_succeeded_sets_only_the_flag(h: &dyn Harness) {
    let ids = h
        .seed_attempts(&typed("anna"), FIRST, &[30, 20], false)
        .await;
    let before = rows_of(h, "anna").await;
    let mut tx = begin(h).await;

    tx.attempts().mark_succeeded(ids[1]).await.unwrap();
    let after = tx
        .attempts()
        .delete_for_login(&typed("anna"), 10)
        .await
        .unwrap()
        .newest;

    assert_eq!(after.len(), 2);
    for row in &after {
        let old = before.iter().find(|old| old.id == row.id).unwrap();
        assert_eq!(row.login, old.login);
        assert_eq!(row.client_ip, old.client_ip);
        assert_eq!(row.attempted_at, old.attempted_at);
        assert_eq!(row.succeeded, row.id == ids[1]);
    }
}

async fn mark_succeeded_of_an_attempt_that_an_unlock_deleted_is_ok(h: &dyn Harness) {
    let outcome = begin_attempt(h, &key("anna", FIRST), &ThrottlePolicy::default()).await;
    let id = outcome.attempt.expect("T1 inserted a row");
    let mut tx = begin(h).await;

    let deleted = tx
        .attempts()
        .delete_for_login(&typed("anna"), 10)
        .await
        .unwrap();
    let result = tx.attempts().mark_succeeded(id).await;

    assert_eq!(deleted.count, 1);
    assert!(result.is_ok(), "{result:?}");
    let rest = tx
        .attempts()
        .delete_for_login(&typed("anna"), 10)
        .await
        .unwrap();
    assert_eq!(rest.count, 0, "the mark inserts no row");
}

async fn mark_succeeded_of_an_unknown_attempt_is_ok(h: &dyn Harness) {
    let mut tx = begin(h).await;

    let result = tx.attempts().mark_succeeded(AttemptId::from(987_654)).await;

    assert!(result.is_ok(), "{result:?}");
}

async fn delete_for_login_gives_the_newest_100_of_250_and_the_count(h: &dyn Harness) {
    let ages: Vec<u64> = (1..=250).collect();
    let ids = h.seed_attempts(&typed("anna"), FIRST, &ages, false).await;
    let others = h.seed_attempts(&typed("bob"), SECOND, &[5, 6], false).await;
    let mut tx = begin(h).await;

    let deleted = tx
        .attempts()
        .delete_for_login(&typed("anna"), 100)
        .await
        .unwrap();
    let again = tx
        .attempts()
        .delete_for_login(&typed("anna"), 100)
        .await
        .unwrap();
    let bob = tx
        .attempts()
        .delete_for_login(&typed("bob"), 100)
        .await
        .unwrap();

    assert_eq!(deleted.count, 250);
    // The smallest ages are the newest rows, and the newest row comes first.
    let newest: Vec<AttemptId> = deleted.newest.iter().map(|row| row.id).collect();
    assert_eq!(newest, ids[..100]);
    assert!(
        deleted
            .newest
            .iter()
            .all(|row| row.login == typed("anna") && row.client_ip == FIRST && !row.succeeded)
    );
    assert_eq!((again.count, again.newest.len()), (0, 0));
    assert_eq!(bob.count, 2);
    assert_eq!(
        bob.newest.iter().map(|row| row.id).collect::<Vec<_>>(),
        others
    );
}

async fn delete_for_login_with_keep_zero_still_gives_the_count(h: &dyn Harness) {
    h.seed_attempts(&typed("anna"), FIRST, &[1, 2, 3], false)
        .await;
    let mut tx = begin(h).await;

    let deleted = tx
        .attempts()
        .delete_for_login(&typed("anna"), 0)
        .await
        .unwrap();

    assert_eq!(deleted.count, 3);
    assert!(deleted.newest.is_empty());
}

async fn throttle_state_counts_the_failures_over_every_address(h: &dyn Harness) {
    let outside = WINDOW.as_secs().checked_add(1).unwrap();
    h.seed_attempts(&typed("anna"), FIRST, &[100], false).await;
    h.seed_attempts(&typed("anna"), SECOND, &[50], false).await;
    h.seed_attempts(&typed("anna"), FIRST, &[10], true).await;
    h.seed_attempts(&typed("anna"), FIRST, &[outside, 2 * 3600], false)
        .await;
    h.seed_attempts(&typed("bob"), FIRST, &[5], false).await;
    let mut tx = begin(h).await;

    let state = tx
        .attempts()
        .throttle_state(&typed("anna"), WINDOW)
        .await
        .unwrap();
    let quiet = tx
        .attempts()
        .throttle_state(&typed("carl"), WINDOW)
        .await
        .unwrap();

    assert_eq!(state.failures, 2);
    let last = state.last_failure.expect("the login has failures");
    // The seeds ran before this call, so the database adds a little time to the age.
    let rows = tx
        .attempts()
        .delete_for_login(&typed("anna"), 10)
        .await
        .unwrap()
        .newest;
    let newest_failure = rows
        .iter()
        .filter(|row| !row.succeeded && row.attempted_at == last)
        .count();
    assert_eq!(newest_failure, 1);
    let youngest = rows.iter().map(|row| row.attempted_at).max().unwrap();
    assert!(seconds_between(last, youngest) >= 40);
    assert_eq!(quiet.failures, 0);
    assert_eq!(quiet.last_failure, None);
}

contract_case!(
    mark_succeeded_sets_only_the_flag,
    mark_succeeded_sets_only_the_flag_memory,
    mark_succeeded_sets_only_the_flag_postgres
);
contract_case!(
    mark_succeeded_of_an_attempt_that_an_unlock_deleted_is_ok,
    mark_succeeded_of_an_attempt_that_an_unlock_deleted_is_ok_memory,
    mark_succeeded_of_an_attempt_that_an_unlock_deleted_is_ok_postgres
);
contract_case!(
    mark_succeeded_of_an_unknown_attempt_is_ok,
    mark_succeeded_of_an_unknown_attempt_is_ok_memory,
    mark_succeeded_of_an_unknown_attempt_is_ok_postgres
);
contract_case!(
    delete_for_login_gives_the_newest_100_of_250_and_the_count,
    delete_for_login_gives_the_newest_100_of_250_and_the_count_memory,
    delete_for_login_gives_the_newest_100_of_250_and_the_count_postgres
);
contract_case!(
    delete_for_login_with_keep_zero_still_gives_the_count,
    delete_for_login_with_keep_zero_still_gives_the_count_memory,
    delete_for_login_with_keep_zero_still_gives_the_count_postgres
);
contract_case!(
    throttle_state_counts_the_failures_over_every_address,
    throttle_state_counts_the_failures_over_every_address_memory,
    throttle_state_counts_the_failures_over_every_address_postgres
);
