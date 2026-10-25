//! Ports of the application layer (design D5).
//!
//! A port is a trait that a use case calls. `recipe-db` implements it with `sqlx`,
//! and the module `memory` implements it for tests.

mod attempt;
mod audit_chain;
mod audit_reads;
mod backup_directory;
mod clock;
mod diff_reads;
mod disk;
mod document_writer;
mod draft;
mod isolation;
mod maintenance;
mod nonempty;
#[cfg(test)]
mod object_tests;
mod operations_reads;
mod plant;
mod read_only;
mod readiness;
mod recipe;
mod recipe_reads;
mod row_image;
mod security;
mod session;
mod setting;
mod state_key;
mod types;
mod unit_of_work;
mod units;
mod user;
mod webhook;
#[cfg(test)]
mod webhook_tests;

pub use attempt::{
    AttemptId, AttemptKey, AttemptOutcome, AttemptRepository, AttemptRow, DeletedAttempts,
    LoginAttemptStore, LoginThrottleState,
};
pub use audit_chain::{
    AuditChainReads, BatchOutcome, ChainBounds, ChainHead, ChainPosition, ChainSeq,
};
pub use audit_reads::{
    AggregateId, AuditEntityType, AuditEntryId, AuditFilter, AuditReads, AuditRecord, AuditRow,
    AuditScope, CursorPage, EntityFilter, EntityId, EntityLabel, LoginMatch, SecurityEventFilter,
    SecurityEventId, SecurityEventRow, TimeRange,
};
pub use backup_directory::{BackupDirectory, BackupFault};
pub use clock::Clock;
pub use diff_reads::{ChangeFlags, DiffReads, DiffSides};
pub use disk::{DiskFault, DiskSpace, FreeSpace};
pub use document_writer::DocumentWriter;
pub use draft::DraftRepository;
pub use isolation::Isolation;
pub use maintenance::MaintenanceStore;
pub use nonempty::NonEmpty;
pub use operations_reads::{
    OperationsReads, OperationsSnapshot, ReplicationState, RoleCheck, SnapshotWindows,
};
pub use plant::{InsertedMachine, MachineFilter, MachineSummary, PlantRepository};
pub use read_only::ReadOnlySwitch;
pub use readiness::{NotReadyCause, Readiness, ReadinessProbe};
pub use recipe::{InsertedRecipe, NewVersion, RecipeRepository, SharedVersion};
pub use recipe_reads::{
    ParameterRow, RecipeFilter, RecipeReads, RecipeSummary, UnitCount, VersionDetail,
    VersionSummary,
};
pub use row_image::{Locked, RowImage};
pub use security::SecurityEventSink;
pub use session::{NewSession, SessionLimits, SessionRepository, SessionRow, SessionStore};
pub use setting::{STATE_SETTING_KEYS, SettingRepository, SettingRow, SettingWritten};
pub use state_key::{StateValue, TimeStateKey, parse_chain_seq_text};
pub use types::{DerivedTypeLock, Ensured, TypeCatalog, TypeUsageRow};
pub use unit_of_work::{AuditWritten, Transaction, UnitOfWork};
pub use units::UnitLookup;
pub use user::{NewUser, UserRepository, UserRow, UserSummary};
pub use webhook::{AlertCondition, AlertWebhook, WebhookFault};
