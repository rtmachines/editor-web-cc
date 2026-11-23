//! Tests of the counting pass of D3: `render_exact` and `RenderedForm::from_template_exact`
//! give a buffer with a capacity equal to its length (task 4.1, design D3, ADR-0223).
//!
//! Each test renders a form of at least 1 MiB, so a growth of the buffer by doubling shows.

use askama::Template;
use recipe_domain::audit::RequestId;
use uuid::Uuid;

use axum::response::Html;

use super::render_exact;
use crate::error::RenderedForm;
use crate::i18n::Lang;
use crate::pages::Texts;
use crate::pages::parameters::big_grid_kit::{MIN_BYTES, big_fragment, big_page};
use crate::test_support::state;

/// The id of the request that the tests pass.
fn request_id() -> RequestId {
    RequestId::from(Uuid::nil())
}

#[test]
fn render_exact_a_page_of_1_mib_has_a_capacity_equal_to_its_length() {
    let state = state();
    let page = big_page(&state);

    let Html(text) = render_exact(&page, request_id()).unwrap();

    assert!(text.len() >= MIN_BYTES, "length {}", text.len());
    assert_eq!(text.capacity(), text.len(), "capacity and length");
}

#[test]
fn render_exact_keeps_the_bytes_of_render() {
    let state = state();
    let page = big_page(&state);
    let plain = page.render().unwrap();

    let Html(exact) = render_exact(&page, request_id()).unwrap();

    assert!(exact == plain, "the exact page differs from the plain page");
}

#[test]
fn render_exact_a_form_of_1_mib_has_a_capacity_equal_to_its_length() {
    let state = state();
    let t = Texts::new(state.bundles(), Lang::En);
    let fragment = big_fragment(t, true);

    let rendered = RenderedForm::from_template_exact(&fragment, request_id()).unwrap();

    assert!(rendered.as_str().len() >= MIN_BYTES);
    assert_eq!(rendered.capacity(), rendered.as_str().len());
}

#[test]
fn render_exact_a_form_keeps_the_bytes_of_from_template() {
    let state = state();
    let t = Texts::new(state.bundles(), Lang::En);
    let fragment = big_fragment(t, true);
    let plain = RenderedForm::from_template(&fragment, request_id()).unwrap();

    let exact = RenderedForm::from_template_exact(&fragment, request_id()).unwrap();

    assert!(exact.as_str() == plain.as_str(), "the forms differ");
}

#[test]
fn render_exact_a_form_without_the_out_of_band_attribute_is_refused() {
    let state = state();
    let t = Texts::new(state.bundles(), Lang::En);
    let fragment = big_fragment(t, false);

    let result = RenderedForm::from_template_exact(&fragment, request_id());

    assert!(matches!(
        result,
        Err(crate::error::WebError::Internal { .. })
    ));
}
