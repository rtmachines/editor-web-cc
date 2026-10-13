//! Ports of the application layer (design D5).
//!
//! A port is a trait that a use case calls. `recipe-db` implements it with `sqlx`,
//! and the module `memory` implements it for tests.

mod clock;
mod draft;
mod isolation;
mod nonempty;
mod plant;
mod recipe;
mod row_image;
mod security;
mod types;
mod unit_of_work;
mod units;

pub use clock::Clock;
pub use draft::DraftRepository;
pub use isolation::Isolation;
pub use nonempty::NonEmpty;
pub use plant::{InsertedMachine, PlantRepository};
pub use recipe::{InsertedRecipe, NewVersion, RecipeRepository, SharedVersion};
pub use row_image::{Locked, RowImage};
pub use security::SecurityEventSink;
pub use types::{DerivedTypeLock, Ensured, TypeCatalog};
pub use unit_of_work::{AuditWritten, Transaction, UnitOfWork};
pub use units::UnitLookup;
