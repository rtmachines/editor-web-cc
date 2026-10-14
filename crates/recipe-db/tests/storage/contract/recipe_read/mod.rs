//! Contract cases of the reads and locks of `RecipeRepository` (task 9.5).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The cases that
//! need PostgreSQL carry the prefix `postgres_only`.

mod locks;
mod postgres_locks;
mod reads;
