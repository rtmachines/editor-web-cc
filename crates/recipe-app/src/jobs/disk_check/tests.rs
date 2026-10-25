//! Tests of the job `disk_check` (task 3.5, design D6 and D14 row T14, ADR-0073 (d) and (e)).
//!
//! They run the job against the in-memory adapters. A moved clock replaces every real wait.

mod faults;
mod support;
mod thresholds;
mod warnings;
