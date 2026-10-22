//! Contract cases of `DiffReads` (ADR-0050 (e), (f), design D4).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. A case compares
//! the change flags with a table that a human wrote. It also compares them with the rows of the
//! domain diff `diff_pairs` on the pairs that `DiffReads::pairs` gives (ADR-0050, Risks).

mod fixtures;
mod order;
mod pairs;
mod parts;
mod sides;
mod snapshot;
mod terms;
