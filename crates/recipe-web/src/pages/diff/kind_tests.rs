//! Tests of the back link of the diff page and of the first page with `kind` of a section without
//! an entry, on the diff page and on the approval preview (design D4 and D8, step-7 item S7-22).

use axum::http::StatusCode;

use super::test_world::R500_DIFF;
use crate::i18n::Lang;
use crate::pages::test_helpers::{assert_clean_html, seed_unedited_version_3};
use crate::test_support::plant::{main_of, text_with};
use crate::test_support::recipes::approval::ApprovalData;
use crate::test_support::{Fixture, get_page, text};

/// The approval preview of the draft version 2 of `R-500`.
const R500_PREVIEW: &str = "/recipes/R-500/versions/2/approve";

/// The diff of version 1 of `R-500` against itself. It has no change.
const R500_SAME: &str = "/recipes/R-500/versions/1/diff/1";

/// Both languages with the texts that the design table D8 gives for `R-500`.
///
/// Each entry is the language, the back link to version 2, and the notice of the section
/// "Definitions" without a change. The texts are copied from the table, not from the bundles.
const D8_TEXTS: [(Lang, &str, &str); 2] = [
    (
        Lang::En,
        "Back to version 2",
        "The section “Definitions” has no change.",
    ),
    (
        Lang::Pl,
        "Wróć do wersji 2",
        "Sekcja „Definicje” nie ma zmian.",
    ),
];

/// A fixture with the approval data.
async fn seeded() -> (Fixture, ApprovalData) {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    (fixture, data)
}

/// Send `path` as the Administrator in `lang`. Check the status and the HTML. Return `main`.
async fn main_in(lang: Lang, fixture: &Fixture, data: &ApprovalData, path: &str) -> String {
    let reply = get_page(fixture, Some(&data.admin), lang.as_str(), path).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
    assert_clean_html(&reply.body);
    main_of(&reply.body).to_owned()
}

/// Assert that `main` shows the notice `diff-kind-empty` for the section "Definitions" and a link
/// to `first` with the text `list-first-page`, and that it shows no section and no pager.
fn assert_kind_empty(lang: Lang, main: &str, (notice, first): (&str, &str)) {
    let from_bundle = text_with(
        lang,
        "diff-kind-empty",
        "section",
        &text(lang, "diff-section-definitions"),
    );
    assert_eq!(from_bundle, notice, "{lang}: the bundle text");
    assert!(
        main.contains(&format!("<span>{notice}</span>")),
        "{lang}: {main}"
    );
    let link = format!(
        "<a class=\"target-link\" href=\"{first}\">{}</a>",
        text(lang, "list-first-page")
    );
    assert!(main.contains(&link), "{lang}: {link}: {main}");
    assert!(!main.contains("class=\"pager\""), "{lang}: {main}");
    assert!(
        !main.contains("<section class=\"diff-section\""),
        "{lang}: {main}"
    );
}

#[tokio::test]
async fn diff_page_back_link_shows_the_text_of_diff_back_in_both_languages() {
    let (fixture, data) = seeded().await;
    for (lang, back, _notice) in D8_TEXTS {
        let main = main_in(lang, &fixture, &data, R500_DIFF).await;
        let from_bundle = text_with(lang, "diff-back", "number", "2");
        assert_eq!(from_bundle, back, "{lang}: the bundle text");
        let link = format!("href=\"/recipes/R-500/versions/2\">{back}</a>");
        assert!(main.contains(&link), "{lang}: {link}: {main}");
    }
}

#[tokio::test]
async fn diff_page_with_kind_of_a_section_without_a_change_shows_the_notice_and_no_pager() {
    let (fixture, data) = seeded().await;
    for (lang, _back, notice) in D8_TEXTS {
        let path = format!("{R500_DIFF}?kind=definitions");
        let main = main_in(lang, &fixture, &data, &path).await;
        assert_kind_empty(lang, &main, (notice, R500_DIFF));
        assert!(!main.contains("class=\"empty\""), "{lang}: {main}");
    }
}

#[tokio::test]
async fn approve_preview_with_kind_of_a_section_without_a_change_shows_the_notice_and_no_pager() {
    let (fixture, data) = seeded().await;
    let first = format!("{R500_PREVIEW}?rv={}", data.r500_v2_row_version);
    for (lang, _back, notice) in D8_TEXTS {
        let path = format!("{R500_PREVIEW}?kind=definitions");
        let main = main_in(lang, &fixture, &data, &path).await;
        assert_kind_empty(lang, &main, (notice, &first));
    }
}

#[tokio::test]
async fn diff_page_with_kind_of_a_section_with_a_change_shows_the_pager_and_no_notice() {
    let (fixture, data) = seeded().await;
    let main = main_in(
        Lang::En,
        &fixture,
        &data,
        &format!("{R500_DIFF}?kind=limits"),
    )
    .await;
    assert!(main.contains("id=\"diff-limits\""), "{main}");
    assert!(main.contains("class=\"pager\""), "{main}");
    assert!(!main.contains("has no change"), "{main}");
}

#[tokio::test]
async fn diff_page_with_kind_after_the_last_page_keeps_the_after_end_text() {
    let (fixture, data) = seeded().await;
    let path = format!("{R500_DIFF}?kind=limits&page=9");
    let main = main_in(Lang::En, &fixture, &data, &path).await;
    assert!(main.contains("Page 9 is after the last page."), "{main}");
    assert!(!main.contains("has no change"), "{main}");
    assert!(!main.contains("class=\"pager\""), "{main}");
}

#[tokio::test]
async fn diff_page_without_any_change_keeps_the_empty_state_and_shows_no_kind_notice() {
    let (fixture, data) = seeded().await;
    let path = format!("{R500_SAME}?kind=definitions");
    let main = main_in(Lang::En, &fixture, &data, &path).await;
    assert!(
        main.contains("<p class=\"empty\">Versions 1 and 1 have the same parameters.</p>"),
        "{main}"
    );
    assert!(!main.contains("has no change"), "{main}");
    assert!(!main.contains("class=\"pager\""), "{main}");
}

/// A preview of a draft without any change shows `diff-empty` and no pager, also with `kind`
/// (design D4, step-7 item S7-22).
#[tokio::test]
async fn approve_preview_without_any_change_keeps_the_empty_state_and_shows_no_pager_with_kind() {
    let (fixture, data) = seeded().await;
    seed_unedited_version_3(&fixture, &data).await;
    for kind in ["", "?kind=definitions"] {
        let path = format!("/recipes/R-500/versions/3/approve{kind}");
        let main = main_in(Lang::En, &fixture, &data, &path).await;
        assert!(
            main.contains("<p class=\"empty\">Versions 3 and 2 have the same parameters.</p>"),
            "{kind}: {main}"
        );
        assert!(!main.contains("has no change"), "{kind}: {main}");
        assert!(!main.contains("class=\"pager\""), "{kind}: {main}");
    }
}
