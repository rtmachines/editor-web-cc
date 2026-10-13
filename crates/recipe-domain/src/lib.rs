//! Domain model of the recipe editor.
//!
//! This crate holds the entities, the value objects, the invariants, the
//! validation rules, the `Permission` enum, and the error codes.
//! The module `export` builds the hashed export body and computes the content hash.
//! The crate does no I/O and defines no port trait.
//!
//! Each bounded context has one error enum: `PlantError`, `RecipeError`,
//! `IdentityError`, and `ExportError`.
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/domain-model.md`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod audit;
pub mod error_codes;
pub mod export;
pub mod identity;
pub mod plant;
pub mod recipe;
pub mod text;
pub mod timestamp;
