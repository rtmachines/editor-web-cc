//! The full life of one recipe through the use cases on PostgreSQL (task 8.1).
//!
//! Design D10, row "Full scenario". The unit tests and the contract tests check each command
//! alone. This test runs the commands one after the other on one database and one audit chain.

mod chain;
mod life;
mod scene;
