//! Tests of the shown name in the added and the removed sections (ADR-0152 (a), (e) to (g)).
//! Test code.
//!
//! The approval preview and the diff page share the sections. The name cell follows the rule of
//! ADR-0126. A translation of the translation language shows without a marker. Without one, the UI
//! language `pl` shows `name_en` in a `lang="en"` span with the marker outside the span. The UI
//! language `en` shows `name_en` as plain text without a `lang` attribute.
//!
//! In the world of `R-500`, version 2 holds the parameter `ADDED` and version 1 holds `OLD_ONLY`.
//! The first is the added section of both pages. The second is the removed section.

use recipe_domain::recipe::{LangCode, RecipeVersionId, StoredParameter, Translation};
use recipe_domain::text::Text;

use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::{assert_clean_html, section};
use crate::test_support::plant::main_of;

/// The diff of the draft version 2 of `R-500` against the approved version 1.
const R500_DIFF: &str = "/recipes/R-500/versions/2/diff/1";
/// The approval preview of the draft version 2 of `R-500`.
const R500_PREVIEW: &str = "/recipes/R-500/versions/2/approve";
/// The pages that share the sections.
const PAGES: [&str; 2] = [R500_DIFF, R500_PREVIEW];
/// The English name of the added parameter.
const ADDED_NAME: &str = "Feed rate";
/// The Polish name of the added parameter.
const ADDED_PL: &str = "Posuw";
/// The English name of the removed parameter.
const REMOVED_NAME: &str = "Line speed";
/// The Polish name of the removed parameter.
const REMOVED_PL: &str = "Prędkość linii";

/// Give the parameter `tag` of `version` the English name `name_en` and, if given, the `pl` name.
async fn name_parameter(
    world: &AuditWorld,
    (version, tag): (RecipeVersionId, &str),
    name_en: &str,
    pl: Option<&str>,
) {
    let stored = world.stored_of(version, tag).await;
    let translations = pl
        .map(|name| Translation {
            lang: LangCode::parse("pl").unwrap(),
            name: Text::parse(name).unwrap(),
            description: None,
        })
        .into_iter()
        .collect();
    let named = StoredParameter {
        name_en: Text::parse(name_en).unwrap(),
        translations,
        ..stored
    };
    world.fixture.store.seed_parameter(version, &named);
}

/// Name the added parameter `ADDED` of version 2.
async fn name_added(world: &AuditWorld, pl: Option<&str>) {
    let target = (world.data.r500_v2, "ADDED");
    name_parameter(world, target, ADDED_NAME, pl).await;
}

/// Name the removed parameter `OLD_ONLY` of version 1.
async fn name_removed(world: &AuditWorld, pl: Option<&str>) {
    let target = (world.data.r500_v1, "OLD_ONLY");
    name_parameter(world, target, REMOVED_NAME, pl).await;
}

/// Return the section `kind` of `path` in the UI language `code` as the Administrator.
async fn section_in(world: &AuditWorld, code: &str, path: &str, kind: &str) -> String {
    let reply = world.get(&world.data.admin, code, path).await;
    assert_clean_html(&reply.body);
    section(main_of(&reply.body), kind).to_owned()
}

/// Assert that the section shows `shown` as a plain cell, with none of the `absent` texts.
fn assert_plain_name(table: &str, shown: &str, absent: &[&str], what: &str) {
    assert!(
        table.contains(&format!("<td>{shown}</td>")),
        "{what}: {table}"
    );
    for text in absent {
        assert!(!table.contains(text), "{what}: {text}: {table}");
    }
}

/// Assert that the section shows `name_en` in a `lang="en"` span, then the marker outside it.
fn assert_marked_name(table: &str, name_en: &str, what: &str) {
    let cell = format!("<td><span lang=\"en\">{name_en}</span> (en)</td>");
    assert!(table.contains(&cell), "{what}: {table}");
}

#[tokio::test]
async fn diff_name_pl_added_parameter_with_a_translation_shows_it_without_a_marker() {
    let world = audit_world().await;
    name_added(&world, Some(ADDED_PL)).await;
    for path in PAGES {
        let table = section_in(&world, "pl", path, "added").await;
        assert_plain_name(&table, ADDED_PL, &[ADDED_NAME, "(en)"], path);
    }
}

#[tokio::test]
async fn diff_name_pl_added_parameter_without_a_translation_shows_name_en_with_a_marker() {
    let world = audit_world().await;
    name_added(&world, None).await;
    for path in PAGES {
        let table = section_in(&world, "pl", path, "added").await;
        assert_marked_name(&table, ADDED_NAME, path);
    }
}

#[tokio::test]
async fn diff_name_pl_removed_parameter_with_a_translation_shows_it_without_a_marker() {
    let world = audit_world().await;
    name_removed(&world, Some(REMOVED_PL)).await;
    for path in PAGES {
        let table = section_in(&world, "pl", path, "removed").await;
        assert_plain_name(&table, REMOVED_PL, &[REMOVED_NAME, "(en)"], path);
    }
}

#[tokio::test]
async fn diff_name_en_added_parameter_shows_name_en_without_a_marker_and_a_lang() {
    let world = audit_world().await;
    name_added(&world, Some(ADDED_PL)).await;
    for path in PAGES {
        let table = section_in(&world, "en", path, "added").await;
        assert_plain_name(&table, ADDED_NAME, &[ADDED_PL, "(en)", "lang="], path);
    }
}

#[tokio::test]
async fn diff_name_en_removed_parameter_shows_name_en_without_a_marker_and_a_lang() {
    let world = audit_world().await;
    name_removed(&world, Some(REMOVED_PL)).await;
    for path in PAGES {
        let table = section_in(&world, "en", path, "removed").await;
        assert_plain_name(&table, REMOVED_NAME, &[REMOVED_PL, "(en)", "lang="], path);
    }
}
