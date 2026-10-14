//! Control of the minute of the database clock for tests of per-minute aggregation.

use std::time::Duration;

use sqlx::PgConnection;

/// The latest second of a minute at which a test starts a group of events that share a minute.
const LATEST_START_SECOND: f64 = 50.0;
const POLL_PAUSE: Duration = Duration::from_millis(500);

/// Wait until the second of the database clock is below 50.
///
/// A test that writes several events in a loop calls this first, so that the loop ends before the
/// minute changes and every event falls into the same aggregation minute.
pub async fn wait_for_early_second(conn: &mut PgConnection) {
    loop {
        let second: f64 =
            sqlx::query_scalar("SELECT extract(second FROM clock_timestamp())::float8")
                .fetch_one(&mut *conn)
                .await
                .unwrap();
        if second < LATEST_START_SECOND {
            return;
        }
        tokio::time::sleep(POLL_PAUSE).await;
    }
}
