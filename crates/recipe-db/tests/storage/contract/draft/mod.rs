//! Contract cases of `DraftRepository` (tasks 9.7 and 9.8).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The cases that
//! need PostgreSQL carry the prefix `postgres_only`.

mod fixtures;
mod insert;
mod narrow_writes;
mod postgres_cascade;
mod postgres_locks;
mod reads;
mod removals;
