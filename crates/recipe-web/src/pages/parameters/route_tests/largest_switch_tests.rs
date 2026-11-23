//! Measurement of the switch answers of the fixture (task 5.5, design D2 and D6, ADR-0223).
//!
//! The tests start after `nested_form_world`: a limit for each element at each array level and the
//! committed `row_version` 2. They send the edit form of the fixture to `/definition` as the
//! ProcessEngineer. One switch joins the innermost type level with `limit_mode`. The other switch
//! is `fields=1`. Both answers show the form again and write nothing. Each test pins the answer of
//! the code after groups 3 and 4, so a pin is a regression check and not a red test. Run the tests
//! alone: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web
//! --all-features --locked largest_switch -- --ignored --test-threads=1`.

use std::time::{Duration, Instant};

use axum::body::to_bytes;
use axum::http::StatusCode;
use recipe_domain::recipe::RecipeVersionId;

use super::largest_form_tests::{
    LEAVES, TYPE_LEVEL_MODE_ROWS, VERSION_ID, form_pairs, nested_form_world,
};
use super::measure_kit::{
    ANSWER_LIMIT, ENGINEER_SESSION, Pin, Rendered, change_a_sum, check_answer, count,
    definition_request, headers_of, loaded_of_post, report,
};
use crate::pages::parameters::definition::check;
use crate::pages::parameters::fields::{FieldSet, read_body};
use crate::pages::parameters::reply::{load_shown, sent_shown, show_form};
use crate::pages::parameters::test_kit::form;
use crate::test_support::call_bounded;
use crate::test_support::heap::{BodyDigest, peak_in, peak_of};
use crate::test_support::plant::{HTMX, POLISH, World};

/// The row version of the draft after `nested_form_world`: the seed has 1 and the save adds 1.
const SAVED_ROW_VERSION: i64 = 2;
/// The number of array levels of the type of the fixture. Each one is a type level.
const TYPE_LEVELS: usize = 32;
/// The mark of the leaf rows of the grid.
const LEAF_ROW: &str = "<tr><th scope=\"row\"";
/// The mark of the mode rows of the grid.
const MODE_ROW: &str = "<tr><td colspan=\"6\"><span class=\"grid-note\">";
/// The mark of the rows `[*]` of the grid: the row of the limit of all elements of an array level.
const ALL_ROW: &str = "<tr class=\"row-all\">";

/// The switch that the test sends.
#[derive(Clone, Copy)]
enum Switch {
    /// The button `limit_mode` of the innermost type level.
    Join,
    /// The request of `fields=1`.
    Fields,
}

/// Return the path of the innermost type level: one `[*]` for each of the 32 type levels.
fn innermost_level() -> String {
    "[*]".repeat(TYPE_LEVELS)
}

/// Return the body of the edit form of the fixture with the committed `row_version` and `switch`.
///
/// The body holds every field of `form_pairs` and one more field: `limit_mode` or `fields`.
fn switch_body(switch: Switch) -> String {
    let mut pairs: Vec<(String, String)> = form_pairs()
        .into_iter()
        .map(|(name, text)| {
            if name == "row_version" {
                (name, SAVED_ROW_VERSION.to_string())
            } else {
                (name, text)
            }
        })
        .collect();
    match switch {
        Switch::Join => pairs.push(("limit_mode".to_owned(), innermost_level())),
        Switch::Fields => pairs.push(("fields".to_owned(), "1".to_owned())),
    }
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let body = form(&refs);
    let mut digest = BodyDigest::new("");
    digest.push(body.as_bytes());
    let (len, sha256) = digest.finish();
    report(&format!("switch body: len {len} sha256 {sha256}"));
    body
}

/// Assert the answer of a switch: the status, the leaf rows, the mode rows, and the rows `[*]`.
///
/// A joined innermost type level has 16 384 rows `[*]`, one for each element group. A request with
/// `HX-Request` gets the form fragment, and a request without it gets the page of the form.
async fn assert_switch(world: &World, headers: &[(&str, &str)], body: &str, switch: Switch) {
    let request = definition_request(headers, &world.engineer.csrf, body);
    let reply = call_bounded(&world.fixture.router(), request, ANSWER_LIMIT).await;
    let page = &reply.body;
    let shown = |needle: &str| count(page, needle);
    let joined_paths = shown("[*]\"><");
    report(&format!(
        "switch answer: status {} len {} leaf rows {} rows [*] {} mode buttons {} mode rows {} \
         hidden paths ending in [*] {joined_paths} form rows {}",
        reply.status,
        page.len(),
        shown(LEAF_ROW),
        shown(ALL_ROW),
        shown("name=\"limit_mode\""),
        shown(MODE_ROW),
        shown("<form"),
    ));
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(shown(LEAF_ROW), LEAVES);
    assert_eq!(shown("name=\"limit_mode\""), TYPE_LEVEL_MODE_ROWS);
    assert_eq!(shown(MODE_ROW), TYPE_LEVEL_MODE_ROWS);
    match switch {
        Switch::Join => assert_eq!(shown(ALL_ROW), LEAVES),
        Switch::Fields => assert_eq!(shown(ALL_ROW), 0),
    }
}

/// Measure the sync build of the switch answer on a loaded view.
///
/// Solves the problem that the handler `save` has no sync step that a test can call alone. The
/// function reads the body and runs the form query outside the interval, as `again` of
/// `prepare.rs` does. Inside `peak_in` it repeats the sync tail of `again`: `sent_shown` and
/// `show_form`. The caller compares the length and the hash with the answer of the router, so a
/// drift between this copy and `prepare.rs` fails the test.
async fn measure_render(
    world: &World,
    headers: &[(&str, &str)],
    body: &str,
    switch: Switch,
) -> Rendered {
    let state = &world.fixture.state;
    let loaded = loaded_of_post(world, ENGINEER_SESSION, headers).await;
    let ctx = loaded.ctx(state);
    let sent = read_body(body.as_bytes(), FieldSet::Parameter).unwrap();
    assert_eq!(sent.wants_fields(), matches!(switch, Switch::Fields));
    assert_eq!(sent.limit_mode().is_some(), matches!(switch, Switch::Join));
    let wanted = match switch {
        Switch::Join => sent.value_type().cloned(),
        Switch::Fields => check(&sent).type_expr,
    };
    let view = load_shown(ctx, wanted, sent.value_type().cloned())
        .await
        .unwrap();
    assert!(view.line.is_none());

    let started = Instant::now();
    let (response, peak) = peak_in(|| {
        let shown = sent_shown(ctx, &view.form, &sent, view.previewed.as_ref());
        show_form(ctx, &view.form, shown)
    });
    let time = started.elapsed();
    let response = response.map_err(|_error| "not shown").unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), ANSWER_LIMIT).await.unwrap();
    let (len, sha256) = loaded.digest_of(std::str::from_utf8(&bytes).unwrap());
    Rendered {
        len,
        sha256,
        peak,
        time,
    }
}

/// Send the switch to `BIG` and check the answer against `pin`.
async fn measure_switch(pin: &Pin, switch: Switch, extra: &[(&str, &str)]) {
    let (world, row_version) = nested_form_world().await;
    assert_eq!(row_version, SAVED_ROW_VERSION);
    let body = switch_body(switch);
    let headers = headers_of(&world.engineer.cookie, POLISH, extra);
    assert_switch(&world, &headers, &body, switch).await;
    let before = world.fixture.snapshot();
    let stored_before = world
        .fixture
        .store
        .state()
        .version_row_version(RecipeVersionId::from(VERSION_ID));

    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || {
        definition_request(&headers, &world.engineer.csrf, &body)
    })
    .await;
    let request_time: Duration = started.elapsed();
    assert_eq!(world.fixture.snapshot(), before);
    assert_eq!(
        world
            .fixture
            .store
            .state()
            .version_row_version(RecipeVersionId::from(VERSION_ID)),
        stored_before
    );
    assert_eq!(stored_before, Some(SAVED_ROW_VERSION));
    let rendered = measure_render(&world, &headers, &body, switch).await;

    report(&format!(
        "{}: request peak {} | render peak {} | allowance {} | render time {} ms | \
         sum of change A {}",
        pin.name,
        measured.peak,
        rendered.peak,
        measured.peak.saturating_sub(rendered.peak),
        rendered.time.as_millis(),
        change_a_sum(rendered.peak),
    ));
    check_answer(pin, &measured, &rendered, request_time);
}

/// The switch of the innermost type level, page without `HX-Request`.
const JOIN_PAGE_PL: Pin = Pin {
    name: "largest_switch_join_page_pl",
    len: 31_841_238,
    sha256: "0856fba95c1a1bbb86607cba98a7819166bac1fca559954343b4eabb62fdb357",
    request_peak: 218_480_985,
    render_peak: 127_402_702,
};

/// The switch of the innermost type level, HTMX answer.
const JOIN_HTMX_PL: Pin = Pin {
    name: "largest_switch_join_htmx_pl",
    len: 31_838_777,
    sha256: "5fb71ebed8d3cb0282db5a036536b74288e8dc11b4097fd4764bcbfa27a4f748",
    request_peak: 218_480_985,
    render_peak: 127_402_702,
};

/// The switch of `fields=1`, page without `HX-Request`.
const FIELDS_PAGE_PL: Pin = Pin {
    name: "largest_switch_fields_page_pl",
    len: 22_518_742,
    sha256: "2493e8ea459f09e65560812efc3328bd28aa9f0b2b9801758315093d050de3b5",
    request_peak: 208_481_018,
    render_peak: 106_203_306,
};

/// The switch of `fields=1`, HTMX answer.
const FIELDS_HTMX_PL: Pin = Pin {
    name: "largest_switch_fields_htmx_pl",
    len: 22_516_281,
    sha256: "f35c9dd91f990988848b56b62640ed9e8a7a5668569d6d340800338a2f87ca1c",
    request_peak: 218_481_018,
    render_peak: 106_203_306,
};

#[tokio::test]
#[ignore = "builds a body of 12 MB and an answer of about 32 MB: run alone with --ignored"]
async fn largest_switch_join_page_pl() {
    measure_switch(&JOIN_PAGE_PL, Switch::Join, &[]).await;
}

#[tokio::test]
#[ignore = "builds a body of 12 MB and an answer of about 32 MB: run alone with --ignored"]
async fn largest_switch_join_htmx_pl() {
    measure_switch(&JOIN_HTMX_PL, Switch::Join, &[HTMX]).await;
}

#[tokio::test]
#[ignore = "builds a body of 12 MB and an answer of about 32 MB: run alone with --ignored"]
async fn largest_switch_fields_page_pl() {
    measure_switch(&FIELDS_PAGE_PL, Switch::Fields, &[]).await;
}

#[tokio::test]
#[ignore = "builds a body of 12 MB and an answer of about 32 MB: run alone with --ignored"]
async fn largest_switch_fields_htmx_pl() {
    measure_switch(&FIELDS_HTMX_PL, Switch::Fields, &[HTMX]).await;
}
