//! Measurement of the largest answer at the cap of a bound text (task 2.4, ADR-0225 (t)).
//!
//! The type is `ARRAY[1..16384] OF INT`. The form holds one row `[*]` with a `min`, a `max`, and a
//! `max_length` of `LIMIT_TEXT_MAX` bytes of `"` each. The reader accepts such a form, because each
//! text is at the cap. The answer to `fields=1` keeps the row `[*]`. The answer to the switch of
//! `limit_mode` clones the three texts to each of the 16 384 element rows (`moved_to` of
//! `grid/limits.rs`). Both answers show the form again and write nothing. Each test pins the answer
//! of the code, so a pin is a regression check and not a red test. Run the tests alone:
//! `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web
//! --all-features --locked largest_cap -- --ignored --test-threads=1`.

use std::time::{Duration, Instant};

use axum::body::to_bytes;
use axum::http::StatusCode;
use recipe_domain::recipe::RecipeVersionId;

use super::largest_form_tests::{LEAVES, VERSION_ID, largest_form_world};
use super::measure_kit::{
    ANSWER_LIMIT, ENGINEER_SESSION, Pin, Rendered, check_answer, count, definition_request,
    headers_of, loaded_of_post, report,
};
use crate::pages::parameters::definition::check;
use crate::pages::parameters::fields::{FieldSet, read_body};
use crate::pages::parameters::reply::{load_shown, sent_shown, show_form};
use crate::pages::parameters::test_kit::form;
use crate::settings::LIMIT_TEXT_MAX;
use crate::test_support::call_bounded;
use crate::test_support::heap::{BodyDigest, peak_in, peak_of};
use crate::test_support::plant::{HTMX, POLISH, World};

/// The type of the fixture: 16 384 elements of `INT`.
const CAP_TYPE: &str = "ARRAY[1..16384] OF INT";
/// The row version of the draft of the world: the seed has 1 and no test saves.
const SEED_ROW_VERSION: i64 = 1;
/// The mark of the leaf rows of the grid.
const LEAF_ROW: &str = "<tr><th scope=\"row\"";
/// The mark of the mode rows of the grid.
const MODE_ROW: &str = "<tr><td colspan=\"6\"><span class=\"grid-note\">";
/// The mark of the rows `[*]` of the grid: the row of the limit of all elements of an array level.
const ALL_ROW: &str = "<tr class=\"row-all\">";
/// The number of mode rows: one for the one type level.
const MODE_ROWS: usize = 1;
/// The path of the one type level, which the button `limit_mode` sends.
const TYPE_LEVEL: &str = "[*]";
/// The number of values of `INT` that the fixture cycles through.
const VALUE_CYCLE: usize = 100;

/// The switch that the test sends.
#[derive(Clone, Copy)]
enum Switch {
    /// The request of `fields=1`: the row `[*]` stays.
    All,
    /// The button `limit_mode` of the one type level: the row `[*]` becomes 16 384 element rows.
    Elements,
}

/// Return the text of a bound at the cap: `LIMIT_TEXT_MAX` bytes of `"`.
fn cap_text() -> String {
    "\"".repeat(LIMIT_TEXT_MAX)
}

/// Return the body of the edit form of the fixture with `switch`.
///
/// The body holds the type, one value for each of the 16 384 leaves, the row `[*]` with three
/// texts at the cap, and one more field: `fields` or `limit_mode`.
fn cap_body(switch: Switch) -> String {
    let text = cap_text();
    let mut pairs: Vec<(String, String)> = [
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "BIG"),
        ("name_en", "Big"),
        ("description_en", "The largest edit form"),
        ("s7_address", ""),
        ("ordinal", "0"),
        ("type", CAP_TYPE),
        ("value_type", CAP_TYPE),
    ]
    .iter()
    .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
    .collect();
    for leaf in 0..LEAVES {
        let number = leaf.checked_add(1).unwrap();
        let value = leaf.checked_rem(VALUE_CYCLE).unwrap();
        pairs.push((format!("value[{number}]"), value.to_string()));
    }
    for (name, value) in [
        ("limits[0].leaf_path", TYPE_LEVEL),
        ("limits[0].min", text.as_str()),
        ("limits[0].max", text.as_str()),
        ("limits[0].max_length", text.as_str()),
    ] {
        pairs.push((name.to_owned(), value.to_owned()));
    }
    match switch {
        Switch::All => pairs.push(("fields".to_owned(), "1".to_owned())),
        Switch::Elements => pairs.push(("limit_mode".to_owned(), TYPE_LEVEL.to_owned())),
    }
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let body = form(&refs);
    let mut digest = BodyDigest::new("");
    digest.push(body.as_bytes());
    let (len, sha256) = digest.finish();
    report(&format!("cap body: len {len} sha256 {sha256}"));
    body
}

/// Assert the answer of `switch`: the status, the leaf rows, the mode rows, and the rows `[*]`.
///
/// The form of `fields=1` keeps one row `[*]`. The switch to the element rows leaves no row `[*]`.
async fn assert_answer(world: &World, headers: &[(&str, &str)], body: &str, switch: Switch) {
    let request = definition_request(headers, &world.engineer.csrf, body);
    let reply = call_bounded(&world.fixture.router(), request, ANSWER_LIMIT).await;
    let page = &reply.body;
    let shown = |needle: &str| count(page, needle);
    report(&format!(
        "cap answer: status {} len {} leaf rows {} rows [*] {} mode buttons {} mode rows {} \
         form rows {}",
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
    assert_eq!(shown("name=\"limit_mode\""), MODE_ROWS);
    assert_eq!(shown(MODE_ROW), MODE_ROWS);
    match switch {
        Switch::All => assert_eq!(shown(ALL_ROW), 1),
        Switch::Elements => assert_eq!(shown(ALL_ROW), 0),
    }
}

/// Measure the sync build of the answer on a loaded view.
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
    assert_eq!(sent.wants_fields(), matches!(switch, Switch::All));
    assert_eq!(
        sent.limit_mode().is_some(),
        matches!(switch, Switch::Elements)
    );
    let wanted = match switch {
        Switch::All => check(&sent).type_expr,
        Switch::Elements => sent.value_type().cloned(),
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

/// Send `switch` to `BIG` and check the answer against `pin`.
async fn measure_cap(pin: &Pin, switch: Switch, extra: &[(&str, &str)]) {
    let world = largest_form_world();
    let body = cap_body(switch);
    let headers = headers_of(&world.engineer.cookie, POLISH, extra);
    assert_answer(&world, &headers, &body, switch).await;
    let before = world.fixture.snapshot();

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
        Some(SEED_ROW_VERSION)
    );
    let rendered = measure_render(&world, &headers, &body, switch).await;

    report(&format!(
        "{}: request peak {} | render peak {} | render time {} ms",
        pin.name,
        measured.peak,
        rendered.peak,
        rendered.time.as_millis(),
    ));
    check_answer(pin, &measured, &rendered, request_time);
}

/// The answer to `fields=1` with the row `[*]`, HTMX answer.
const ALL_HTMX_PL: Pin = Pin {
    name: "largest_cap_all_htmx_pl",
    len: 48_004_340,
    sha256: "be872cd5a8119186b1685f538bfac7ccbb890a2a64a1ffd0a33067399cd7f3d9",
    request_peak: 85_535_890,
    render_peak: 82_273_386,
};

/// The answer to the switch of `limit_mode` to the element rows, HTMX answer.
const SWITCH_HTMX_PL: Pin = Pin {
    name: "largest_cap_switch_htmx_pl",
    len: 56_677_591,
    sha256: "052f398015a6865c57fd72d77c208993f90fd079e02d48aeaea35a301e2356e2",
    request_peak: 88_295_509,
    render_peak: 85_032_983,
};

#[tokio::test]
#[ignore = "builds an answer of about 48 MB: run alone with --ignored"]
async fn largest_cap_all_htmx_pl() {
    measure_cap(&ALL_HTMX_PL, Switch::All, &[HTMX]).await;
}

#[tokio::test]
#[ignore = "builds an answer of about 57 MB: run alone with --ignored"]
async fn largest_cap_switch_htmx_pl() {
    measure_cap(&SWITCH_HTMX_PL, Switch::Elements, &[HTMX]).await;
}
