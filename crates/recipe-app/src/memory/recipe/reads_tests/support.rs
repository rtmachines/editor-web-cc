//! Helpers of the tests of the in-memory `RecipeReads`.

use recipe_domain::identity::{Login, PasswordHash, Role};
use recipe_domain::plant::MachineCode;
use recipe_domain::recipe::{Recipe, StoredRecipe, StoredVersionHeader, VersionState};
use recipe_domain::timestamp::Timestamp;

use crate::memory::fixtures::{mid, reference, rid, text, user, vid};
use crate::memory::{MemoryStore, MemoryTransaction};
use crate::ports::{Isolation, NewUser};

/// A valid PHC string for a seeded user.
const HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0$ZG91YmxlZG91YmxlZG91YmxlZG91Ymxl";

/// Open a transaction with `isolation` on `store`.
pub(super) fn begin(store: &MemoryStore, isolation: Isolation) -> MemoryTransaction {
    store.begin_memory(isolation).unwrap()
}

/// Return the machine code of a test text.
pub(super) fn code(value: &str) -> MachineCode {
    MachineCode::parse(value).unwrap()
}

/// Store the active user `n` with the login `login_text`.
pub(super) fn seed_user(store: &MemoryStore, n: u128, login_text: &str) {
    store.seed_user(&NewUser {
        id: user(n),
        login: Login::parse(login_text).unwrap(),
        password_hash: PasswordHash::parse(HASH).unwrap(),
        role: Role::ProcessEngineer,
        must_change_password: false,
    });
}

/// The values of a seeded recipe that a test chooses.
pub(super) struct Seed<'a> {
    pub(super) recipe: u128,
    pub(super) machine: u128,
    pub(super) reference: &'a str,
    pub(super) name: &'a str,
    pub(super) deleted_at: Option<Timestamp>,
    pub(super) versions: &'a [(u128, VersionState)],
}

/// Build a recipe whose version headers have the numbers 1 to the length of the list.
pub(super) fn recipe_of(seed: &Seed<'_>) -> Recipe {
    let headers = (1_u32..)
        .zip(seed.versions)
        .map(|(number, (v, state))| StoredVersionHeader {
            id: vid(*v),
            number,
            state: *state,
            row_version: 1,
        })
        .collect();
    Recipe::try_from(StoredRecipe {
        id: rid(seed.recipe),
        machine: mid(seed.machine),
        reference: reference(seed.reference),
        name: text(seed.name),
        deleted_at: seed.deleted_at,
        versions: headers,
    })
    .unwrap()
}

/// Store the recipe of `seed` on a `TwinCat3` machine. The creator is the user 9.
pub(super) fn seed_recipe(store: &MemoryStore, seed: &Seed<'_>) {
    store.seed_recipe(
        &recipe_of(seed),
        recipe_domain::plant::PlcPlatform::TwinCat3,
        user(9),
    );
}

/// Store a live recipe of the machine M1 with the given reference, name, and versions.
pub(super) fn seed_live(
    store: &MemoryStore,
    (recipe, reference_text, name): (u128, &str, &str),
    versions: &[(u128, VersionState)],
) {
    seed_recipe(
        store,
        &Seed {
            recipe,
            machine: 1,
            reference: reference_text,
            name,
            deleted_at: None,
            versions,
        },
    );
}
