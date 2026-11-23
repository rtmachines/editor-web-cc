//! Tests of the memory of `with_form` (task 4.2, design D3, ADR-0223).

use askama::Template;
use axum::response::Html;
use recipe_domain::audit::RequestId;
use uuid::Uuid;

use super::with_form;
use crate::error::WebError;
use crate::i18n::Lang;
use crate::pages::Texts;
use crate::pages::parameters::big_grid_kit::big_fragment;
use crate::test_support::heap::peak_in;
use crate::test_support::state;

/// The slack that the peak may add to the capacity of the body.
const SLACK: usize = 64 * 1024;
/// The text before the form, like the summary of a success answer.
const SUMMARY: &str = "<p>saved</p>";

fn request_id() -> RequestId {
    RequestId::from(Uuid::nil())
}

#[test]
#[ignore = "measurement test: run alone with --ignored --test-threads=1"]
fn with_form_peak_stays_within_the_body_for_a_form_of_one_mebibyte() {
    let state = state();
    let t = Texts::new(state.bundles(), Lang::En);
    let form = big_fragment(t, true);
    let form_length = form.render().unwrap().len();
    let mut text = String::with_capacity(SUMMARY.len() + form_length);
    text.push_str(SUMMARY);
    let capacity = text.capacity();
    let body = Html(text);

    let (answer, peak) = peak_in(|| with_form(body, &form, request_id()).unwrap());

    let Html(answer) = answer;
    assert!(answer.len() >= 1024 * 1024, "form length {}", answer.len());
    assert_eq!(answer.capacity(), capacity, "the body grew");
    let bound = capacity + SLACK;
    assert!(
        peak <= bound,
        "peak {peak} exceeds bound {bound} (body capacity {capacity} + {SLACK})"
    );
}

#[test]
fn with_form_appends_the_form_after_the_body_without_a_change_of_its_bytes() {
    let state = state();
    let t = Texts::new(state.bundles(), Lang::En);
    let form = big_fragment(t, true);
    let expected = format!("{SUMMARY}{}", form.render().unwrap());

    let Html(answer) = with_form(Html(SUMMARY.to_owned()), &form, request_id()).unwrap();

    assert_eq!(answer, expected);
}

#[test]
fn with_form_checks_the_first_tag_of_the_form_and_not_the_start_of_the_body() {
    let state = state();
    let t = Texts::new(state.bundles(), Lang::En);
    let form = big_fragment(t, false);
    // The body holds the out-of-band attribute itself. Only the first tag of the form counts.
    let body = Html("<form id=\"x\" hx-swap-oob=\"true\"></form>".to_owned());

    let result = with_form(body, &form, request_id());

    assert!(matches!(result, Err(WebError::Internal { .. })));
}
