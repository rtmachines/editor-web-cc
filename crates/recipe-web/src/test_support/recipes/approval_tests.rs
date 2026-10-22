//! Tests of `Fixture::seed_approval_recipes` (design D13, task 9.4).
//!
//! Each test checks one fact of D13: the three users, the recipe `R-500` with its approved
//! version 1 and its draft version 2, the recipe `R-600` with a retired version, and the deleted
//! recipe `R-700` on the archived machine `M12`.

use std::collections::BTreeMap;

use recipe_app::auth::{CurrentUser, RequestMeta};
use recipe_app::error::AppError;
use recipe_app::identity::authenticate;
use recipe_app::memory::MemoryTransaction;
use recipe_app::ports::{Isolation, Transaction as _};
use recipe_app::recipe::{ApproveVersion, ApproveVersionInput};
use recipe_domain::audit::{AuditAction, RequestId};
use recipe_domain::identity::{Role, SessionToken, UserId};
use recipe_domain::recipe::{Recipe, ReferenceNumber, StoredParameter, VersionState};
use uuid::Uuid;

use super::approval::ApprovalData;
use crate::test_support::{Fixture, TestSession};

/// Seed the data of D13 and return the fixture with the returned identifiers.
async fn seeded() -> (Fixture, ApprovalData) {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    (fixture, data)
}

/// Open a read transaction on the store of `fixture`.
fn begin(fixture: &Fixture) -> MemoryTransaction {
    fixture.store.begin_memory(Isolation::ReadOnly).unwrap()
}

/// Return the recipe with the reference number `reference`.
async fn recipe_of(fixture: &Fixture, reference: &str) -> Recipe {
    let reference = ReferenceNumber::parse(reference).unwrap();
    begin(fixture)
        .recipes()
        .recipe_by_reference(&reference)
        .await
        .unwrap()
        .unwrap()
}

/// Return the states of the versions of `reference`, with the number of each version.
async fn states_of(fixture: &Fixture, reference: &str) -> Vec<(u32, VersionState)> {
    recipe_of(fixture, reference)
        .await
        .versions()
        .iter()
        .map(|header| (header.number(), header.state()))
        .collect()
}

/// Return the parameters of the version `number` of `R-500`, by tag.
async fn r500_parameters(fixture: &Fixture, number: u32) -> BTreeMap<String, StoredParameter> {
    let recipe = recipe_of(fixture, "R-500").await;
    let header = recipe
        .versions()
        .iter()
        .find(|header| header.number() == number)
        .unwrap();
    begin(fixture)
        .recipes()
        .version_parameters(header.id())
        .await
        .unwrap()
        .into_iter()
        .map(|parameter| (parameter.tag.as_str().to_owned(), parameter))
        .collect()
}

/// Return the login and the role of the user of `session`.
async fn who(fixture: &Fixture, session: &TestSession, number: u8) -> (String, Role) {
    let token = SessionToken::from_bytes([number; 16]);
    assert!(session.cookie.contains(&token.to_hex()));
    let auth = authenticate(fixture.state.identity(), &token)
        .await
        .unwrap()
        .unwrap();
    (auth.user().login().to_string(), auth.user().role())
}

#[tokio::test]
async fn seed_approval_recipes_adds_the_administrator_and_two_process_engineers() {
    let (fixture, data) = seeded().await;
    let rows = [
        (&data.admin, 41, "admin", Role::Administrator),
        (&data.anowak, 42, "anowak", Role::ProcessEngineer),
        (&data.jsmith, 43, "jsmith", Role::ProcessEngineer),
    ];
    for (session, number, login, role) in rows {
        let found = who(&fixture, session, number).await;
        assert_eq!(found, (login.to_owned(), role));
    }
    let ids = [data.admin_id, data.anowak_id, data.jsmith_id];
    assert_eq!(
        ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
        3
    );
}

#[tokio::test]
async fn seed_approval_recipes_r500_has_an_approved_version_1_and_a_draft_version_2_on_m7() {
    let (fixture, data) = seeded().await;
    let recipe = recipe_of(&fixture, "R-500").await;
    assert_eq!(recipe.id(), data.r500);
    assert_eq!(recipe.machine(), data.recipes.plant.m7);
    assert!(!recipe.is_deleted());
    assert_eq!(
        states_of(&fixture, "R-500").await,
        [(1, VersionState::Approved), (2, VersionState::LiveDraft)]
    );
    let ids: Vec<_> = recipe.versions().iter().map(|header| header.id()).collect();
    assert_eq!(ids, [data.r500_v1, data.r500_v2]);
}

#[tokio::test]
async fn seed_approval_recipes_audit_shows_anowak_as_author_and_admin_as_approver() {
    let (fixture, data) = seeded().await;
    let entries = fixture.store.audit_entries();
    let by = |action: AuditAction| -> Vec<Option<UserId>> {
        entries
            .iter()
            .filter(|entry| entry.action() == action)
            .map(|entry| entry.actor())
            .collect()
    };
    assert_eq!(by(AuditAction::RecipeCreated), [Some(data.anowak_id); 2]);
    // Version 1 of each recipe and version 2 of `R-500`.
    assert_eq!(by(AuditAction::VersionCreated), [Some(data.anowak_id); 3]);
    assert_eq!(
        by(AuditAction::VersionApproved),
        [Some(data.admin_id); 2],
        "R-500 version 1 and R-600 version 1"
    );
    assert_eq!(by(AuditAction::VersionRetired), [Some(data.admin_id)]);
    // Every other change was written by `anowak`: he created and edited, `jsmith` edited nothing.
    for entry in &entries {
        let approval = matches!(
            entry.action(),
            AuditAction::VersionApproved | AuditAction::VersionRetired
        );
        let expected = if approval {
            data.admin_id
        } else {
            data.anowak_id
        };
        assert_eq!(entry.actor(), Some(expected), "{:?}", entry.action());
    }
}

#[tokio::test]
async fn seed_approval_recipes_the_draft_version_2_differs_from_version_1_in_four_ways() {
    let (fixture, _data) = seeded().await;
    let (one, two) = (
        r500_parameters(&fixture, 1).await,
        r500_parameters(&fixture, 2).await,
    );
    let tags = |parameters: &BTreeMap<String, StoredParameter>| -> Vec<String> {
        parameters.keys().cloned().collect()
    };
    assert_eq!(tags(&one), ["LINE_A", "OLD_ONLY", "SPEED", "SPEED_TABLE"]);
    assert_eq!(tags(&two), ["ADDED", "LINE_A", "SPEED", "SPEED_TABLE"]);
    // The kept parameters keep their key, so the diff compares them.
    for tag in ["LINE_A", "SPEED", "SPEED_TABLE"] {
        assert_eq!(one[tag].key, two[tag].key, "{tag}");
    }
    // `SPEED` stays as it was: the same value and the same limit rows.
    assert_eq!(one["SPEED"].value, two["SPEED"].value);
    assert_eq!(limits_of(&one["SPEED"]), limits_of(&two["SPEED"]));
}

/// Return the limit rows of a parameter as text: leaf path, minimum, and maximum.
fn limits_of(parameter: &StoredParameter) -> Vec<(String, String, String)> {
    let bound = |value: &Option<_>| value.as_ref().map_or_else(String::new, ToString::to_string);
    parameter
        .limits
        .iter()
        .map(|limit| {
            (
                limit.leaf_path.to_string(),
                bound(&limit.min),
                bound(&limit.max),
            )
        })
        .collect()
}

#[tokio::test]
async fn seed_approval_recipes_the_limit_row_all_of_speed_table_changes_from_0_to_10() {
    let (fixture, _data) = seeded().await;
    let (one, two) = (
        r500_parameters(&fixture, 1).await,
        r500_parameters(&fixture, 2).await,
    );
    let row = |min: &str| vec![("[*]".to_owned(), min.to_owned(), "3000".to_owned())];
    assert_eq!(limits_of(&one["SPEED_TABLE"]), row("0"));
    assert_eq!(limits_of(&two["SPEED_TABLE"]), row("10"));
    assert_eq!(one["SPEED_TABLE"].value, two["SPEED_TABLE"].value);
}

#[tokio::test]
async fn seed_approval_recipes_a_value_inside_a_struct_in_a_struct_changes() {
    let (fixture, _data) = seeded().await;
    let (one, two) = (
        r500_parameters(&fixture, 1).await,
        r500_parameters(&fixture, 2).await,
    );
    let head_a = |parameters: &BTreeMap<String, StoredParameter>| {
        parameters["LINE_A"].value.as_json()["head"]["a"].as_i64()
    };
    assert_eq!(head_a(&one), Some(1));
    assert_eq!(head_a(&two), Some(2));
    // Only that leaf changes: the other leaves and the limits stay.
    assert_eq!(
        one["LINE_A"].value.as_json()["head"]["b"],
        two["LINE_A"].value.as_json()["head"]["b"]
    );
    assert_eq!(limits_of(&one["LINE_A"]), limits_of(&two["LINE_A"]));
}

#[tokio::test]
async fn seed_approval_recipes_version_2_of_r500_is_based_on_version_1() {
    let (fixture, data) = seeded().await;
    let created: Vec<_> = fixture
        .store
        .audit_entries()
        .into_iter()
        .filter(|entry| entry.action() == AuditAction::VersionCreated)
        .filter_map(|entry| entry.after().cloned())
        .map(|after| {
            (
                after["number"].as_u64(),
                after["based_on_version_id"].clone(),
            )
        })
        .collect();
    assert!(
        created.contains(&(Some(2), data.r500_v1.to_string().into())),
        "{created:?}"
    );
    assert_eq!(
        created
            .iter()
            .filter(|(number, _base)| *number == Some(2))
            .count(),
        1,
        "only version 2 of R-500 has a base version"
    );
}

#[tokio::test]
async fn seed_approval_recipes_anowak_cannot_approve_the_draft_version_2_that_he_edited() {
    let (fixture, data) = seeded().await;
    // The draft is unchanged after the seed, so the row version of the data is current.
    let refused = approve_version_2(&fixture, &data, &data.anowak_user())
        .await
        .unwrap_err();
    assert_eq!(refused.code().to_string(), "E-RCP-015", "{refused:?}");
    assert_eq!(
        states_of(&fixture, "R-500").await[1],
        (2, VersionState::LiveDraft)
    );
}

/// Approve version 2 of `R-500` as `user` with the row version of the data.
async fn approve_version_2(
    fixture: &Fixture,
    data: &ApprovalData,
    user: &CurrentUser,
) -> Result<(), AppError> {
    let ctx = fixture.state.identity().app().clone();
    let meta = RequestMeta::new(RequestId::from(Uuid::from_u128(9_001)), None, None, None);
    let input = ApproveVersionInput {
        reference: ReferenceNumber::parse("R-500").unwrap(),
        number: 2,
        expected_row_version: data.r500_v2_row_version,
        allow_outdated: false,
    };
    ApproveVersion::new(ctx)
        .execute(user, &meta, input)
        .await
        .map(|_outcome| ())
}

#[tokio::test]
async fn seed_approval_recipes_jsmith_edited_nothing_so_he_may_approve_version_2() {
    let (fixture, data) = seeded().await;
    approve_version_2(&fixture, &data, &data.jsmith_user())
        .await
        .unwrap();
    let states = states_of(&fixture, "R-500").await;
    assert_eq!(states[1], (2, VersionState::Approved));
}

#[tokio::test]
async fn seed_approval_recipes_the_approval_of_version_2_by_admin_retires_version_1() {
    let (fixture, data) = seeded().await;
    approve_version_2(&fixture, &data, &data.admin_user())
        .await
        .unwrap();
    assert_eq!(
        states_of(&fixture, "R-500").await,
        [(1, VersionState::Retired), (2, VersionState::Approved)]
    );
}

#[tokio::test]
async fn seed_approval_recipes_r600_has_only_a_retired_version_1() {
    let (fixture, data) = seeded().await;
    let recipe = recipe_of(&fixture, "R-600").await;
    assert_eq!(recipe.id(), data.r600);
    assert_eq!(
        recipe.versions().first().map(|header| header.id()),
        Some(data.r600_v1)
    );
    assert_eq!(recipe.machine(), data.recipes.plant.m7);
    assert_eq!(
        states_of(&fixture, "R-600").await,
        [(1, VersionState::Retired)]
    );
}

#[tokio::test]
async fn seed_approval_recipes_r700_is_deleted_and_stands_on_the_archived_machine_m12() {
    let (fixture, data) = seeded().await;
    let recipe = recipe_of(&fixture, "R-700").await;
    assert_eq!(recipe.id(), data.r700);
    assert!(recipe.is_deleted());
    assert_eq!(recipe.machine(), data.recipes.plant.m12);
    let state = fixture.store.state();
    assert!(!state.machine(data.recipes.plant.m12).unwrap().is_live());
}

#[tokio::test]
async fn seed_approval_recipes_keeps_the_data_of_seed_recipes() {
    let (fixture, data) = seeded().await;
    let expected = Fixture::new().seed_recipes();
    assert_eq!(data.recipes, expected);
    assert_eq!(
        states_of(&fixture, "R-200").await,
        [(1, VersionState::LiveDraft)]
    );
}
