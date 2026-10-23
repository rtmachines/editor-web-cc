//! Ports of the application layer (design D5).
//!
//! A port is a trait that a use case calls. `recipe-db` implements it with `sqlx`,
//! and the module `memory` implements it for tests.

mod attempt;
mod audit_reads;
mod clock;
mod diff_reads;
mod document_writer;
mod draft;
mod isolation;
mod nonempty;
mod plant;
mod recipe;
mod recipe_reads;
mod row_image;
mod security;
mod session;
mod types;
mod unit_of_work;
mod units;
mod user;

pub use attempt::{
    AttemptId, AttemptKey, AttemptOutcome, AttemptRepository, AttemptRow, DeletedAttempts,
    LoginAttemptStore, LoginThrottleState,
};
pub use audit_reads::{
    AggregateId, AuditEntityType, AuditEntryId, AuditFilter, AuditReads, AuditRecord, AuditRow,
    AuditScope, CursorPage, EntityFilter, EntityId, EntityLabel, LoginMatch, SecurityEventFilter,
    SecurityEventId, SecurityEventRow, TimeRange,
};
pub use clock::Clock;
pub use diff_reads::{ChangeFlags, DiffReads, DiffSides};
pub use document_writer::DocumentWriter;
pub use draft::DraftRepository;
pub use isolation::Isolation;
pub use nonempty::NonEmpty;
pub use plant::{InsertedMachine, MachineFilter, MachineSummary, PlantRepository};
pub use recipe::{InsertedRecipe, NewVersion, RecipeRepository, SharedVersion};
pub use recipe_reads::{
    ParameterRow, RecipeFilter, RecipeReads, RecipeSummary, UnitCount, VersionDetail,
    VersionSummary,
};
pub use row_image::{Locked, RowImage};
pub use security::SecurityEventSink;
pub use session::{NewSession, SessionLimits, SessionRepository, SessionRow, SessionStore};
pub use types::{DerivedTypeLock, Ensured, TypeCatalog, TypeUsageRow};
pub use unit_of_work::{AuditWritten, Transaction, UnitOfWork};
pub use units::UnitLookup;
pub use user::{NewUser, UserRepository, UserRow, UserSummary};
