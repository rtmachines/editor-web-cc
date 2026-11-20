//! Tests of the parameter forms on a machine with the STRING encoding `Utf8` (ADR-0202 (m)).

use axum::http::StatusCode;

use super::test_kit::{assert_panel_error, post_htmx};
use crate::test_support::recipes::utf8_world::{LABEL_KEY, REFERENCE, utf8_world};

/// The path of the value form of `LABEL`, the `STRING(4)` of the draft 1 of `R-410`.
fn label_path() -> String {
    format!("/recipes/{REFERENCE}/versions/1/parameters/{LABEL_KEY}/value")
}

#[tokio::test]
async fn adr_0202_value_form_names_the_length_in_bytes() {
    let world = utf8_world();
    let before = world.fixture.snapshot();

    // `ééé` has 3 characters and 6 UTF-8 bytes. `STRING(4)` holds 4 bytes.
    let refused = post_htmx(
        &world,
        &label_path(),
        &[("row_version", "1"), ("value", "ééé")],
    )
    .await;

    assert_panel_error(&refused, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-005");
    assert!(
        refused
            .body
            .contains("a string of at most 4 UTF-8 bytes without U+0000"),
        "{}",
        refused.body
    );
    assert_eq!(world.fixture.snapshot(), before);

    // `éé` has 4 UTF-8 bytes, so the same form saves it.
    let saved = post_htmx(
        &world,
        &label_path(),
        &[("row_version", "1"), ("value", "éé")],
    )
    .await;

    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
}
