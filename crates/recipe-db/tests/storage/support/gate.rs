//! Deterministic races of two commands (design D14).
//!
//! A race test holds the first command inside its transaction at a known point. A test trigger
//! of the superuser asks for a shared advisory lock at that point, and the test holds the same
//! key exclusively. The first command therefore waits at the gate with all its row locks. The
//! test then starts the second command and polls `pg_stat_activity` until the second command
//! waits for a lock that the first command holds. Only then the test opens the gate. No step
//! depends on a sleep or on wall-clock luck. The polls have an overall bound, so a bug fails the
//! test and never hangs it.

use std::fmt::Debug;
use std::future::Future;
use std::time::{Duration, Instant};

use recipe_app::error::AppError;
use recipe_db::settings::PoolSettings;
use recipe_domain::error_codes::ErrorCode;

use super::app::App;

/// The key of the gate. The one-key advisory lock never meets the two-key locks of the adapter.
const GATE_KEY: i64 = 7_240_200;
/// The pause between two polls of `pg_stat_activity`.
const POLL: Duration = Duration::from_millis(20);
/// The longest time that a poll waits for its condition.
const BOUND: Duration = Duration::from_secs(20);
/// The `lock_timeout` of the pool of a race test. A correct wait must never turn into `E-DB-001`.
const RACE_LOCK_TIMEOUT: Duration = Duration::from_secs(15);

/// The first command sits at the gate: a session waits for the advisory lock.
const AT_GATE: &str = "SELECT count(*) FROM pg_stat_activity \
    WHERE datname = current_database() \
      AND wait_event_type = 'Lock' AND wait_event = 'advisory'";

/// A session waits for a row lock, and a session at the gate blocks it.
const BLOCKED_BY_GATE: &str = "SELECT count(*) FROM pg_stat_activity AS w \
    WHERE w.datname = current_database() \
      AND w.wait_event_type = 'Lock' AND w.wait_event <> 'advisory' \
      AND EXISTS (SELECT 1 FROM unnest(pg_blocking_pids(w.pid)) AS b(pid) \
                  JOIN pg_stat_activity AS g ON g.pid = b.pid \
                  WHERE g.wait_event_type = 'Lock' AND g.wait_event = 'advisory')";

/// The settings of the pool of a race test: the default values with a long `lock_timeout`.
pub fn pool() -> PoolSettings {
    PoolSettings {
        lock_timeout: RACE_LOCK_TIMEOUT,
        ..PoolSettings::default()
    }
}

impl App {
    /// Create a gate on a table: a row trigger that waits for the gate when `condition` holds.
    ///
    /// `condition` is PL/pgSQL text over `NEW`. The caller passes a constant.
    pub async fn install_gate(&mut self, table: &str, events: &str, condition: &str) {
        let body = format!(
            "IF {condition} THEN PERFORM pg_catalog.pg_advisory_xact_lock_shared({GATE_KEY}); END IF;"
        );
        self.install_probe(table, "BEFORE", events, &body).await;
    }

    /// Close the gate. A command that reaches the gate waits until `open_gate`.
    pub async fn close_gate(&mut self) {
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(GATE_KEY)
            .execute(self.admin())
            .await
            .unwrap();
    }

    /// Open the gate. The waiting command continues.
    pub async fn open_gate(&mut self) {
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(GATE_KEY)
            .execute(self.admin())
            .await
            .unwrap();
    }

    /// Poll until a session waits at the gate.
    pub async fn wait_until_at_gate(&mut self) {
        self.poll(AT_GATE, "a command waits at the gate").await;
    }

    /// Poll until a session waits for a row lock that a session at the gate holds.
    pub async fn wait_until_blocked_by_gate(&mut self) {
        self.poll(BLOCKED_BY_GATE, "the second command waits for the first")
            .await;
    }

    async fn poll(&mut self, query: &str, what: &str) {
        let start = Instant::now();
        loop {
            let waiting: i64 = sqlx::query_scalar(query)
                .fetch_one(self.admin())
                .await
                .unwrap();
            if waiting > 0 {
                return;
            }
            assert!(start.elapsed() < BOUND, "timeout: {what}");
            tokio::time::sleep(POLL).await;
        }
    }
}

/// Run two commands as a deterministic race and return both results.
///
/// The caller installed a gate on a statement of `first`. The function closes the gate, starts
/// `first`, and waits until `first` sits at the gate. Then it starts `second` and waits until
/// `second` waits for a lock of `first`. Then it opens the gate.
pub async fn run<A, B>(
    app: &mut App,
    first: impl Future<Output = A> + Send + 'static,
    second: impl Future<Output = B> + Send + 'static,
) -> (A, B)
where
    A: Send + 'static,
    B: Send + 'static,
{
    app.close_gate().await;
    let first = tokio::spawn(first);
    app.wait_until_at_gate().await;
    let second = tokio::spawn(second);
    app.wait_until_blocked_by_gate().await;
    app.open_gate().await;
    (first.await.unwrap(), second.await.unwrap())
}

/// The code of the error of a result. The call fails the test for an `Ok`.
pub fn error_code<T: Debug>(result: &Result<T, AppError>) -> ErrorCode {
    result.as_ref().unwrap_err().code()
}

/// Assert that a result is not a deadlock or a lock timeout (`E-DB-001`).
pub fn assert_no_deadlock<T: Debug>(result: &Result<T, AppError>) {
    if let Err(error) = result {
        assert_ne!(error.code(), ErrorCode::DB_001, "{error:?}");
    }
}
