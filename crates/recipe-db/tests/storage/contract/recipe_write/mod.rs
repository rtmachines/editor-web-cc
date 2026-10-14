//! Contract cases of the writes of `RecipeRepository` (task 9.6).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The cases that
//! need PostgreSQL carry the prefix `postgres_only`.

mod guard;
mod recipes;
mod status;
mod versions;
