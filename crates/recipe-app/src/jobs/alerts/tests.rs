//! Tests of the job `alerts` (task 3.8, design D6 and D14 row T14).
//!
//! `reliability.md` 3.7, ADR-0073 (a), (c), (f), (g), ADR-0075, ADR-0089 (d), ADR-0090 (a).
//! They run the job against stubs and in-memory adapters. A moved database time replaces every
//! real wait.

mod borders;
mod conditions;
mod disks;
mod during;
mod faults;
mod grace;
mod pairs;
mod panics;
mod settings;
mod skew;
mod slots;
mod support;
mod webhook;
