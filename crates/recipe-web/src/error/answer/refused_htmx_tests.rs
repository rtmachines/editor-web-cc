//! Tests of the memory of the HTMX refused answer (task 4.4, design D3, ADR-0223).

use axum::http::StatusCode;
use recipe_app::ports::NonEmpty;
use recipe_domain::audit::RequestId;
use uuid::Uuid;

use super::{ErrorContext, TOAST_CLEAR, body_of};
use crate::error::{FieldError, FieldReason, RenderedForm, WebError};
use crate::i18n::Lang;
use crate::pages::Texts;
use crate::pages::parameters::big_grid_kit::{MIN_BYTES, big_fragment};
use crate::state::WebState;
use crate::test_support::heap::peak_in;
use crate::test_support::state;

/// The slack that the peak may add to the length of the body.
const SLACK: usize = 64 * 1024;

fn request_id() -> RequestId {
    RequestId::from(Uuid::nil())
}

/// Build the refused form of at least 1 MiB before the measured interval.
fn refused_form_error(texts: Texts<'_>) -> WebError {
    let form = RenderedForm::from_template_exact(&big_fragment(texts, true), request_id()).unwrap();
    WebError::Fields {
        lines: NonEmpty::new(FieldError::bad_request("tag", FieldReason::Tag)),
        form: Some(form),
        failure: None,
    }
}

fn htmx_context(state: &WebState) -> ErrorContext<'_> {
    ErrorContext {
        request_id: request_id(),
        lang: Lang::En,
        htmx: true,
        retry: None,
        state,
    }
}

fn body_for(error: &WebError, state: &WebState) -> String {
    let ctx = htmx_context(state);
    body_of(error, error.code(), StatusCode::UNPROCESSABLE_ENTITY, &ctx)
}

#[test]
fn refused_htmx_body_of_a_form_of_one_mebibyte_has_a_capacity_equal_to_its_length() {
    let state = state();
    let texts = Texts::new(state.bundles(), Lang::En);
    let error = refused_form_error(texts);

    let body = body_for(&error, &state);

    assert!(body.len() >= MIN_BYTES, "body length {}", body.len());
    assert!(body.ends_with(TOAST_CLEAR));
    assert_eq!(
        body.capacity(),
        body.len(),
        "capacity {} differs from length {}",
        body.capacity(),
        body.len()
    );
}

#[test]
#[ignore = "measurement test: run alone with --ignored --test-threads=1"]
fn refused_htmx_peak_stays_within_the_body_for_a_form_of_one_mebibyte() {
    let state = state();
    let texts = Texts::new(state.bundles(), Lang::En);
    let error = refused_form_error(texts);

    let (body, peak) = peak_in(|| body_for(&error, &state));

    assert!(body.len() >= MIN_BYTES, "body length {}", body.len());
    let bound = body.len() + SLACK;
    assert!(
        peak <= bound,
        "peak {peak} exceeds bound {bound} (body length {} + {SLACK}, capacity {})",
        body.len(),
        body.capacity()
    );
}
