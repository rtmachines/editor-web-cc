//! Tests of the tag link of the first row of a parameter in each diff section, on the diff page and
//! on the approval preview (ADR-0050 (d), step-7 item VW4b1-10).
//!
//! The link opens the parameter page of a version that holds the parameter. It opens the old
//! version only when the new version does not hold the parameter. A limit row that only the old
//! side holds can come first in its section. So the part of the first row does not decide the
//! version.
//!
//! In the values section, the rows of the new leaves come first (`values.rs` of the domain diff).
//! The test of the retyped parameter also checks the values section. That check fails when the
//! draft keeps the old type.

use recipe_app::ports::{Isolation, Transaction as _};
use recipe_domain::plant::{LeafPath, S7Address};
use recipe_domain::recipe::{RecipeVersionId, StoredParameter};

use super::test_world::{R500_DIFF, admin_main};
use crate::pages::test_helpers::section;
use crate::test_support::Fixture;
use crate::test_support::recipes::approval::ApprovalData;

/// The approval preview of the draft version 2 of `R-500`.
const R500_PREVIEW: &str = "/recipes/R-500/versions/2/approve";

/// The two pages that show the diff of version 2 of `R-500` against version 1.
const PAGES: [&str; 2] = [R500_DIFF, R500_PREVIEW];

/// A fixture with the approval data.
async fn seeded() -> (Fixture, ApprovalData) {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    (fixture, data)
}

/// Return the stored parameter of `version` with the tag `tag`.
async fn stored(fixture: &Fixture, version: RecipeVersionId, tag: &str) -> StoredParameter {
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let parameters = tx.recipes().version_parameters(version).await.unwrap();
    drop(tx);
    parameters
        .into_iter()
        .find(|parameter| parameter.tag.as_str() == tag)
        .unwrap()
}

/// Return the tag link of `parameter` to its page in the version `number` of `R-500`.
fn tag_link(number: u32, parameter: &StoredParameter) -> String {
    format!(
        "href=\"/recipes/R-500/versions/{number}/parameters/{}\">{}</a>",
        parameter.key, parameter.tag
    )
}

/// Assert on each page that the section `kind` links the tag of `parameter` to the version
/// `linked` and never to the version `other`.
async fn assert_tag_links(
    (fixture, data): (&Fixture, &ApprovalData),
    kind: &str,
    parameter: &StoredParameter,
    (linked, other): (u32, u32),
) {
    for page in PAGES {
        let main = admin_main(fixture, data, page).await;
        let rows = section(&main, kind);
        let wanted = tag_link(linked, parameter);
        assert!(rows.contains(&wanted), "{page} {kind}: {wanted}: {rows}");
        let unwanted = tag_link(other, parameter);
        assert!(
            !rows.contains(&unwanted),
            "{page} {kind}: {unwanted}: {rows}"
        );
    }
}

/// Remove the limit row `.head.a` from `LINE_A` in the draft. The other rows stay equal.
///
/// Then the first and only limit row of `LINE_A` in the diff only has the old side.
async fn drop_first_limit_of_line_a(fixture: &Fixture, data: &ApprovalData) -> StoredParameter {
    let line_a = stored(fixture, data.r500_v2, "LINE_A").await;
    let first = LeafPath::parse(".head.a").unwrap();
    let trimmed = StoredParameter {
        limits: line_a
            .limits
            .iter()
            .filter(|limit| limit.leaf_path != first)
            .cloned()
            .collect(),
        ..line_a
    };
    fixture.store.seed_parameter(data.r500_v2, &trimmed);
    trimmed
}

/// Change the type of `SPEED_TABLE` in the draft from `ARRAY[1..8] OF INT` to `INT`.
///
/// The value and the limit come from `SPEED`. The old row `[*]` covers eight leaves, so it comes
/// before the new row of the whole value. Every leaf of the array only has the old side.
async fn retype_speed_table(fixture: &Fixture, data: &ApprovalData) -> StoredParameter {
    let speed = stored(fixture, data.r500_v2, "SPEED").await;
    let table = stored(fixture, data.r500_v2, "SPEED_TABLE").await;
    let retyped = StoredParameter {
        plc_type: speed.plc_type,
        s7_address: Some(S7Address::parse("DB10.DBW10").unwrap()),
        limits: speed.limits,
        value: speed.value,
        ..table
    };
    fixture.store.seed_parameter(data.r500_v2, &retyped);
    retyped
}

#[tokio::test]
async fn diff_tag_of_a_kept_parameter_with_an_old_only_first_limit_row_links_the_new_version() {
    let (fixture, data) = seeded().await;
    let line_a = drop_first_limit_of_line_a(&fixture, &data).await;
    assert_tag_links((&fixture, &data), "limits", &line_a, (2, 1)).await;
}

#[tokio::test]
async fn diff_tag_of_a_retyped_parameter_with_an_old_only_first_limit_row_links_the_new_version() {
    let (fixture, data) = seeded().await;
    let table = retype_speed_table(&fixture, &data).await;
    for kind in ["limits", "values"] {
        assert_tag_links((&fixture, &data), kind, &table, (2, 1)).await;
    }
}

#[tokio::test]
async fn diff_tag_of_a_removed_parameter_links_the_old_version() {
    let (fixture, data) = seeded().await;
    let old_only = stored(&fixture, data.r500_v1, "OLD_ONLY").await;
    for kind in ["limits", "removed"] {
        assert_tag_links((&fixture, &data), kind, &old_only, (1, 2)).await;
    }
}

#[tokio::test]
async fn diff_tag_of_an_added_parameter_links_the_new_version() {
    let (fixture, data) = seeded().await;
    let added = stored(&fixture, data.r500_v2, "ADDED").await;
    for kind in ["limits", "added"] {
        assert_tag_links((&fixture, &data), kind, &added, (2, 1)).await;
    }
}
