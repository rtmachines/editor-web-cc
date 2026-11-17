//! Contract cases of the writes of `RecipeRepository` (task 9.6).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions.

mod guard;
mod read_only_locks;
mod recipes;
mod refresh_after_lock;
mod stale_approve;
mod stale_record;
mod status;
mod versions;
