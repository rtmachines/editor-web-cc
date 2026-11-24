//! Tests of the HTMX success answer of `/definition` and of the memory of its body (task 4.6,
//! design D3, ADR-0223).

use axum::http::StatusCode;
use axum::response::Html;
use recipe_domain::audit::RequestId;
use uuid::Uuid;

use super::big_grid_kit::{MIN_BYTES, big_fragment};
use super::edit::saved_body;
use super::form_view::ParameterFormFragment;
use super::summary::{SummaryFragment, SummaryView};
use super::test_kit::{SPEED_FIELDS, parameter_path, post_htmx};
use super::translations::{TranslationsBase, TranslationsBaseFragment};
use crate::i18n::Lang;
use crate::pages::Texts;
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::heap::{BodyDigest, peak_in};
use crate::test_support::recipes::SPEED_KEY;
use crate::test_support::state;

/// The SHA-256 of the HTMX success answer of `SPEED`, as hex.
///
/// The bytes are those of the code before task 4.6. The hash input replaces each copy of the
/// request id. Task 4.6 changes how the body joins its parts and must keep this value.
const SAVED_HTMX_SHA256: &str = "b1a659177bc5fd77ac1dcd88bc2fc026f5d83c5181bec7dfdf3ee4eea6c49149";

#[tokio::test]
async fn definition_saved_pin_of_the_htmx_success_answer() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    let reply = post_htmx(&world, &path, &SPEED_FIELDS).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("Parameter SPEED is saved."),
        "{}",
        reply.body
    );
    assert!(
        reply.body.contains("hx-swap-oob=\"true\""),
        "{}",
        reply.body
    );
    let mut digest = BodyDigest::new(&reply.header("x-request-id"));
    digest.push(reply.body.as_bytes());
    let (_len, sha256) = digest.finish();
    assert_eq!(sha256, SAVED_HTMX_SHA256, "{}", reply.body);
}

/// The slack that the peak may add to the length of the body.
const SLACK: usize = 64 * 1024;

/// Build the three parts of a saved body before the measured interval: the summary, a clean form
/// of at least 1 MiB, and the base block.
fn big_parts(
    t: Texts<'_>,
) -> (
    SummaryFragment<'_>,
    ParameterFormFragment<'_>,
    TranslationsBaseFragment<'_>,
) {
    let summary = SummaryFragment {
        t,
        summary: SummaryView {
            key: SPEED_KEY.to_string(),
            unit: "U1".to_owned(),
            tag: "BIG".to_owned(),
            name_en: "BIG".to_owned(),
            description_en: "A big parameter".to_owned(),
            type_text: "ARRAY[1..1200] OF INT".to_owned(),
            s7_address: None,
            ordinal: "0".to_owned(),
            toast: "Parameter BIG is saved.".to_owned(),
            address_lines: Vec::new(),
        },
    };
    let base = TranslationsBase {
        name_en: "BIG".to_owned(),
        description_en: "A big parameter".to_owned(),
    };
    (
        summary,
        big_fragment(t, true),
        TranslationsBaseFragment { t, base },
    )
}

fn request_id() -> RequestId {
    RequestId::from(Uuid::nil())
}

#[test]
fn definition_saved_body_of_a_form_of_one_mebibyte_has_a_capacity_equal_to_its_length() {
    let state = state();
    let t = Texts::new(state.bundles(), Lang::En);
    let (summary, form, base) = big_parts(t);

    let Html(body) = saved_body((&summary, &form, &base), request_id()).unwrap();

    assert!(body.len() >= MIN_BYTES, "body length {}", body.len());
    assert!(body.contains("Parameter BIG is saved."));
    assert!(
        body.contains("id=\"translations-base\""),
        "{}",
        &body[body.len() - 400..]
    );
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
fn saved_body_peak_stays_within_the_body_for_a_form_of_one_mebibyte() {
    let state = state();
    let t = Texts::new(state.bundles(), Lang::En);
    let (summary, form, base) = big_parts(t);

    let (body, peak) = peak_in(|| saved_body((&summary, &form, &base), request_id()).unwrap());

    let Html(body) = body;
    assert!(body.len() >= MIN_BYTES, "body length {}", body.len());
    let bound = body.len() + SLACK;
    assert!(
        peak <= bound,
        "peak {peak} exceeds bound {bound} (body length {} + {SLACK}, capacity {})",
        body.len(),
        body.capacity()
    );
}
