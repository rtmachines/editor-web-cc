//! Races of two commands on two connections (tasks 10.4 and 10.5, design D14).
//!
//! Each race holds the first command at a gate and proves with `pg_stat_activity` that the
//! second command waits for it. No test depends on a sleep.

mod approve;
mod order;
