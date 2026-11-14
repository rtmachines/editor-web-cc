//! Tests of the notice answer after a committed change with a failed read (ADR-0163).

use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::Response;
use recipe_app::error::{AppError, StorageError};
use recipe_domain::audit::RequestId;
use uuid::Uuid;

use super::saved_notice;
use crate::error::WebError;
use crate::i18n::{Bundles, Lang};
use crate::pages::texts::Texts;
use crate::test_support::capture_logs;

const HREF: &str = "/recipes/R-100/versions/3";
const REQUEST_ID: u128 = 0x5a;

fn fault() -> WebError {
    WebError::App(AppError::Storage(StorageError::PoolTimeout))
}

fn answer(htmx: bool, lang: Lang, row_version: Option<i64>) -> Response {
    let bundles = Bundles::load().unwrap();
    let request_id = RequestId::from(Uuid::from_u128(REQUEST_ID));
    saved_notice(
        htmx,
        request_id,
        Texts::new(&bundles, lang),
        &fault(),
        HREF,
        row_version,
    )
    .unwrap()
}

fn header<'a>(response: &'a Response, name: &str) -> Option<&'a str> {
    response
        .headers()
        .get(name)
        .map(|value| value.to_str().unwrap())
}

async fn body(response: Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn saved_notice_htmx_answer_targets_toast_and_empties_error_panel() {
    let response = answer(true, Lang::En, None);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(header(&response, "hx-retarget"), Some("#toast"));
    assert_eq!(header(&response, "hx-reswap"), Some("innerHTML"));
    assert_eq!(header(&response, "hx-push-url"), Some("false"));
    assert_eq!(header(&response, "hx-trigger"), None);
    let html = body(response).await;
    assert!(
        html.contains("The change is saved. Reload the page."),
        "{html}"
    );
    assert!(
        html.contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>"),
        "{html}"
    );
}

#[tokio::test]
async fn saved_notice_htmx_answer_sets_row_version_when_the_command_gives_it() {
    let response = answer(true, Lang::En, Some(8));
    assert_eq!(header(&response, "hx-trigger"), Some("{\"rowVersion\": 8}"));
}

#[tokio::test]
async fn saved_notice_htmx_answer_uses_the_polish_text() {
    let html = body(answer(true, Lang::Pl, None)).await;
    assert!(
        html.contains("Zmiana jest zapisana. Załaduj stronę ponownie."),
        "{html}"
    );
}

#[test]
fn saved_notice_plain_request_gets_see_other_to_the_page() {
    let response = answer(false, Lang::En, Some(8));
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(header(&response, "location"), Some(HREF));
    assert_eq!(header(&response, "hx-trigger"), None);
}

#[test]
fn saved_notice_writes_one_warn_line_with_code_request_id_and_chain() {
    let (logs, _guard) = capture_logs();
    let _response = answer(true, Lang::En, None);
    let text = logs.text();
    let lines: Vec<&str> = text.lines().filter(|line| line.contains("WARN")).collect();
    assert_eq!(lines.len(), 1, "{text}");
    let line = lines.first().unwrap();
    assert!(line.contains("error_code=E-DB-004"), "{line}");
    assert!(
        line.contains(&Uuid::from_u128(REQUEST_ID).to_string()),
        "{line}"
    );
    assert!(line.contains("chain="), "{line}");
}
