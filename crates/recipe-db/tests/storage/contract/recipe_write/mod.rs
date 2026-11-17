//! Contract cases of the writes of `RecipeRepository` (task 9.6).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions.

mod guard;
mod recipes;
mod stale_record;
mod status;
mod versions;
