//! Port of the database reads of the jobs `alerts` and `clock_check`, and of step 5 of the start
//! (`reliability.md` section 3.7, ADR-0060 (e), ADR-0061).

use std::time::Duration;

use async_trait::async_trait;
use recipe_domain::timestamp::Timestamp;

use super::TimeStateKey;
use crate::error::StorageError;

/// The role that the application must use (`reliability.md` section 5).
///
/// The role name is fixed, because `01-roles.sh` creates it and the migrations grant to it.
/// A `db.app_user` with another value therefore fails this check.
pub(crate) const APPLICATION_ROLE: &str = "recipe_app";

/// The windows of the two sums that count security events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotWindows {
    /// The window of the failed logins (`alerts.failed_login_window`).
    pub failed_logins: Duration,
    /// The window of `csrf_rejected` and `forbidden` on `UserManage` (`alerts.security_window`).
    pub security: Duration,
}

/// The state of the WAL receiver `walarchive` (`replication_state()`, `data-model.md` section 5.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplicationState {
    /// True when a row of `recipe_replicator` with `application_name = 'walarchive'` exists.
    pub connected: bool,
    /// The lag in bytes. `None` with `connected` false means that no row exists.
    /// `None` with `connected` true means a row without `sent_lsn` (ADR-0090 (a)).
    pub lag_bytes: Option<u64>,
}

/// The facts that the role check reads about the connection (`reliability.md` section 5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleCheck {
    /// The value of `current_user`.
    pub current_user: String,
    /// True when the role is a superuser.
    pub is_superuser: bool,
    /// True when the role owns a table.
    pub owns_relations: bool,
}

impl RoleCheck {
    /// Return true when the connection runs as `recipe_app`, as no superuser, and owns no table.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        self.current_user == APPLICATION_ROLE && !self.is_superuser && !self.owns_relations
    }
}

/// Every database value of one run of the job `alerts`.
///
/// The adapter reads all values in one `READ ONLY` transaction, so they describe one instant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationsSnapshot {
    /// The time of the database (`now()`).
    pub database_now: Timestamp,
    /// The state of the WAL receiver.
    pub replication: ReplicationState,
    /// The sum of `count` of the `login_failed` events in the window.
    pub failed_logins: u64,
    /// The number of `csrf_rejected` events and `forbidden` events on `UserManage` in the window.
    pub security_alerts: u64,
    /// The newest `occurred_at` of a `security_event` row of kind `audit_chain_broken`.
    ///
    /// The value is `None` when no such row exists. It serves the condition "Audit chain check
    /// failed" of `reliability.md` section 3.7. ADR-0073 (a) gives the rule.
    pub audit_chain_broken_at: Option<Timestamp>,
    /// The state key `audit.last_check_at`, or `None` when no check completed.
    pub audit_last_check_at: Option<Timestamp>,
    /// The oldest `created_at` of `app_setting`: the first start of the installation.
    pub first_setting_at: Option<Timestamp>,
}

/// Port of the reads of the jobs `alerts` and `clock_check`. It also serves the catch-up of
/// `audit_chain_check` and the role check of step 5 of the start (ADR-0061).
///
/// The port solves one problem: the job needs several values that agree with each other, and
/// `recipe-app` must not hold a `sqlx` type. The adapter parses each stored time into a
/// [`Timestamp`], so `recipe-app` keeps its closed list of dependencies (ADR-0035 (g)).
#[async_trait]
pub trait OperationsReads: Send + Sync {
    /// Read every value of one run of the job `alerts` in one `READ ONLY` transaction.
    async fn snapshot(&self, windows: &SnapshotWindows)
    -> Result<OperationsSnapshot, StorageError>;

    /// Read the time of the database with one `SELECT now()` for the job `clock_check`.
    ///
    /// `reliability.md` section 7.9 describes the job.
    async fn database_now(&self) -> Result<Timestamp, StorageError>;

    /// Read the facts of the role check of step 5 of the start (ADR-0061, `reliability.md`
    /// section 5).
    async fn role_check(&self) -> Result<RoleCheck, StorageError>;

    /// Read one state key of `app_setting` as a time, or `None` when the row does not exist.
    ///
    /// A stored text in another form than ADR-0073 (i) gives a `StorageError`.
    async fn state(&self, key: TimeStateKey) -> Result<Option<Timestamp>, StorageError>;
}
