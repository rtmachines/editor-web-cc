//! Port of the webhook of the alerts (`reliability.md` section 3.7, ADR-0063).

use std::fmt;

use async_trait::async_trait;

/// One alert condition of `reliability.md` section 3.7.
///
/// The type solves one problem: the log line, the webhook body, and the tests must name each
/// condition in the same way. [`AlertCondition::name`] is the value of the field `alert` and of
/// the key `alert` in the webhook body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlertCondition {
    /// More than `alerts.panic_count` panic events in `alerts.panic_window`.
    Panics,
    /// `/readyz` was not ready for more than `alerts.not_ready_for`.
    NotReady,
    /// The newest dump is older than `backup.max_age_alert`.
    BackupAge,
    /// The WAL receiver is not connected, has no known lag, or has a lag at or above
    /// `wal.max_lag_bytes` (ADR-0090 (a)).
    Replication,
    /// Free disk is below `disk.warn_percent`.
    LowDisk,
    /// More than `alerts.failed_login_count` failed logins in `alerts.failed_login_window`.
    FailedLogins,
    /// A `csrf_rejected` event, or a `forbidden` event on `UserManage`, in
    /// `alerts.security_window`.
    SecurityEvents,
    /// The audit chain check failed.
    AuditChainBroken,
    /// `audit.last_check_at` is older than `audit.max_check_age`.
    AuditCheckAge,
    /// One or more `E-EXP-002` results since the previous run (ADR-0093 (b)).
    ExportHashMismatch,
}

impl AlertCondition {
    /// Every condition, in the order of the table in `reliability.md` section 3.7.
    pub const ALL: [Self; 10] = [
        Self::Panics,
        Self::NotReady,
        Self::BackupAge,
        Self::Replication,
        Self::LowDisk,
        Self::FailedLogins,
        Self::SecurityEvents,
        Self::AuditChainBroken,
        Self::AuditCheckAge,
        Self::ExportHashMismatch,
    ];

    /// Return the stable name of the condition (ADR-0075 (a)).
    ///
    /// The name has lower-case ASCII letters and underscores only, and it starts with a letter. It never changes (ADR-0075 (b)).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Panics => "panics",
            Self::NotReady => "not_ready",
            Self::BackupAge => "backup_age",
            Self::Replication => "replication",
            Self::LowDisk => "low_disk",
            Self::FailedLogins => "failed_logins",
            Self::SecurityEvents => "security_events",
            Self::AuditChainBroken => "audit_chain_broken",
            Self::AuditCheckAge => "audit_check_age",
            Self::ExportHashMismatch => "export_hash_mismatch",
        }
    }
}

/// Why a call of the webhook failed.
///
/// This is a reason type. It owns no error code, like `ValueFaults` (ADR-0025, ADR-0060 (r)).
/// No variant holds a part of the webhook URL. The `Display` text and the `Debug` text hold
/// none either (ADR-0063 (g)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebhookFault {
    /// The connection to the host failed.
    Connection,
    /// The TLS handshake failed.
    Tls,
    /// The call did not finish within `alerts.webhook_timeout`.
    Timeout,
    /// The host answered with a status outside `2xx`.
    Status {
        /// The HTTP status code.
        code: u16,
    },
}

impl fmt::Display for WebhookFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection => f.write_str("The connection to the webhook host failed."),
            Self::Tls => f.write_str("The TLS handshake with the webhook host failed."),
            Self::Timeout => f.write_str("The webhook call took too long."),
            Self::Status { code } => {
                write!(f, "The webhook host answered with the status {code}.")
            }
        }
    }
}

/// Port of the webhook that receives the alerts.
///
/// The port solves one problem: the job `alerts` must send to a URL that it never sees. The
/// adapter in `recipe-server` holds the URL and the TLS client (ADR-0063). A test uses an
/// in-memory adapter.
#[async_trait]
pub trait AlertWebhook: Send + Sync {
    /// Send one `POST` for each condition. The calls of one run go out at the same time.
    ///
    /// The result has one entry for each condition, in the order of the argument.
    /// With no webhook configured, every entry is `Ok`.
    async fn send_all(&self, conditions: &[AlertCondition]) -> Vec<Result<(), WebhookFault>>;
}
