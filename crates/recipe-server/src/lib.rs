//! Library of the recipe editor binary.
//!
//! This crate loads the configuration and connects the adapters to the use cases.
//! It holds every module of the binary, so a test can import a module (ADR-0060 (bb)).
//! The binary `recipe-server` parses the arguments and calls this library.
//! The crate is not published. Only the binary and its tests use the library.
//! It is the only crate that uses `anyhow` (Assumption A37).
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/reliability.md` section 2.

#![forbid(unsafe_code)]

pub mod backups;
pub mod clock;
pub mod coded;
pub mod config;
pub mod create_admin;
pub mod disk;
pub mod listener;
pub mod logging;
pub mod migrate;
pub mod panic_hook;
pub mod pg_env;
pub mod port;
pub mod probe;
pub mod schedule;
pub mod serve;
pub mod supervisor;
pub mod watchdog;
pub mod webhook;
