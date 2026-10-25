//! Tests of the job `clock_check` (task 3.7, design D6 and D14 row T14, `reliability.md` 7.9).
//!
//! They run the job against the in-memory adapters. Two clocks replace a real skew.

mod extremes;
mod faults;
mod skew;
mod support;
