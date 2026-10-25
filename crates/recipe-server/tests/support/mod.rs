//! Helpers that the tests which run the built binary share: `migrate`, `create_admin`, and `serve`.
//!
//! Every item of these modules is used by every test crate that includes them. A crate that
//! needs less than this includes none of them. The helpers that only `serve.rs` uses live in
//! `process.rs` and `net.rs`, which that file includes by `#[path]`.

pub mod cli;
pub mod database;
pub mod temp;
