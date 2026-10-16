//! Contract cases of `RecipeReads` (design D5, ADR-0042).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The case that
//! needs PostgreSQL carries the prefix `postgres_only`.

mod fixtures;
mod parameter_reads;
mod recipe_list;
mod repeatable_read;
mod version_reads;
