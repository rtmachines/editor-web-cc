//! Tests of the job `cleanup` (task 3.4, design D6 and D14 row T14, ADR-0088 (c)).
//!
//! They run the job against the in-memory adapters. A moved clock replaces every real wait.

mod batches;
mod cadence;
mod faults;
mod recorded_instant;
mod support;
