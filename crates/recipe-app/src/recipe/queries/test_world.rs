//! Helpers of the tests of the recipe queries.

use recipe_domain::identity::{Login, PasswordHash, Role};
use recipe_domain::plant::{Machine, PlcPlatform};
use recipe_domain::recipe::{Recipe, RecipeDeleter, VersionState};

use crate::memory::fixtures::{
    Fixture, mid, recipe_with, reference, rid, text, timestamp, user, vid,
};
use crate::ports::NewUser;

/// A valid PHC string for a seeded user.
const HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0$ZG91YmxlZG91YmxlZG91YmxlZG91Ymxl";

/// Store the four fixed users of the tests: `admin`, `jkowalski`, `anowak`, and `mhoffman`.
///
/// The identifiers are the ones of `actor` and `second_engineer`. The seeded versions name
/// `jkowalski` (user 2) as their creator, so `GetVersion` finds a login for them.
pub(super) fn seed_users(fixture: &Fixture) {
    let users = [
        (1, "admin", Role::Administrator),
        (2, "jkowalski", Role::ProcessEngineer),
        (3, "anowak", Role::ProcessEngineer),
        (4, "mhoffman", Role::Operator),
    ];
    for (n, login, role) in users {
        fixture.store.seed_user(&NewUser {
            id: user(n),
            login: Login::parse(login).unwrap(),
            password_hash: PasswordHash::parse(HASH).unwrap(),
            role,
            must_change_password: false,
        });
    }
}

/// Store the deleted recipe `n` on the machine M1 with one discarded draft.
///
/// The version identifier is `n * 100 + 1`. The creator is `jkowalski`.
pub(super) fn seed_deleted(fixture: &Fixture, n: u128, reference_text: &str) {
    let mut recipe = Recipe::create(
        rid(n),
        mid(1),
        reference(reference_text),
        text("Deleted"),
        vid(n.checked_mul(100).unwrap().checked_add(1).unwrap()),
    );
    recipe
        .delete(RecipeDeleter::Administrator, timestamp())
        .unwrap();
    fixture
        .store
        .seed_recipe(&recipe, PlcPlatform::TwinCat3, user(2));
}

/// Store recipe `n` of `machine` with the given reference number and version states.
///
/// The version identifiers are `n * 100 + number`. The creator is `jkowalski`.
pub(super) fn seed_recipe(
    fixture: &Fixture,
    machine: &Machine,
    (n, reference): (u128, &str),
    states: &[VersionState],
) -> Recipe {
    let versions: Vec<(u128, VersionState)> = (1_u128..)
        .zip(states)
        .map(|(number, state)| {
            (
                n.checked_mul(100).unwrap().checked_add(number).unwrap(),
                *state,
            )
        })
        .collect();
    let machine_number = machine.id().as_uuid().as_u128();
    let recipe = recipe_with((n, machine_number), reference, &versions);
    fixture
        .store
        .seed_recipe(&recipe, machine.platform(), user(2));
    recipe
}
