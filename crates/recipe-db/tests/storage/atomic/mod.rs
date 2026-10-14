//! Use cases on the database (tasks 10.1 to 10.3, design D14).
//!
//! Each command has a success test and a midway failure test. A test trigger that the superuser of
//! the harness creates makes one statement of the command fail. The failed command leaves no row
//! and does not move the head of the audit chain.

mod draft;
mod identity;
mod plant;
mod recipe;
pub mod world;
