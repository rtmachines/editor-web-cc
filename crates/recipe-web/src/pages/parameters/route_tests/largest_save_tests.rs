//! Measurement of a valid save of the edit form of the fixture (task 5.7, design D3 and D6,
//! ADR-0223).
//!
//! The tests start after `nested_form_world`: a limit for each element at each array level and the
//! committed `row_version` 2. They send the edit form of the fixture to `/definition` as the
//! ProcessEngineer in `pl`, with a new text in every value leaf and a new description. The save
//! writes and commits the `row_version` 3. The first answer is the `303` of a request without
//! `HX-Request`. It has no render interval and builds no grid. The second answer is the HTMX
//! success. It holds the clean form out of band, and the form is the only grid that the request
//! builds. Each test pins the answer of the code after groups 3 and 4, so a pin is a regression
//! check and not a red test. Run the tests alone: `RUSTFLAGS="--cfg heap_measure"
//! CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked largest_save --
//! --ignored --test-threads=1`.

use std::time::{Duration, Instant};

use axum::http::StatusCode;
use recipe_app::recipe::DraftOutput;
use recipe_domain::recipe::{ParameterKey, RecipeVersionId};
use serde_json::Value;

use super::largest_form_tests::{
    KEY, LEAVES, REFERENCE, TYPE_LEVEL_MODE_ROWS, VERSION_ID, form_pairs, ldt_count,
    nested_form_world,
};
use super::measure_kit::{
    ENGINEER_SESSION, PEAK_SLACK, Pin, Rendered, check_status, count, definition_request,
    headers_of, loaded_of_post, report,
};
use crate::error::FormErrors;
use crate::pages::draft_reply::with_change_number;
use crate::pages::parameters::definition::check;
use crate::pages::parameters::edit::saved_body;
use crate::pages::parameters::fields::{FieldSet, read_body};
use crate::pages::parameters::form_view::{ParameterFormFragment, ParameterFormView, Place};
use crate::pages::parameters::grid::build_count::{builds, reset};
use crate::pages::parameters::page::{read_parameter, stored_shown};
use crate::pages::parameters::reply::load_shown;
use crate::pages::parameters::summary::{SummaryFragment, SummaryView};
use crate::pages::parameters::test_kit::form;
use crate::pages::parameters::translations::{TranslationsBase, TranslationsBaseFragment};
use crate::test_support::heap::{BodyDigest, peak_in, peak_of};
use crate::test_support::plant::{HTMX, POLISH, World};

/// The row version of the draft after `nested_form_world`: the seed has 1 and the save adds 1.
const OLD_ROW_VERSION: i64 = 2;
/// The row version of the draft after the save of these tests.
const NEW_ROW_VERSION: i64 = 3;
/// The mark of the leaf rows of the grid.
const LEAF_ROW: &str = "<tr><th scope=\"row\"";
/// The mark of the mode rows of the grid.
const MODE_ROW: &str = "<tr><td colspan=\"6\"><span class=\"grid-note\">";
/// The new English description that the save writes.
const NEW_DESCRIPTION: &str = "The saved edit form";
/// The number of the value texts that one step of the new texts spans.
const TEXT_STEPS: usize = 800;

/// What the answer without a render interval must have.
struct RedirectPin {
    /// The name of the answer in the printed line.
    name: &'static str,
    /// The length of the answer body in bytes.
    len: usize,
    /// The SHA-256 of the answer body, with the request id replaced.
    sha256: &'static str,
    /// The measured peak of the request. The pinned bound is this value plus [`PEAK_SLACK`].
    request_peak: usize,
}

/// Return the new value text of the leaf number `leaf`: the old text of the next leaf.
fn new_text(leaf: usize) -> String {
    let next = leaf.checked_add(1).unwrap();
    format!("LDT#2262-04-11-23:47:16.854775{:03}", next % TEXT_STEPS)
}

/// Return the body of the edit form of the fixture with a real change.
///
/// The body differs from `form_pairs` in `row_version`, `description_en`, and the text of every
/// value leaf. Each new text lies inside the limits of the leaf.
fn save_body() -> String {
    let mut next_leaf = 0_usize;
    let pairs: Vec<(String, String)> = form_pairs()
        .into_iter()
        .map(|(name, text)| {
            if name == "row_version" {
                (name, OLD_ROW_VERSION.to_string())
            } else if name == "description_en" {
                (name, NEW_DESCRIPTION.to_owned())
            } else if name.starts_with("value[") {
                let changed = new_text(next_leaf);
                next_leaf = next_leaf.checked_add(1).unwrap();
                (name, changed)
            } else {
                (name, text)
            }
        })
        .collect();
    assert_eq!(next_leaf, LEAVES);
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let body = form(&refs);
    let mut digest = BodyDigest::new("");
    digest.push(body.as_bytes());
    let (len, sha256) = digest.finish();
    report(&format!("save body: len {len} sha256 {sha256}"));
    body
}

/// Collect the leaf counts of the nested JSON `value` in the order of the leaves.
fn leaf_counts(value: &Value, counts: &mut Vec<i64>) {
    match value {
        Value::Array(items) => items.iter().for_each(|item| leaf_counts(item, counts)),
        other => counts.push(other.as_i64().unwrap()),
    }
}

/// Assert that the store holds the saved change: the `row_version`, the description, the values.
///
/// The stored parameter must keep its 16 384 limits.
fn assert_saved(world: &World) {
    let version = RecipeVersionId::from(VERSION_ID);
    let state = world.fixture.store.state();
    assert_eq!(state.version_row_version(version), Some(NEW_ROW_VERSION));
    let stored = state.parameters_of(version);
    assert_eq!(stored.len(), 1);
    let parameter = stored.first().unwrap();
    assert_eq!(parameter.description_en.to_string(), NEW_DESCRIPTION);
    assert_eq!(parameter.limits.len(), LEAVES);
    let mut counts = Vec::new();
    leaf_counts(parameter.value.as_json(), &mut counts);
    let expected: Vec<i64> = (0..LEAVES).map(|leaf| ldt_count(&new_text(leaf))).collect();
    assert_eq!(
        counts, expected,
        "the stored values are not the saved texts"
    );
}

/// Return the path of the parameter page that `Location` names.
fn page_location() -> String {
    format!("/recipes/{REFERENCE}/versions/1/parameters/{KEY}")
}

/// Build the success body of the HTMX save on a loaded view and measure the join.
///
/// Solves the problem that the handler `save` has no sync step that a test can call alone. The
/// function repeats `saved` of `edit.rs` up to the templates outside the interval: the summary, the
/// clean form from the stored parameter, and the base block. Inside `peak_in` it calls
/// `saved_body`, the only part of `saved` that renders. The caller compares the length and the
/// hash with the answer of the router, so a drift between this copy and `edit.rs` fails the test.
async fn measure_saved_render(
    world: &World,
    headers: &[(&str, &str)],
    body: &str,
) -> (Rendered, String) {
    let state = &world.fixture.state;
    let loaded = loaded_of_post(world, ENGINEER_SESSION, headers).await;
    let ctx = loaded.ctx(state);
    let sent = read_body(body.as_bytes(), FieldSet::Parameter).unwrap();
    let checked = check(&sent);
    assert!(checked.lines.is_empty());
    let wanted = checked.type_expr.clone().unwrap();
    assert_eq!(sent.value_type(), Some(&wanted));
    let definition = checked.definition.unwrap();
    let view = load_shown(ctx, Some(wanted), None).await.unwrap();
    assert!(view.line.is_none());
    let form = view.form;

    let key = ParameterKey::from(KEY);
    let output = DraftOutput {
        version: RecipeVersionId::from(VERSION_ID),
        row_version: NEW_ROW_VERSION,
    };
    let t = ctx.texts();
    let sentence = t.with_arg("parameter-saved", "tag", &definition.tag);
    let toast = with_change_number(t, &sentence, &output);
    let summary = SummaryFragment {
        t,
        summary: SummaryView::saved(key, &definition, toast),
    };
    let stored = read_parameter(ctx).await.unwrap();
    let mut clean = stored_shown(ctx, &stored).unwrap();
    clean.texts.row_version = output.row_version.to_string();
    let csrf = loaded.request.csrf_token();
    let place = (clean, Place::OutOfBand);
    let view = ParameterFormView::new(t, &form, place, csrf, FormErrors::default());
    let clean_form = ParameterFormFragment { t, form: view };
    let base = TranslationsBase {
        name_en: definition.name_en.to_string(),
        description_en: definition.description_en.to_string(),
    };
    let base = TranslationsBaseFragment { t, base };

    let started = Instant::now();
    let (body, peak) =
        peak_in(|| saved_body((&summary, &clean_form, &base), loaded.request.request_id()));
    let time = started.elapsed();
    let body = body.map_err(|_error| "not shown").unwrap().0;

    let (len, sha256) = loaded.digest_of(&body);
    let rendered = Rendered {
        len,
        sha256,
        peak,
        time,
    };
    (rendered, body)
}

/// Assert the clean form of the HTMX success: out of band, 16 384 leaf rows, 32 mode rows.
fn assert_clean_form(page: &str) {
    let shown = |needle: &str| count(page, needle);
    let new_first = format!("value=\"{}\"", new_text(0));
    report(&format!(
        "save answer: len {} leaf rows {} mode buttons {} mode rows {} oob {} description {} \
         first new text {} forms {}",
        page.len(),
        shown(LEAF_ROW),
        shown("name=\"limit_mode\""),
        shown(MODE_ROW),
        shown("hx-swap-oob=\"true\""),
        shown(NEW_DESCRIPTION),
        shown(&new_first),
        shown("<form"),
    ));
    assert_eq!(shown(LEAF_ROW), LEAVES);
    assert_eq!(shown("name=\"limit_mode\""), TYPE_LEVEL_MODE_ROWS);
    assert_eq!(shown(MODE_ROW), TYPE_LEVEL_MODE_ROWS);
    assert!(shown("hx-swap-oob=\"true\"") >= 1);
    assert!(shown(NEW_DESCRIPTION) >= 1);
    assert!(shown(&new_first) >= 1);
}

/// Print the line of the answer without a render interval and check it against `pin`.
fn check_redirect(
    pin: &RedirectPin,
    measured: &crate::test_support::heap::Measured,
    time: Duration,
) {
    report(&format!(
        "{}: len {} sha256 {} | request peak {} | render peak none | allowance none | \
         render time none | request time {} ms",
        pin.name,
        measured.len,
        measured.sha256,
        measured.peak,
        time.as_millis(),
    ));
    assert_eq!(measured.len, pin.len, "{}", pin.name);
    assert_eq!(measured.sha256, pin.sha256, "{}", pin.name);
    let bound = pin.request_peak.checked_add(PEAK_SLACK).unwrap();
    assert!(
        measured.peak <= bound,
        "{}: request peak {} exceeds {bound}",
        pin.name,
        measured.peak
    );
}

/// The `303` answer of the save, without `HX-Request`.
const REDIRECT_PL: RedirectPin = RedirectPin {
    name: "largest_save_redirect_pl",
    len: 0,
    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    request_peak: 345_978_140,
};

/// The HTMX success of the save.
const HTMX_PL: Pin = Pin {
    name: "largest_save_htmx_pl",
    len: 22_515_938,
    sha256: "66e972ad8bd9ef1f8ba90f8ef5f8c3dd6ab1fd1e2eba4cec158b189244c6b8d9",
    request_peak: 345_980_004,
    render_peak: 22_516_579,
};

#[tokio::test]
#[ignore = "sends a body of 11.6 MB, takes about 7 minutes, and peaks at 346 MB: run alone"]
async fn largest_save_redirect_pl() {
    let (world, row_version) = nested_form_world().await;
    assert_eq!(row_version, OLD_ROW_VERSION);
    let body = save_body();
    let headers = headers_of(&world.engineer.cookie, POLISH, &[]);

    reset();
    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || {
        definition_request(&headers, &world.engineer.csrf, &body)
    })
    .await;
    let request_time = started.elapsed();
    let grids = builds();

    assert_eq!(measured.status, StatusCode::SEE_OTHER);
    let location = measured.headers.get("location").unwrap();
    assert_eq!(location.to_str().unwrap(), page_location());
    assert_saved(&world);
    assert_eq!(grids, 0, "a save that answers 303 builds no grid");
    check_redirect(&REDIRECT_PL, &measured, request_time);
}

#[tokio::test]
#[ignore = "sends a body of 11.6 MB, gets an answer of 22.5 MB, takes about 10 minutes: run alone"]
async fn largest_save_htmx_pl() {
    let (world, row_version) = nested_form_world().await;
    assert_eq!(row_version, OLD_ROW_VERSION);
    let body = save_body();
    let headers = headers_of(&world.engineer.cookie, POLISH, &[HTMX]);

    reset();
    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || {
        definition_request(&headers, &world.engineer.csrf, &body)
    })
    .await;
    let request_time = started.elapsed();
    let grids = builds();

    assert_eq!(measured.status, StatusCode::OK);
    let trigger = measured.headers.get("hx-trigger").unwrap();
    assert_eq!(
        trigger.to_str().unwrap(),
        format!("{{\"rowVersion\": {NEW_ROW_VERSION}}}")
    );
    assert_saved(&world);
    assert_eq!(grids, 1, "only the clean form of the answer has a grid");
    let (rendered, page) = measure_saved_render(&world, &headers, &body).await;
    assert_clean_form(&page);
    drop(page);
    check_status(&HTMX_PL, StatusCode::OK, &measured, &rendered, request_time);
}
