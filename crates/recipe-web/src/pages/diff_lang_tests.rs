//! Tests of the language of the texts in the diff sections (finding CX-DR-1, WCAG 2.2 success
//! criterion 3.1.2, `design-system.md` section 1). Test code.
//!
//! The approval preview and the diff page share the sections. A translation text carries the tag
//! of its translation row. A definition text `name_en` or `description_en` carries `lang="en"`. The
//! labels, the column headers, and the word for an empty side stay in the UI language and carry no
//! `lang`.

use recipe_domain::recipe::{LangCode, RecipeVersionId, StoredParameter, Translation};
use recipe_domain::text::{LongText, Text};

use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::{assert_clean_html, section};
use crate::test_support::plant::main_of;

/// The diff of the draft version 2 of `R-500` against the approved version 1.
const R500_DIFF: &str = "/recipes/R-500/versions/2/diff/1";
/// The approval preview of the draft version 2 of `R-500`.
const R500_PREVIEW: &str = "/recipes/R-500/versions/2/approve";
/// The UI languages of the tests.
const UI_LANGUAGES: [&str; 2] = ["pl", "en"];

/// Return the translation of `lang` with a name and an optional description.
fn translation(lang: &str, name: &str, description: Option<&str>) -> Translation {
    Translation {
        lang: LangCode::parse(lang).unwrap(),
        name: Text::parse(name).unwrap(),
        description: description.map(|text| LongText::parse(text).unwrap()),
    }
}

/// Give `LINE_A` of one version the definition texts and the `de` translation of that version.
async fn describe_line_a(
    world: &AuditWorld,
    version: RecipeVersionId,
    texts: (&str, &str),
    translations: Vec<Translation>,
) {
    let stored = world.stored_of(version, "LINE_A").await;
    let described = StoredParameter {
        name_en: Text::parse(texts.0).unwrap(),
        description_en: LongText::parse(texts.1).unwrap(),
        translations,
        ..stored
    };
    world.fixture.store.seed_parameter(version, &described);
}

/// Change the definition texts and the translations of `LINE_A` between version 1 and 2.
///
/// Version 1 holds a `de` translation without a description. Version 2 changes it and adds a `pl`
/// translation, so the `pl` row has an empty old side and an empty description.
async fn change_line_a(world: &AuditWorld) {
    let data = &world.data;
    let old = vec![translation("de", "Alte Linie", None)];
    describe_line_a(world, data.r500_v1, ("Line A old", "Old description"), old).await;
    let new = vec![
        translation("de", "Neue Linie", Some("Neue Beschreibung")),
        translation("pl", "Nowa linia", None),
    ];
    describe_line_a(world, data.r500_v2, ("Line A new", "New description"), new).await;
}

/// Return `main` of `path` in the UI language `code` as the Administrator.
async fn main_in(world: &AuditWorld, code: &str, path: &str) -> String {
    let reply = world.get(&world.data.admin, code, path).await;
    assert_clean_html(&reply.body);
    main_of(&reply.body).to_owned()
}

/// Return the opening tags that start with `start`, for example `<th`.
fn opening_tags<'a>(html: &'a str, start: &str) -> Vec<&'a str> {
    html.split(start)
        .skip(1)
        .filter_map(|rest| rest.split_once('>').map(|(tag, _after)| tag))
        .collect()
}

/// Assert the language of the translation texts in `main`.
fn assert_translations_carry_their_language(main: &str, ui: &str) {
    let translations = section(main, "translations");
    for text in [
        "<dd lang=\"de\">Alte Linie</dd>",
        "<dd lang=\"de\">Neue Linie</dd>",
        "<dd lang=\"de\">Neue Beschreibung</dd>",
    ] {
        assert!(translations.contains(text), "{ui}: {text}: {translations}");
    }
    // The two empty descriptions and the one empty side show the word of the UI language.
    assert_eq!(
        translations
            .matches("<dd><span class=\"diff-none\">")
            .count(),
        2,
        "{ui}: {translations}"
    );
    assert_eq!(
        translations
            .matches("<td><span class=\"diff-none\">")
            .count(),
        1,
        "{ui}: {translations}"
    );
    // The tag of the language stays a plain cell.
    assert!(translations.contains("<td class=\"data\">de</td>"), "{ui}");
}

/// Assert the language of the definition texts in `main`.
fn assert_definitions_carry_english(main: &str, ui: &str) {
    let definitions = section(main, "definitions");
    for text in [
        "<td lang=\"en\">Line A old</td>",
        "<td lang=\"en\">Line A new</td>",
        "<td lang=\"en\">Old description</td>",
        "<td lang=\"en\">New description</td>",
    ] {
        assert!(definitions.contains(text), "{ui}: {text}: {definitions}");
    }
    // Only the four text cells carry a `lang`. The label and the state cell stay in the UI language.
    let cells = opening_tags(definitions, "<td");
    assert_eq!(
        cells.iter().filter(|tag| tag.contains("lang")).count(),
        4,
        "{ui}: {definitions}"
    );
}

/// Assert that no column header and no label of a pair carries a `lang`.
///
/// The exact counts of the empty sides in `assert_translations_carry_their_language` show that the
/// word for an empty side has no `lang` either.
fn assert_ui_texts_carry_no_language(main: &str, ui: &str) {
    for kind in ["definitions", "translations"] {
        let table = section(main, kind);
        for start in ["<th", "<dt"] {
            let tags = opening_tags(table, start);
            assert!(
                tags.iter().all(|tag| !tag.contains("lang")),
                "{ui}: {start}: {table}"
            );
        }
    }
}

#[tokio::test]
async fn diff_page_texts_carry_their_language() {
    let world = audit_world().await;
    change_line_a(&world).await;
    for ui in UI_LANGUAGES {
        let main = main_in(&world, ui, R500_DIFF).await;
        assert_translations_carry_their_language(&main, ui);
        assert_definitions_carry_english(&main, ui);
        assert_ui_texts_carry_no_language(&main, ui);
    }
}

#[tokio::test]
async fn approval_preview_texts_carry_their_language() {
    let world = audit_world().await;
    change_line_a(&world).await;
    for ui in UI_LANGUAGES {
        let main = main_in(&world, ui, R500_PREVIEW).await;
        assert_translations_carry_their_language(&main, ui);
        assert_definitions_carry_english(&main, ui);
        assert_ui_texts_carry_no_language(&main, ui);
    }
}
