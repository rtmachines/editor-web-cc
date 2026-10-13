//! The fixture that bundles a store, a sink, and a clock, and seeds recipes.

use recipe_domain::plant::{Machine, PlcPlatform};
use recipe_domain::recipe::{Recipe, RecipeVersionId, VersionState};

use super::{instant, machine_on, recipe_with, reference, rid, text, user, vid};
use crate::memory::{MemoryClock, MemorySink, MemoryStore};

/// Return the number of the version `number` of recipe `n`, as the `u128` of a version id.
fn version_key(n: u128, number: u128) -> u128 {
    n.checked_mul(100).unwrap().checked_add(number).unwrap()
}

/// A store, a security event sink, and a clock that one test shares.
///
/// The store holds the primitive types. The store and the clock share the time.
pub(crate) struct Fixture {
    /// The in-memory store with the primitive types.
    pub store: MemoryStore,
    /// The security event sink of the test.
    pub sink: MemorySink,
    /// The clock of the store.
    pub clock: MemoryClock,
}

impl Fixture {
    /// Build an empty fixture with the primitive types and the clock at `NOW_SECONDS`.
    pub(crate) fn new() -> Self {
        let clock = MemoryClock::new(instant());
        let store = MemoryStore::with_clock(clock.clone());
        store.seed_primitive_types();
        Self {
            store,
            sink: MemorySink::default(),
            clock,
        }
    }

    /// Build the machine of [`machine_on`] and store it.
    pub(crate) fn seed_machine_on(&self, platform: PlcPlatform, n: u128, code: &str) -> Machine {
        let machine = machine_on(platform, n, code);
        self.store.seed_machine(&machine);
        machine
    }

    /// Store recipe `n` of `machine` with version 1 as a live draft created by `jkowalski`.
    pub(crate) fn seed_draft(
        &self,
        machine: &Machine,
        n: u128,
        reference_text: &str,
    ) -> (Recipe, RecipeVersionId) {
        let version = vid(version_key(n, 1));
        let recipe = Recipe::create(
            rid(n),
            machine.id(),
            reference(reference_text),
            text("Recipe"),
            version,
        );
        self.store.seed_recipe(&recipe, machine.platform(), user(2));
        (recipe, version)
    }

    /// Store recipe `n` of `machine` with version 1 approved and version 2 a live draft.
    ///
    /// The result holds the recipe, the approved version, and the draft.
    pub(crate) fn seed_approved_and_draft(
        &self,
        machine: &Machine,
        n: u128,
        reference_text: &str,
    ) -> (Recipe, RecipeVersionId, RecipeVersionId) {
        let versions = [
            (version_key(n, 1), VersionState::Approved),
            (version_key(n, 2), VersionState::LiveDraft),
        ];
        let recipe = self.seed_recipe_with(machine, n, reference_text, &versions);
        (recipe, vid(version_key(n, 1)), vid(version_key(n, 2)))
    }

    /// Store recipe `n` of `machine` with version 1 retired and version 2 approved.
    ///
    /// The result holds the recipe, the retired version, and the approved version.
    pub(crate) fn seed_retired(
        &self,
        machine: &Machine,
        n: u128,
        reference_text: &str,
    ) -> (Recipe, RecipeVersionId, RecipeVersionId) {
        let versions = [
            (version_key(n, 1), VersionState::Retired),
            (version_key(n, 2), VersionState::Approved),
        ];
        let recipe = self.seed_recipe_with(machine, n, reference_text, &versions);
        (recipe, vid(version_key(n, 1)), vid(version_key(n, 2)))
    }

    /// Store recipe `n` of `machine` with the given version states, created by `jkowalski`.
    fn seed_recipe_with(
        &self,
        machine: &Machine,
        n: u128,
        reference_text: &str,
        versions: &[(u128, VersionState)],
    ) -> Recipe {
        let machine_number = machine.id().as_uuid().as_u128();
        let recipe = recipe_with((n, machine_number), reference_text, versions);
        self.store.seed_recipe(&recipe, machine.platform(), user(2));
        recipe
    }
}
