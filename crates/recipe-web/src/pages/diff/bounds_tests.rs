//! Tests of the bounds of the diff page: `page.diff` parameters over all sections, `kind`, the
//! cut links, the pager, and the page after the end (task 12.1, design D4 and D10 "Diff bounds",
//! ADR-0050 (e)).

use recipe_app::blocking::AppSettings;

use super::test_world::{R500_DIFF, admin_main, section_count};
use crate::pages::test_helpers::{part_of, section, small_pages};
use crate::test_support::Fixture;
use crate::test_support::recipes::approval::ApprovalData;

/// One parameter on each page. The seed holds more than one entry in the limits section.
async fn one_entry() -> (Fixture, ApprovalData) {
    small_pages(1, AppSettings::default().blocking).await
}

#[tokio::test]
async fn diff_page_holds_at_most_page_diff_parameters_over_all_sections() {
    let (fixture, data) = one_entry().await;
    for page in 1..=6 {
        let path = format!("{R500_DIFF}?page={page}");
        let main = admin_main(&fixture, &data, &path).await;
        assert_eq!(section_count(&main), 1, "page {page}: {main}");
        assert_eq!(
            main.matches("/parameters/").count(),
            1,
            "page {page}: {main}"
        );
    }
}

#[tokio::test]
async fn diff_page_without_a_section_fills_the_sections_in_their_order() {
    let (fixture, data) = one_entry().await;
    let first = admin_main(&fixture, &data, R500_DIFF).await;
    assert!(section(&first, "limits").contains(">OLD_ONLY<"), "{first}");
    let second = admin_main(&fixture, &data, &format!("{R500_DIFF}?page=2")).await;
    assert!(
        section(&second, "limits").contains(">SPEED_TABLE<"),
        "{second}"
    );
    assert!(!second.contains(">OLD_ONLY<"), "{second}");
}

#[tokio::test]
async fn diff_page_links_each_cut_section_to_its_own_pages() {
    let (fixture, data) = one_entry().await;
    let main = admin_main(&fixture, &data, R500_DIFF).await;
    for kind in ["limits", "values", "added", "removed"] {
        let link = format!("href=\"{R500_DIFF}?kind={kind}\">Pages of the section ");
        assert!(main.contains(&link), "{link}: {main}");
    }
    assert!(!main.contains("id=\"diff-values\""), "{main}");
}

/// A page with `kind` shows no cut links, because its pager leads to the other pages of the
/// section (design D4, ADR-0050 (e)).
#[tokio::test]
async fn diff_page_of_a_section_has_its_pager_and_no_cut_links() {
    let (fixture, data) = one_entry().await;
    let path = format!("{R500_DIFF}?kind=limits&page=2");
    let main = admin_main(&fixture, &data, &path).await;
    assert!(
        section(&main, "limits").contains(">SPEED_TABLE</a>"),
        "{main}"
    );
    assert_eq!(section_count(&main), 1, "{main}");
    let pager = part_of(&main, "<nav class=\"pager\"", "</nav>");
    assert!(
        pager.contains("aria-label=\"Pages of the section Process limits\""),
        "{pager}"
    );
    assert!(pager.contains("page=1"), "{pager}");
    assert!(pager.contains("page=3"), "{pager}");
    assert!(pager.contains("aria-current=\"page\""), "{pager}");
    assert!(!main.contains("class=\"link-list\""), "{main}");
    assert!(!main.contains("\">Pages of the section "), "{main}");
}

#[tokio::test]
async fn diff_page_after_the_end_shows_the_notice_and_the_first_page() {
    let (fixture, data) = one_entry().await;
    let main = admin_main(&fixture, &data, &format!("{R500_DIFF}?kind=limits&page=9")).await;
    assert!(main.contains("Page 9 is after the last page."), "{main}");
    assert!(
        main.contains(&format!("href=\"{R500_DIFF}?kind=limits\"")),
        "{main}"
    );
    assert!(!main.contains("class=\"pager\""), "{main}");
    assert!(!main.contains("class=\"link-list\""), "{main}");
    assert!(!main.contains("class=\"empty\""), "{main}");
}

#[tokio::test]
async fn diff_page_after_the_end_without_a_section_links_the_first_page() {
    let (fixture, data) = one_entry().await;
    let main = admin_main(&fixture, &data, &format!("{R500_DIFF}?page=99")).await;
    assert!(main.contains("Page 99 is after the last page."), "{main}");
    assert!(main.contains(&format!("href=\"{R500_DIFF}\"")), "{main}");
}
