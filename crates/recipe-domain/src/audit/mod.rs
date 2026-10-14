//! Audit context of `domain-model.md` section 2.4.
//!
//! The audit records are write-only. A command writes one `AuditEntry` in its
//! transaction, and a security check writes one `SecurityEvent`. This module
//! imports the identifier types of the other contexts and no entity.

mod action;
mod entry;
mod request;
mod security;
mod session_ref;
mod target;

pub use action::AuditAction;
pub use entry::{AuditContext, AuditEntry, DiffAction, RowAction, RowChange, StateAction};
pub use request::{RequestId, UserAgent};
pub use security::{LoginHmac, LoginSubject, SecurityEvent, SecurityEventKind};
pub use session_ref::{SessionRef, SessionRefReason};
pub use target::{AggregateType, AuditTarget};
