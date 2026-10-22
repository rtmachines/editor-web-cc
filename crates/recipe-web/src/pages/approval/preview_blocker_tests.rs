//! Tests that the approval preview shows one line for each blocker entry, in order, with its
//! code, its tag, and its link (item CX-RCW-1, design D10, spec "Bound of the blocker list").
//!
//! `blockers_tests` checks the vectors of lines. These tests read the HTML of the page, so a
//! template that shows only the first entry fails them.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use recipe_domain::plant::{MachineId, UnitId, UnitPath, UnitUsage};
use recipe_domain::recipe::{RecipeVersionId, VersionState};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::approve_error_tests::seed_unlimited_ints;
use super::preview_rows_tests::stored_parameter;
use super::preview_tests::part_of;
use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::recipes::test_world::{seed_parameter_on_unit, seed_recipe_of_m20};
use crate::test_support::plant::main_of;
use crate::test_support::recipes::M20_ID;
use crate::test_support::visible_text::visible_text;

/// The number of the lines that the notice shows (ADR-0049 (c), ADR-0041 (d)).
const SHOWN_LINES: usize = 20;
/// The instant of the archive of the unit `U2` of `M20`.
const ARCHIVED_AT_SECONDS: i64 = 600_000;
/// The code of the item of a parameter on an archived unit (`E-RCP-020` lists it).
const ARCHIVED_UNIT: &str = "E-PLT-009";

/// One blocker line of the notice, as the page shows it.
#[derive(Debug, PartialEq, Eq)]
struct Line {
    /// The code of the line.
    code: String,
    /// The tag that the line names.
    tag: String,
    /// The address of the parameter page, or `None` for a line without a link.
    href: Option<String>,
}

/// Read one line of the notice from the markup after its `<li>`.
fn read_line(item: &str) -> Line {
    let href = item
        .split_once("href=\"")
        .map(|(_before, rest)| rest.split('"').next().unwrap().to_owned());
    let text = visible_text(item);
    let mut words = text.split_whitespace();
    Line {
        code: words.next().unwrap().to_owned(),
        tag: words.next().unwrap().to_owned(),
        href,
    }
}

/// Return the lines of the notice of the blockers in the page `body`, in their order.
/// Also return the markup of the notice.
///
/// The sentence that counts the other lines has no code, so it is not a line.
fn notice_lines(body: &str) -> (Vec<Line>, &str) {
    let notice = part_of(main_of(body), "<h2 id=\"approve-blockers\">", "</ul>");
    let lines = notice
        .split("<li>")
        .skip(1)
        .filter(|item| item.contains("alert-code"))
        .map(read_line)
        .collect();
    (lines, notice)
}

/// Return the address of the parameter page of each of the `tags` of `version`.
///
/// The key of each parameter comes from the store through `stored_parameter`.
async fn links_by_tag(
    world: &AuditWorld,
    version: RecipeVersionId,
    base: &str,
    tags: &[String],
) -> BTreeMap<String, String> {
    let mut links = BTreeMap::new();
    for tag in tags {
        let stored = stored_parameter(&world.fixture, version, tag).await;
        links.insert(tag.clone(), format!("{base}/parameters/{}", stored.key));
    }
    links
}

/// Build the lines that the page must show: the `code` and the linked `tags` in this order.
fn expected(code: &str, tags: &[String], links: &BTreeMap<String, String>) -> Vec<Line> {
    tags.iter()
        .map(|tag| Line {
            code: code.to_owned(),
            tag: tag.clone(),
            href: Some(links.get(tag).unwrap().clone()),
        })
        .collect()
}

/// Return the tags `P01` to `P<count>`.
fn numbered_tags(count: u32) -> Vec<String> {
    (1..=count).map(|n| format!("P{n:02}")).collect()
}

/// Seed the draft of the recipe `(number, reference)` and return its id.
fn seed_draft(world: &AuditWorld, number: u32, reference: &str) -> RecipeVersionId {
    let states = [VersionState::LiveDraft];
    let ids = seed_recipe_of_m20(&world.fixture, (number, reference), &states);
    *ids.first().unwrap()
}

/// Archive the unit `U2` of `M20` and return its identifier.
///
/// A draft that holds a parameter on that unit fails the snapshot check with one item for each
/// parameter. The database holds a foreign key, so a unit that does not exist cannot occur.
fn archive_u2_of_m20(world: &AuditWorld) -> UnitId {
    let at = Timestamp::from(OffsetDateTime::from_unix_timestamp(ARCHIVED_AT_SECONDS).unwrap());
    let path = UnitPath::parse("U2").unwrap();
    let mut machine = world
        .fixture
        .store
        .state()
        .machine(MachineId::from(M20_ID))
        .unwrap();
    machine
        .archive_unit(&path, at, &UnitUsage::default())
        .unwrap();
    world.fixture.store.seed_machine(&machine);
    machine.unit(&path).unwrap().id()
}

/// Archive the unit `U2` of `M20`. Then seed the parameters `tags` on it in this order.
fn seed_on_archived_unit(world: &AuditWorld, version: RecipeVersionId, tags: &[String]) {
    let unit = archive_u2_of_m20(world);
    for (n, tag) in (1..).zip(tags) {
        seed_parameter_on_unit(&world.fixture, version, unit, (n, tag));
    }
}

/// Return the page of the preview of version 1 of `reference` for `jsmith`.
async fn preview_of(world: &AuditWorld, reference: &str) -> String {
    let path = format!("/recipes/{reference}/versions/1/approve");
    let reply = world.get_en(&world.data.jsmith, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body
}

#[tokio::test]
async fn approve_preview_shows_one_line_for_each_missing_limit_in_order_with_its_link() {
    let world = audit_world().await;
    let version = seed_draft(&world, 803, "R-803");
    seed_unlimited_ints(&world.fixture, version, 3);
    let body = preview_of(&world, "R-803").await;
    let (lines, notice) = notice_lines(&body);
    let tags = numbered_tags(3);
    let links = links_by_tag(&world, version, "/recipes/R-803/versions/1", &tags).await;
    assert_eq!(lines, expected("E-RCP-016", &tags, &links), "{notice}");
    assert!(!notice.contains("Other errors"), "{notice}");
}

#[tokio::test]
async fn approve_preview_shows_one_line_for_each_item_of_a_failed_snapshot_in_order_with_its_link()
{
    let world = audit_world().await;
    let version = seed_draft(&world, 804, "R-804");
    // The seed order differs from the tag order, so the order of the lines is the order of the tags.
    let seeded = ["P_CHARLIE", "P_ALPHA", "P_BRAVO"].map(str::to_owned);
    seed_on_archived_unit(&world, version, &seeded);
    let tags = ["P_ALPHA", "P_BRAVO", "P_CHARLIE"].map(str::to_owned);
    let body = preview_of(&world, "R-804").await;
    let (lines, notice) = notice_lines(&body);
    let links = links_by_tag(&world, version, "/recipes/R-804/versions/1", &tags).await;
    assert_eq!(lines, expected(ARCHIVED_UNIT, &tags, &links), "{notice}");
    assert!(!notice.contains("Other errors"), "{notice}");
}

#[tokio::test]
async fn approve_preview_of_a_long_failed_snapshot_shows_20_lines_and_counts_the_other_five() {
    let world = audit_world().await;
    let version = seed_draft(&world, 805, "R-805");
    seed_on_archived_unit(&world, version, &numbered_tags(25));
    let body = preview_of(&world, "R-805").await;
    let (lines, notice) = notice_lines(&body);
    let shown = numbered_tags(20);
    let links = links_by_tag(&world, version, "/recipes/R-805/versions/1", &shown).await;
    assert_eq!(lines.len(), SHOWN_LINES, "{notice}");
    assert_eq!(lines, expected(ARCHIVED_UNIT, &shown, &links), "{notice}");
    let more = "<li>Other errors: 5. Each one stands under its field.</li>";
    assert_eq!(notice.matches(more).count(), 1, "{notice}");
}
