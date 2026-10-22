//! Tests of the table rows of the diff sections on the approval preview: one table row for each
//! row of `diff_pairs`, so the counts of the summary equal the rows of the tables (task 10.1,
//! design D4, ADR-0050 (c), spec "Preview of a changed limit").

use axum::http::StatusCode;
use recipe_app::blocking::{AppSettings, DiffSettings, PageSizes};
use recipe_app::memory::seed_type_id;
use recipe_app::ports::{Isolation, Transaction as _};
use recipe_domain::plant::{Primitive, S7Address};
use recipe_domain::recipe::{LangCode, Ordinal, RecipeVersionId, StoredParameter, Translation};
use recipe_domain::text::{LongText, Text};

use super::preview_tests::{
    admin_preview, assert_before, assert_old_header_first, part_of, section,
};
use crate::pages::audit::test_world::{AuditWorld, audit_world};
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

/// Return the stored parameter of `version` with the tag `tag`.
pub(super) async fn stored_parameter(
    fixture: &Fixture,
    version: RecipeVersionId,
    tag: &str,
) -> StoredParameter {
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let stored = tx.recipes().version_parameters(version).await.unwrap();
    drop(tx);
    stored
        .into_iter()
        .find(|parameter| parameter.tag.as_str() == tag)
        .unwrap()
}

/// Give `LINE_A` of `version` a `de` translation without a description and a `pl` translation.
async fn translate_line_a(fixture: &Fixture, version: RecipeVersionId) {
    let line_a = stored_parameter(fixture, version, "LINE_A").await;
    let translated = StoredParameter {
        translations: vec![
            translation("de", "Linie A", None),
            translation("pl", "Linia A", Some("Opis linii A")),
        ],
        ..line_a
    };
    fixture.store.seed_parameter(version, &translated);
}

/// Change the type of `SPEED` in `version` from `INT` to `DINT`, with an address of that width.
///
/// The limit row of `SPEED` stays `0 to 3000`, so the limit row is equal on both sides.
async fn retype_speed(fixture: &Fixture, version: RecipeVersionId) {
    let speed = stored_parameter(fixture, version, "SPEED").await;
    let retyped = StoredParameter {
        plc_type: seed_type_id(Primitive::Dint, None).unwrap(),
        s7_address: Some(S7Address::parse("DB10.DBD0").unwrap()),
        ..speed
    };
    fixture.store.seed_parameter(version, &retyped);
}

/// Change the English name of `LINE_A` in `version`.
async fn rename_line_a(fixture: &Fixture, version: RecipeVersionId) {
    let line_a = stored_parameter(fixture, version, "LINE_A").await;
    let renamed = StoredParameter {
        name_en: Text::parse("Line A renamed").unwrap(),
        ..line_a
    };
    fixture.store.seed_parameter(version, &renamed);
}

/// Return `main` of the preview of `R-500` version 2 after `retype_speed` and `rename_line_a`.
async fn retyped_preview(world: &AuditWorld) -> String {
    retype_speed(&world.fixture, world.data.r500_v2).await;
    rename_line_a(&world.fixture, world.data.r500_v2).await;
    admin_preview(world).await
}

/// Assert the counts of the summary item of the section `kind` and the rows of its table.
fn assert_counts_equal_rows(main: &str, kind: &str, parameters: usize, rows: usize) {
    let summary = part_of(main, "<ul class=\"diff-summary\">", "</ul>");
    let item = part_of(summary, &format!("href=\"#diff-{kind}\""), "</li>");
    for text in [
        format!("<span>Parameters: {parameters}</span>"),
        format!("<span>Rows: {rows}</span>"),
    ] {
        assert!(item.contains(&text), "{text}: {item}");
    }
    let table = section(main, kind);
    assert_eq!(body_rows(table).len(), rows, "{kind}: {table}");
}

/// Return `main` of the preview of `R-500` version 2 with the translations of `LINE_A`.
async fn translated_preview(world: &AuditWorld) -> String {
    translate_line_a(&world.fixture, world.data.r500_v2).await;
    admin_preview(world).await
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
    for (kind, parameters, rows) in [("limits", 3, 3), ("translations", 1, 2)] {
        assert_counts_equal_rows(&main, kind, parameters, rows);
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
    assert!(row.contains("<td class=\"data\">[*]</td>"), "{row}");
    // The old limit comes first and the new limit second (spec "Preview of a changed limit").
    assert_before(
        row,
        "<td class=\"diff-value\">0 to 3000</td>",
        "<td class=\"diff-value\">10 to 3000</td>",
    );
    // A limit that only version 1 holds: the old side shows the limit, the new side shows none.
    let old_only = rows.iter().find(|row| row.contains(">OLD_ONLY<")).unwrap();
    assert_before(
        old_only,
        "<td class=\"diff-value\">0 to 10</td>",
        "<span class=\"diff-none\">none</span>",
    );
    // The header of the old version comes first, as the cells do.
    assert_old_header_first(limits, true);
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
            "<dl><dt>Name</dt><dd lang=\"pl\">Linia A</dd><dt>Description</dt><dd lang=\"pl\">Opis linii A</dd></dl>"
        ),
        "{pl}"
    );
    let de = rows
        .iter()
        .find(|row| row.contains("<td class=\"data\">de</td>"))
        .unwrap();
    assert!(
        de.contains(
            "<dl><dt>Name</dt><dd lang=\"de\">Linie A</dd><dt>Description</dt><dd><span class=\"diff-none\">none</span></dd></dl>"
        ),
        "{de}"
    );
}

/// A type change shows every limit row of both sides, also a row with the same limit on each side
/// (ADR-0050 (c), design D4). The row appears in the table and in the counts of the summary.
#[tokio::test]
async fn approve_preview_retyped_parameter_shows_its_equal_limit_row() {
    let world = audit_world().await;
    let main = retyped_preview(&world).await;
    let limits = section(&main, "limits");
    let rows = body_rows(limits);
    let speed: Vec<&&str> = rows.iter().filter(|row| row.contains(">SPEED<")).collect();
    assert_eq!(speed.len(), 1, "{limits}");
    let same_limit = "<td class=\"diff-value\">0 to 3000</td>";
    assert_eq!(
        speed.first().unwrap().matches(same_limit).count(),
        2,
        "{limits}"
    );
    assert_counts_equal_rows(&main, "limits", 4, 4);
}

/// The summary counts the definitions as the parameters and as the rows of its table: one row
/// for each changed field (design D4). `SPEED` changes two fields and `LINE_A` changes one.
#[tokio::test]
async fn approve_preview_definition_counts_equal_the_rows_of_its_table() {
    let world = audit_world().await;
    let main = retyped_preview(&world).await;
    assert_counts_equal_rows(&main, "definitions", 2, 3);
    let definitions = section(&main, "definitions");
    assert_old_header_first(definitions, false);
    let rows = body_rows(definitions);
    let row_of = |field: &str| {
        let cell = format!("<td>{field}</td>");
        rows.iter().find(|row| row.contains(&cell)).unwrap()
    };
    // Each row shows the old side before the new side.
    assert_before(row_of("Type"), ">INT</td>", ">DINT</td>");
    assert_before(row_of("S7 address"), ">DB10.DBW0</td>", ">DB10.DBD0</td>");
    assert_before(
        row_of("English name"),
        "<td lang=\"en\">LINE_A</td>",
        "<td lang=\"en\">Line A renamed</td>",
    );
}

/// The order of `LINE_A` in `version` after `reorder_line_a`. No other parameter of the seed has it.
const MOVED_ORDINAL: u32 = 77;

/// Move `LINE_A` in `version` to the order `MOVED_ORDINAL`.
async fn reorder_line_a(fixture: &Fixture, version: RecipeVersionId) {
    let line_a = stored_parameter(fixture, version, "LINE_A").await;
    let moved = StoredParameter {
        ordinal: Ordinal::parse(MOVED_ORDINAL).unwrap(),
        ..line_a
    };
    fixture.store.seed_parameter(version, &moved);
}

/// A changed order is a number. Both sides align to the right in the class `num`
/// (`design-system.md` sections 5 and 10.4, finding CX-DR-3).
#[tokio::test]
async fn approve_preview_changed_ordinal_shows_each_side_in_a_right_aligned_number_cell() {
    let world = audit_world().await;
    let old = stored_parameter(&world.fixture, world.data.r500_v1, "LINE_A")
        .await
        .ordinal
        .value();
    reorder_line_a(&world.fixture, world.data.r500_v2).await;
    let main = admin_preview(&world).await;
    let definitions = section(&main, "definitions");
    let rows = body_rows(definitions);
    let row = rows
        .iter()
        .find(|row| row.contains(&format!("{MOVED_ORDINAL}</td>")))
        .expect(definitions);
    assert_before(
        row,
        &format!("<td class=\"num\">{old}</td>"),
        &format!("<td class=\"num\">{MOVED_ORDINAL}</td>"),
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
