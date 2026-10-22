use super::support::*;
use super::*;

use VersionState::{Approved, Discarded, LiveDraft, Retired};

fn recipe_with(states: &[(u32, VersionState)]) -> Recipe {
    recipe(states.iter().map(|&(n, s)| header(n, s, 7)).collect())
}

#[test]
fn approved_version_gives_the_header_in_the_state_approved() {
    let recipe = recipe_with(&[(1, Retired), (2, Approved), (3, LiveDraft)]);
    let header = recipe.approved_version().unwrap();
    assert_eq!(header.number(), 2);
    assert_eq!(header.id(), vid(2));
    assert_eq!(header.state(), Approved);
}

#[test]
fn approved_version_gives_none_for_a_draft_and_retired_versions_only() {
    let recipe = recipe_with(&[(1, Retired), (2, Discarded), (3, Retired), (4, LiveDraft)]);
    assert!(recipe.approved_version().is_none());
}

#[test]
fn approved_version_gives_none_for_a_recipe_without_a_version() {
    let recipe = recipe_with(&[]);
    assert!(recipe.approved_version().is_none());
}

#[test]
fn approved_version_gives_none_for_a_deleted_recipe_with_retired_versions_only() {
    let recipe = deleted_recipe(vec![header(1, Retired, 7), header(2, Retired, 7)]);
    assert!(recipe.is_deleted());
    assert!(recipe.approved_version().is_none());
}
