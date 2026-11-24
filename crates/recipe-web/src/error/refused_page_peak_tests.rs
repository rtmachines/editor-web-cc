//! Tests of the memory of the refused page (task 4.3, design D3, ADR-0223).

use askama::Template;
use axum::body::to_bytes;
use axum::http::StatusCode;
use recipe_app::ports::NonEmpty;
use recipe_domain::audit::RequestId;
use uuid::Uuid;

use super::answer::{ErrorContext, answer};
use super::{FieldError, FieldReason, RenderedForm, WebError};
use crate::i18n::Lang;
use crate::layout::SessionFrame;
use crate::pages::Texts;
use crate::pages::parameters::big_grid_kit::{MIN_BYTES, big_fragment};
use crate::test_support::heap::peak_in;
use crate::test_support::state;

/// The slack that the peak may add to the length of the page.
const SLACK: usize = 64 * 1024;

fn request_id() -> RequestId {
    RequestId::from(Uuid::nil())
}

/// Build the refused form of at least 1 MiB with the session parts of the layout.
fn refused_form_error(texts: Texts<'_>) -> WebError {
    let form = RenderedForm::from_template_exact(&big_fragment(texts, true), request_id())
        .unwrap()
        .with_frame(Some(SessionFrame::for_test("c0ffee00c0ffee00", "engineer")));
    WebError::Fields {
        lines: NonEmpty::new(FieldError::bad_request("tag", FieldReason::Tag)),
        form: Some(form),
        failure: None,
    }
}

#[tokio::test]
#[ignore = "measurement test: run alone with --ignored --test-threads=1"]
async fn refused_page_peak_stays_within_the_page_for_a_form_of_one_mebibyte() {
    let state = state();
    let texts = Texts::new(state.bundles(), Lang::En);
    let error = refused_form_error(texts);
    let ctx = ErrorContext {
        request_id: request_id(),
        lang: Lang::En,
        htmx: false,
        retry: None,
        state: &state,
    };

    let (response, peak) = peak_in(|| answer(&error, &ctx));

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let page = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(page.len() >= MIN_BYTES, "page length {}", page.len());
    assert!(!page.windows(13).any(|w| w == b"hx-swap-oob=\""));
    let bound = page.len() + SLACK;
    assert!(
        peak <= bound,
        "peak {peak} exceeds bound {bound} (page length {} + {SLACK})",
        page.len()
    );
}

#[tokio::test]
async fn refused_page_of_a_form_of_one_mebibyte_holds_the_plain_form_in_the_layout() {
    let state = state();
    let texts = Texts::new(state.bundles(), Lang::En);
    let error = refused_form_error(texts);
    let ctx = ErrorContext {
        request_id: request_id(),
        lang: Lang::En,
        htmx: false,
        retry: None,
        state: &state,
    };

    let response = answer(&error, &ctx);

    let page = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let page = String::from_utf8(page.to_vec()).unwrap();
    let form = big_fragment(texts, true).render().unwrap();
    let body = form.split_once('>').unwrap().1;
    assert!(page.starts_with("<!DOCTYPE html>"));
    let tag = "<form id=\"parameter-new-form\" method=\"post\"";
    assert!(page.contains(tag), "no plain form tag");
    assert!(page.contains(body), "the page lacks the body of the form");
    assert!(!page.contains("hx-swap-oob"));
    assert!(!page.contains("hx-post"));
}
