//! Tests of the job `audit_chain_check` (task 3.6, design D6 and D14 row T14, ADR-0086, ADR-0088).
//!
//! They run the job against the in-memory adapters. A scripted reader gives the cases that the
//! memory chain cannot reach.

mod anchor;
mod batches;
mod breaks;
mod catch_up;
mod end_check;
mod faults;
mod full_chain;
mod head_rules;
mod settings;
mod support;
