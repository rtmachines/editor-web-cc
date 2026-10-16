//! The contract suite (design D13).
//!
//! A case is an async function `fn(&dyn Harness)`. The macro `contract_case!` writes two tests
//! for it, one on `MemoryStore` and one on `Database`, with the same assertions. A difference
//! between the two adapters shows as one failing test of the pair.

/// Write the tests `$memory` and `$postgres` for the case function `$case`.
///
/// The names come from the caller, because `macro_rules!` cannot join identifiers.
macro_rules! contract_case {
    ($case:ident, $memory:ident, $postgres:ident) => {
        #[tokio::test]
        async fn $memory() {
            let harness = $crate::contract::MemoryHarness::create().await;
            $case(&harness).await;
        }

        #[tokio::test]
        async fn $postgres() {
            let harness = $crate::contract::PgHarness::create().await;
            $case(&harness).await;
        }
    };
}

mod attempts;
mod chain;
mod draft;
mod harness;
mod images;
mod locks;
mod machine;
mod machine_list;
mod memory_harness;
mod postgres_harness;
mod recipe_read;
mod recipe_reads;
mod recipe_seed;
mod recipe_write;
mod seed;
mod sessions;
mod type_images;
mod types;
mod unit;
mod users;

pub use memory_harness::MemoryHarness;
pub use postgres_harness::PgHarness;
