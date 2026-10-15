//! Contract cases of `UnitLookup` and `TypeCatalog` (task 9.4).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The cases that
//! need PostgreSQL carry the prefix `postgres_only`.

mod advisory;
mod derived;
mod lookup;
mod rows;
mod structs;
mod usage;
