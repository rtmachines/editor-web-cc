//! Contract cases of `version_page`, `version_detail`, and `last_change` (design D5).

use recipe_app::ports::VersionSummary;
use recipe_domain::identity::Login;
use recipe_domain::recipe::{RecipeId, RecipeVersionId, VersionState};
use recipe_domain::text::LongText;

use super::fixtures::{
    approve_as, begin_read, create_named, discard, far_past, second_user, touch_by,
};
use crate::contract::harness::{FIXTURE_NOTE, Harness};
use crate::contract::recipe_seed::{approve, create_version, plant, retire, version_id};

/// The login of the user of every harness.
const HARNESS_LOGIN: &str = "harness-admin";

/// Parse a login for an assertion.
fn login(text: &str) -> Login {
    Login::parse(text).unwrap()
}

/// Read one page of the versions in a transaction that nothing commits.
async fn page(
    h: &dyn Harness,
    recipe: RecipeId,
    (offset, limit): (u64, u32),
) -> Vec<VersionSummary> {
    let mut tx = begin_read(h).await;
    tx.recipe_reads()
        .version_page(recipe, offset, limit)
        .await
        .unwrap()
}

/// Return the numbers and the states of the rows, in the order of the rows.
fn numbers_and_states(rows: &[VersionSummary]) -> Vec<(u32, VersionState)> {
    rows.iter().map(|row| (row.number, row.state)).collect()
}

/// Read the login of the last change of `recipe` at `at`, in a transaction that nothing commits.
async fn last_change(
    h: &dyn Harness,
    recipe: RecipeId,
    at: recipe_domain::timestamp::Timestamp,
) -> Option<Login> {
    let mut tx = begin_read(h).await;
    tx.recipe_reads().last_change(recipe, at).await.unwrap()
}

/// Read the detail of `version` in a transaction that nothing commits.
async fn detail(
    h: &dyn Harness,
    version: RecipeVersionId,
) -> Option<recipe_app::ports::VersionDetail> {
    let mut tx = begin_read(h).await;
    tx.recipe_reads().version_detail(version).await.unwrap()
}

async fn version_page_gives_the_newest_number_first_and_cuts_the_page(h: &dyn Harness) {
    let plant = plant(h, 20, "M20").await;
    let recipe = create_named(h, &plant, 20, "REF-20", "Recipe").await;
    let first = recipe.versions()[0].id();
    approve(h, first).await;
    retire(h, first).await;
    create_version(h, (recipe.id(), version_id(120)), 2, None).await;
    approve(h, version_id(120)).await;
    create_version(h, (recipe.id(), version_id(220)), 3, None).await;

    let newest = page(h, recipe.id(), (0, 2)).await;
    let tail = page(h, recipe.id(), (2, 10)).await;
    let beyond = page(h, recipe.id(), (3, 10)).await;

    assert_eq!(
        numbers_and_states(&newest),
        [(3, VersionState::LiveDraft), (2, VersionState::Approved)]
    );
    assert_eq!(numbers_and_states(&tail), [(1, VersionState::Retired)]);
    assert!(beyond.is_empty(), "{beyond:?}");
    assert!(newest[0].approved_at.is_none());
    assert!(newest[1].approved_at.is_some());
    assert_eq!(
        newest[0].change_note,
        LongText::parse(FIXTURE_NOTE).unwrap()
    );
}

async fn version_detail_gives_the_logins_of_the_creator_and_the_approver(h: &dyn Harness) {
    let plant = plant(h, 21, "M21").await;
    let recipe = create_named(h, &plant, 21, "REF-21", "Recipe").await;
    let version = recipe.versions()[0].id();
    let approver = second_user(h, "approver").await;
    approve_as(h, version, approver).await;

    let read = detail(h, version).await.unwrap();

    assert_eq!(read.id, version);
    assert_eq!(read.recipe, recipe.id());
    assert_eq!(read.number, 1);
    assert_eq!(read.state, VersionState::Approved);
    assert_eq!(read.change_note, LongText::parse(FIXTURE_NOTE).unwrap());
    assert_eq!(read.created_by, login(HARNESS_LOGIN));
    assert_eq!(read.approved_by, Some(login("approver")));
    assert!(read.approved_at.is_some());
    assert!(read.created_at <= read.updated_at);
    assert_eq!(read.row_version, 2);
}

async fn version_detail_of_a_draft_has_no_approver_and_a_missing_version_gives_none(
    h: &dyn Harness,
) {
    let plant = plant(h, 22, "M22").await;
    let recipe = create_named(h, &plant, 22, "REF-22", "Recipe").await;

    let draft = detail(h, recipe.versions()[0].id()).await.unwrap();
    let missing = detail(h, version_id(99)).await;

    assert_eq!(draft.state, VersionState::LiveDraft);
    assert_eq!(draft.approved_by, None);
    assert_eq!(draft.approved_at, None);
    assert_eq!(draft.row_version, 1);
    assert!(missing.is_none());
}

async fn version_detail_of_a_version_without_a_base_has_no_base_number(h: &dyn Harness) {
    let plant = plant(h, 25, "M25").await;
    let recipe = create_named(h, &plant, 25, "REF-25", "Recipe").await;
    let first = recipe.versions()[0].id();
    approve(h, first).await;
    create_version(h, (recipe.id(), version_id(125)), 2, None).await;

    let created = detail(h, first).await.unwrap();
    let empty = detail(h, version_id(125)).await.unwrap();

    assert_eq!(created.based_on, None);
    assert_eq!(empty.based_on, None);
}

// The version 4 is based on the version 1, so a rule "number minus 1" gives 3 and fails.
async fn version_detail_gives_the_number_of_the_base_version(h: &dyn Harness) {
    let plant = plant(h, 26, "M26").await;
    let recipe = create_named(h, &plant, 26, "REF-26", "Recipe").await;
    let first = recipe.versions()[0].id();
    approve(h, first).await;
    retire(h, first).await;
    create_version(h, (recipe.id(), version_id(126)), 2, Some(first)).await;
    approve(h, version_id(126)).await;
    retire(h, version_id(126)).await;
    create_version(h, (recipe.id(), version_id(226)), 3, Some(version_id(126))).await;
    approve(h, version_id(226)).await;
    create_version(h, (recipe.id(), version_id(326)), 4, Some(first)).await;

    let second = detail(h, version_id(126)).await.unwrap();
    let third = detail(h, version_id(226)).await.unwrap();
    let fourth = detail(h, version_id(326)).await.unwrap();

    assert_eq!(second.based_on, Some(1));
    assert_eq!(third.based_on, Some(2));
    assert_eq!(fourth.based_on, Some(1));
}

// The version 2 is a discarded draft and the version 4 names it as its base. The number 3 would
// follow from the rule "number minus 1", and the number 1 from the rule "newest version that is
// not discarded".
async fn version_detail_gives_the_number_of_a_base_version_that_is_a_discarded_draft(
    h: &dyn Harness,
) {
    let plant = plant(h, 27, "M27").await;
    let recipe = create_named(h, &plant, 27, "REF-27", "Recipe").await;
    let first = recipe.versions()[0].id();
    approve(h, first).await;
    retire(h, first).await;
    create_version(h, (recipe.id(), version_id(127)), 2, Some(first)).await;
    discard(h, version_id(127)).await;
    create_version(h, (recipe.id(), version_id(227)), 3, None).await;
    discard(h, version_id(227)).await;
    create_version(h, (recipe.id(), version_id(327)), 4, Some(version_id(127))).await;

    let base = detail(h, version_id(127)).await.unwrap();
    let fourth = detail(h, version_id(327)).await.unwrap();

    assert_eq!(base.state, VersionState::Discarded);
    assert_eq!(fourth.based_on, Some(2));
}

// The recipe numbers differ from the machine numbers. `machine_id(n)` and `recipe_id(n)` share
// one UUID, and the seed of a machine names the machine as the aggregate of its entry.
async fn last_change_gives_the_last_actor_at_the_exact_time_only(h: &dyn Harness) {
    let plant = plant(h, 23, "M23").await;
    let recipe = create_named(h, &plant, 123, "REF-23", "Recipe").await;
    let version = recipe.versions()[0].id();
    let editor = second_user(h, "editor").await;
    touch_by(h, (recipe.id(), version), &[editor]).await;
    let touched = detail(h, version).await.unwrap();

    let at_touch = last_change(h, recipe.id(), touched.updated_at).await;
    let before = last_change(h, recipe.id(), far_past()).await;

    assert_eq!(at_touch, Some(login("editor")));
    assert_eq!(before, None);
}

async fn last_change_without_an_entry_of_the_recipe_gives_none(h: &dyn Harness) {
    let plant = plant(h, 24, "M24").await;
    let recipe = create_named(h, &plant, 124, "REF-24", "Recipe").await;
    let created = detail(h, recipe.versions()[0].id()).await.unwrap();

    let at_creation = last_change(h, recipe.id(), created.updated_at).await;

    assert_eq!(at_creation, None);
}

contract_case!(
    version_page_gives_the_newest_number_first_and_cuts_the_page,
    version_page_gives_the_newest_number_first_and_cuts_the_page_memory,
    version_page_gives_the_newest_number_first_and_cuts_the_page_postgres
);
contract_case!(
    version_detail_gives_the_logins_of_the_creator_and_the_approver,
    version_detail_gives_the_logins_of_the_creator_and_the_approver_memory,
    version_detail_gives_the_logins_of_the_creator_and_the_approver_postgres
);
contract_case!(
    version_detail_of_a_draft_has_no_approver_and_a_missing_version_gives_none,
    version_detail_of_a_draft_has_no_approver_and_a_missing_version_gives_none_memory,
    version_detail_of_a_draft_has_no_approver_and_a_missing_version_gives_none_postgres
);
contract_case!(
    version_detail_of_a_version_without_a_base_has_no_base_number,
    version_detail_of_a_version_without_a_base_has_no_base_number_memory,
    version_detail_of_a_version_without_a_base_has_no_base_number_postgres
);
contract_case!(
    version_detail_gives_the_number_of_the_base_version,
    version_detail_gives_the_number_of_the_base_version_memory,
    version_detail_gives_the_number_of_the_base_version_postgres
);
contract_case!(
    version_detail_gives_the_number_of_a_base_version_that_is_a_discarded_draft,
    version_detail_gives_the_number_of_a_base_version_that_is_a_discarded_draft_memory,
    version_detail_gives_the_number_of_a_base_version_that_is_a_discarded_draft_postgres
);
contract_case!(
    last_change_gives_the_last_actor_at_the_exact_time_only,
    last_change_gives_the_last_actor_at_the_exact_time_only_memory,
    last_change_gives_the_last_actor_at_the_exact_time_only_postgres
);
contract_case!(
    last_change_without_an_entry_of_the_recipe_gives_none,
    last_change_without_an_entry_of_the_recipe_gives_none_memory,
    last_change_without_an_entry_of_the_recipe_gives_none_postgres
);
