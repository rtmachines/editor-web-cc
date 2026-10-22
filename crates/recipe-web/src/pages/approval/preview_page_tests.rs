//! Tests of the pages of a large preview and of the failures of the diff stage (task 10.1,
//! ADR-0050 (e), (g), (h), spec "Pages of a large diff", "Review state of a paged preview").

use axum::http::StatusCode;
use recipe_app::blocking::{AppSettings, Blocking};
use recipe_domain::identity::Role;

use crate::pages::test_helpers::{assert_clean_html, assert_error, part_of, section, small_pages};
use crate::test_support::plant::main_of;
use crate::test_support::recipes::approval::ApprovalData;
use crate::test_support::{Fixture, Reply, get_page};

/// The preview of the draft version 2 of `R-500`.
const R500_PREVIEW: &str = "/recipes/R-500/versions/2/approve";

/// The count of diff parameters on each page of these tests.
const ONE_PER_PAGE: u32 = 1;

/// Send `path` as the Administrator in English.
async fn admin_get(fixture: &Fixture, data: &ApprovalData, path: &str) -> Reply {
    get_page(fixture, Some(&data.admin), "en", path).await
}

/// The default timeouts of the blocking tasks.
fn normal() -> Blocking {
    AppSettings::default().blocking
}

#[tokio::test]
async fn approve_preview_links_each_cut_section_to_its_own_pages_with_the_review_state() {
    let (fixture, data) = small_pages(ONE_PER_PAGE, normal()).await;
    let reply = admin_get(&fixture, &data, R500_PREVIEW).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    let rv = format!("rv={}", data.r500_v2_row_version);
    for kind in ["limits", "values", "added", "removed"] {
        // Askama writes `&` as `&#38;`. The summary also links a section that is not on the page,
        // with the name of the section as its text, so the test finds the link by its label.
        let link = format!("href=\"{R500_PREVIEW}?kind={kind}&#38;{rv}\">Pages of the section ");
        assert!(main.contains(&link), "{link}: {main}");
    }
    assert!(!main.contains("id=\"diff-values\""), "{main}");
}

#[tokio::test]
async fn approve_preview_section_page_has_the_pager_of_its_section() {
    let (fixture, data) = small_pages(ONE_PER_PAGE, normal()).await;
    let rv = data.r500_v2_row_version;
    let path = format!("{R500_PREVIEW}?kind=limits&page=2&rv={rv}");
    let reply = admin_get(&fixture, &data, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let limits = section(main, "limits");
    assert!(limits.contains(">SPEED_TABLE</a>"), "{limits}");
    let pager = part_of(main, "<nav class=\"pager\"", "</nav>");
    assert!(
        pager.contains("aria-label=\"Pages of the section Process limits\""),
        "{pager}"
    );
    assert!(pager.contains("page=1"), "{pager}");
    assert!(pager.contains("page=3"), "{pager}");
    assert!(pager.contains(&format!("rv={rv}")), "{pager}");
    assert!(pager.contains("aria-current=\"page\""), "{pager}");
    assert!(main.contains("<ul class=\"diff-summary\">"), "{main}");
}

/// A page with `kind` shows no cut links, because its pager leads to the other pages of the
/// section (design D4, design-system.md 7, test plan row B19).
#[tokio::test]
async fn approve_preview_section_page_shows_no_cut_links() {
    let (fixture, data) = small_pages(ONE_PER_PAGE, normal()).await;
    let rv = data.r500_v2_row_version;
    let path = format!("{R500_PREVIEW}?kind=limits&page=2&rv={rv}");
    let reply = admin_get(&fixture, &data, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("<nav class=\"pager\""), "{main}");
    assert!(!main.contains("class=\"link-list\""), "{main}");
    assert!(!main.contains("\">Pages of the section "), "{main}");
}

#[tokio::test]
async fn approve_preview_page_after_the_end_shows_the_notice_and_the_first_page() {
    let (fixture, data) = small_pages(ONE_PER_PAGE, normal()).await;
    let path = format!("{R500_PREVIEW}?kind=limits&page=9");
    let reply = admin_get(&fixture, &data, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("Page 9 is after the last page."), "{main}");
    let rv = data.r500_v2_row_version;
    assert!(
        main.contains(&format!("href=\"{R500_PREVIEW}?kind=limits&#38;rv={rv}\"")),
        "{main}"
    );
    // The page after the end has no pager, but its `kind` still hides the cut links (design D4).
    assert!(!main.contains("class=\"link-list\""), "{main}");
}

/// Without `kind`, the link to the first page also keeps `rv` (design D3).
#[tokio::test]
async fn approve_preview_page_after_the_end_without_a_section_keeps_the_review_state() {
    let (fixture, data) = small_pages(ONE_PER_PAGE, normal()).await;
    let path = format!("{R500_PREVIEW}?page=9");
    let reply = admin_get(&fixture, &data, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("Page 9 is after the last page."), "{main}");
    let rv = data.r500_v2_row_version;
    assert!(
        main.contains(&format!("href=\"{R500_PREVIEW}?rv={rv}\"")),
        "{main}"
    );
}

/// The seed of `seed_approval_recipes` runs `ApproveVersion`, which fails with these settings. So
/// the test reads the draft version 3 of `R-100`, which `seed_recipes` writes into the store.
#[tokio::test]
async fn approve_preview_answers_the_failures_of_the_diff_stage() {
    let cases = [
        (
            Blocking::elapsed(),
            StatusCode::SERVICE_UNAVAILABLE,
            "E-WEB-009",
        ),
        (
            Blocking::panicked(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "E-WEB-011",
        ),
    ];
    for (blocking, status, code) in cases {
        let fixture = Fixture::with_app_settings(AppSettings {
            blocking,
            ..AppSettings::default()
        });
        fixture.seed_recipes();
        let engineer = fixture.seed_session(2, Role::ProcessEngineer, false);
        let path = "/recipes/R-100/versions/3/approve";
        let reply = get_page(&fixture, Some(&engineer), "en", path).await;
        assert_error(&reply, status, code, code);
    }
}
