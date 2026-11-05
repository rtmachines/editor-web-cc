use std::time::Duration;

use super::{pool_options, release_verdict};
use crate::settings::PoolSettings;

#[test]
fn a_connection_without_an_open_transaction_is_kept() {
    assert!(matches!(release_verdict(Ok(false)), Ok(true)));
}

#[test]
fn a_connection_with_an_open_transaction_is_closed_gracefully() {
    assert!(matches!(release_verdict(Ok(true)), Ok(false)));
}

#[test]
fn a_connection_whose_release_statement_fails_is_closed_hard() {
    let verdict = release_verdict(Err(sqlx::Error::PoolClosed));

    assert!(matches!(verdict, Err(sqlx::Error::PoolClosed)));
}

#[test]
fn the_pool_takes_wait_warn_as_the_slow_acquire_threshold() {
    let settings = PoolSettings {
        wait_warn: Duration::from_millis(1234),
        ..PoolSettings::default()
    };

    let options = pool_options(&settings);

    assert_eq!(
        options.get_acquire_slow_threshold(),
        Duration::from_millis(1234)
    );
}
