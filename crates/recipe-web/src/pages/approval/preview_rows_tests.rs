//! Tests of the table rows of the diff sections on the approval preview: one table row for each
//! row of `diff_pairs`, so the counts of the summary equal the rows of the tables (task 10.1,
//! design D4, ADR-0050 (c), spec "Preview of a changed limit").

use axum::http::StatusCode;
use recipe_app::blocking::{AppSettings, DiffSettings, PageSizes};
use recipe_app::ports::{Isolation, Transaction as _};
use recipe_domain::recipe::{LangCode, RecipeVersionId, StoredParameter, Translation};
use recipe_domain::text::{LongText, Text};

use super::preview_tests::{part_of, section};
use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::main_of;
use crate::test_support::{Fixture, get_page};

/// The preview of the draft version 2 of `R-500`.
const R500_PREVIEW: &str = "/recipes/R-500/versions/2/approve";

fn translation(lang: &str, name: &str, description: Option<&str>) -> Translation {
    Translation {
        lang: LangCode::parse(lang).unwrap(),
        name: Text::parse(name).unwrap(),
        description: description.map(|text| LongText::parse(text).unwrap()),
    }
}

/// Give `LINE_A` of `version` a `de` translation without a description and a `pl` translation.
async fn translate_line_a(fixture: &Fixture, version: RecipeVersionId) {
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let stored = tx.recipes().version_parameters(version).await.unwrap();
    drop(tx);
    let line_a = stored
        .iter()
        .find(|parameter| parameter.tag.as_str() == "LINE_A")
        .unwrap();
    let translated = StoredParameter {
        translations: vec![
            translation("de", "Linie A", None),
            translation("pl", "Linia A", Some("Opis linii A")),
        ],
        ..line_a.clone()
    };
    fixture.store.seed_parameter(version, &translated);
}

/// Return `main` of the preview of `R-500` version 2 with the translations of `LINE_A`.
async fn translated_preview(world: &AuditWorld) -> String {
    translate_line_a(&world.fixture, world.data.r500_v2).await;
    let reply = world.get_en(&world.data.admin, R500_PREVIEW).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    main_of(&reply.body).to_owned()
}

/// Return the rows of the body of the table of `section`.
fn body_rows(section: &str) -> Vec<&str> {
    let body = part_of(section, "<tbody>", "</tbody>");
    body.split("<tr>").skip(1).collect()
}

#[tokio::test]
async fn approve_preview_summary_counts_equal_the_rows_of_each_table() {
    let world = audit_world().await;
    let main = translated_preview(&world).await;
    let summary = part_of(&main, "<ul class=\"diff-summary\">", "</ul>");
    for (kind, parameters, rows) in [("limits", 3, 3), ("translations", 1, 2)] {
        let item = part_of(summary, &format!("href=\"#diff-{kind}\""), "</li>");
        assert!(
            item.contains(&format!("<span>Parameters: {parameters}</span>")),
            "{item}"
        );
        assert!(
            item.contains(&format!("<span>Rows: {rows}</span>")),
            "{item}"
        );
        let table = section(&main, kind);
        assert_eq!(body_rows(table).len(), rows, "{kind}: {table}");
    }
}

#[tokio::test]
async fn approve_preview_limit_row_shows_each_side_as_one_limit_text() {
    let world = audit_world().await;
    let main = translated_preview(&world).await;
    let limits = section(&main, "limits");
    assert!(!limits.contains("Minimum"), "{limits}");
    let rows = body_rows(limits);
    let speed: Vec<&&str> = rows
        .iter()
        .filter(|row| row.contains(">SPEED_TABLE<"))
        .collect();
    assert_eq!(speed.len(), 1, "{limits}");
    let row = speed.first().unwrap();
    for cell in [
        "<td class=\"data\">[*]</td>",
        "<td class=\"diff-value\">0 to 3000</td>",
        "<td class=\"diff-value\">10 to 3000</td>",
    ] {
        assert!(row.contains(cell), "{cell}: {row}");
    }
    let old_only = rows.iter().find(|row| row.contains(">OLD_ONLY<")).unwrap();
    assert!(
        old_only.contains("<td class=\"diff-value\">0 to 10</td>"),
        "{old_only}"
    );
    assert!(
        old_only.contains("<span class=\"diff-none\">none</span>"),
        "{old_only}"
    );
}

#[tokio::test]
async fn approve_preview_translation_row_shows_the_name_and_the_description_of_one_language() {
    let world = audit_world().await;
    let main = translated_preview(&world).await;
    let translations = section(&main, "translations");
    assert_eq!(
        translations.matches(">LINE_A</a>").count(),
        1,
        "{translations}"
    );
    assert_eq!(
        translations.matches(">LINE_A<").count(),
        2,
        "{translations}"
    );
    let rows = body_rows(translations);
    let pl = rows
        .iter()
        .find(|row| row.contains("<td class=\"data\">pl</td>"))
        .unwrap();
    assert!(
        pl.contains("<td><span class=\"diff-none\">none</span></td>"),
        "{pl}"
    );
    assert!(
        pl.contains(
            "<dl><dt>Name</dt><dd>Linia A</dd><dt>Description</dt><dd>Opis linii A</dd></dl>"
        ),
        "{pl}"
    );
    let de = rows
        .iter()
        .find(|row| row.contains("<td class=\"data\">de</td>"))
        .unwrap();
    assert!(
        de.contains(
            "<dl><dt>Name</dt><dd>Linie A</dd><dt>Description</dt><dd><span class=\"diff-none\">none</span></dd></dl>"
        ),
        "{de}"
    );
}

/// The adjacent links of the cut sections and of a cell with more rows keep the gap of 8 px
/// (`design-system.md` section 7). The classes `link-list` and `actions` hold that gap.
#[tokio::test]
async fn approve_preview_adjacent_links_stand_in_a_list_or_a_row_with_the_gap() {
    let defaults = AppSettings::default();
    let fixture = Fixture::with_app_settings(AppSettings {
        pages: PageSizes {
            diff: 1,
            ..defaults.pages
        },
        diff: DiffSettings {
            leaf_rows: 1,
            ..defaults.diff
        },
        ..defaults
    });
    let data = fixture.seed_approval_recipes().await;
    translate_line_a(&fixture, data.r500_v2).await;
    let first = get_page(&fixture, Some(&data.admin), "en", R500_PREVIEW).await;
    let main = main_of(&first.body);
    let cut = part_of(main, "<ul class=\"link-list\">", "</ul>");
    assert!(cut.contains(">Pages of the section "), "{main}");
    let path = format!("{R500_PREVIEW}?kind=translations");
    let reply = get_page(&fixture, Some(&data.admin), "en", &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let translations = section(main_of(&reply.body), "translations");
    let more = part_of(translations, "More rows: 1", "</td>");
    let links = part_of(more, "<div class=\"actions\">", "</div>");
    assert_eq!(links.matches("class=\"target-link\"").count(), 2, "{more}");
    assert!(links.contains(">Parameter in version 1</a>"), "{more}");
    assert!(links.contains(">Parameter in version 2</a>"), "{more}");
}
