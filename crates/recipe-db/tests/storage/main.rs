// reliability.md section 2.3: test code may use `unwrap`, `expect`, and indexing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

//! Storage tests of `recipe-db` against a real PostgreSQL server (design D12).
//!
//! The binary starts one container for all tests. Each module covers one area.

mod atomic;
mod constraints;
mod contract;
mod errors;
mod grants;
mod hardening;
mod migrate;
mod pool;
mod race;
mod roles;
mod scenario;
mod schema;
mod security_event_guard;
mod security_events;
mod seed;
mod support;
mod triggers;
mod unit_of_work;
