//! Port of the readiness check of `/readyz` (`reliability.md` section 7.2, ADR-0059).

use std::time::Duration;

use async_trait::async_trait;

/// Why the server is not ready.
///
/// The cause is for the log of the job and for the tests. `/readyz` never returns it to a client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotReadyCause {
    /// The database does not answer `SELECT 1`.
    Unreachable,
    /// The check did not finish within its timeout.
    TimedOut,
    /// A migration of the binary is missing, has another checksum, or failed (ADR-0059 (h)).
    SchemaMismatch,
}

/// The result of one readiness check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Readiness {
    /// The database answers and the schema matches the binary.
    Ready,
    /// The server is not ready.
    NotReady(NotReadyCause),
}

impl Readiness {
    /// Return true for [`Readiness::Ready`].
    #[must_use]
    pub const fn is_ready(self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// Port of the readiness check.
///
/// The port solves one problem: `recipe-web` must answer `/readyz` without a database type.
/// The adapter runs the check on the health pool, so a slow request pool does not hide a fault.
/// Any failure of the check is a [`Readiness::NotReady`], never an error.
#[async_trait]
pub trait ReadinessProbe: Send + Sync {
    /// Run the check and stop it after `timeout` (`health.db_check_timeout`).
    async fn check(&self, timeout: Duration) -> Readiness;
}
