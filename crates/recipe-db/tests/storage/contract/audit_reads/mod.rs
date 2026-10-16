//! Contract cases of `AuditReads` (ADR-0051 (a), design D5).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The ids of the
//! entries differ between the adapters, so a case compares the order of the rows, not the ids.

mod aggregate_label;
mod entry;
mod events;
mod fixtures;
mod history;
mod labels;
mod list;
