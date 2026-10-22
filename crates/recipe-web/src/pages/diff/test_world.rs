//! The helpers of the diff page tests. Test code.
//!
//! The helpers solve one problem: the diff page tests send the same requests and count the same
//! sections in several files. The helpers that the approval preview tests share too live in
//! `pages::test_helpers`.

use axum::http::StatusCode;

use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::main_of;
use crate::test_support::recipes::approval::ApprovalData;
use crate::test_support::{Fixture, Reply, get_page};

/// The diff of the draft version 2 of `R-500` against the approved version 1.
pub(super) const R500_DIFF: &str = "/recipes/R-500/versions/2/diff/1";

/// Return the count of the sections in `main`.
pub(super) fn section_count(main: &str) -> usize {
    main.matches("<section class=\"diff-section\"").count()
}

/// Send `path` as the Administrator `admin` in English. Check the status and the HTML.
/// Return `main`.
pub(super) async fn admin_main(fixture: &Fixture, data: &ApprovalData, path: &str) -> String {
    let reply: Reply = get_page(fixture, Some(&data.admin), "en", path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    main_of(&reply.body).to_owned()
}
