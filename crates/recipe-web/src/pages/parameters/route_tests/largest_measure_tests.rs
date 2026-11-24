//! Measurement of the answers of the largest valid parameter (tasks 5.1 and 5.2, design D6,
//! ADR-0223).
//!
//! The tests run after `nested_form_world`. They pin the answer of the code after groups 3 and 4,
//! so each pin is a regression check and not a red test. Run them alone:
//! `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web
//! --all-features --locked largest_page -- --ignored --test-threads=1`. Replace the filter
//! `largest_page` with `largest_read` for task 5.2.

use std::time::Instant;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use recipe_app::recipe::{DraftOutput, ParameterView};
use recipe_domain::recipe::RecipeVersionId;
use serde_json::Value;

use super::largest_form_tests::{
    LEAVES, PAGE_LIMIT, TYPE_LEVEL_MODE_ROWS, VERSION_ID, ldt_count, leaf_path, nested_form_world,
};
use super::measure_kit::{
    ENGINEER_SESSION, Pin, Rendered, check_answer, headers_of, loaded_of, page_path,
};
use crate::error::FormErrors;
use crate::layout::{Layout, NavTarget};
use crate::pages::draft_reply::with_change_number;
use crate::pages::parameters::form_view::{
    GridTexts, ParameterFormView, Place, Shown, address_lines, fault_errors, grid_of,
};
use crate::pages::parameters::grid::GridValuesFragment;
use crate::pages::parameters::page::{read_parameter, stored_shown};
use crate::pages::parameters::remove::toast_html;
use crate::pages::parameters::reply::{Ctx, load, parameter_title, version_text};
use crate::pages::parameters::summary::{ParameterPage, RemoveView, SummaryView};
use crate::pages::parameters::test_kit::form;
use crate::pages::parameters::translations::{RowSource, TranslationsBlock, translation_rows};
use crate::pages::parameters::value::values_body;
use crate::pages::render::render_exact;
use crate::test_support::heap::{peak_in, peak_of};
use crate::test_support::plant::{ENGLISH, HTMX, HTMX_CONTENT, POLISH, World};
use crate::test_support::{get_bounded, request};

/// The number of the seeded user of the Operator session of `world` (`seed_session(3, ..)`).
///
/// The Operator holds only the read permission, so the page of the Operator is read-only.
const OPERATOR_SESSION: u8 = 3;

/// The row version of the draft after `nested_form_world`: the seed has 1 and the save adds 1.
const SAVED_ROW_VERSION: i64 = 2;

/// Build the `GET` request of the page with the headers.
pub(super) fn page_request(headers: &[(&str, &str)]) -> Request<Body> {
    let mut builder = request(Method::GET, &page_path());
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    builder.body(Body::empty()).unwrap()
}

/// Build the translation rows of the stored parameter, as `translations_of` of `page.rs` does.
fn translations_of(ctx: Ctx<'_>, view: &ParameterView, editable: bool) -> TranslationsBlock {
    let source = RowSource {
        parameter_href: page_path(),
        csrf_token: ctx.request.csrf_token(),
        row_version: view.detail.row_version.to_string(),
        editable,
    };
    let langs = &ctx.state.settings().translation_langs;
    TranslationsBlock {
        name_en: view.parameter.name_en.to_string(),
        description_en: view.parameter.description_en.to_string(),
        rows: translation_rows(&source, langs, &view.parameter.translations),
    }
}

/// Assert the status, the 16 384 leaf rows, and the 32 mode rows of the edit page.
async fn assert_rows(world: &World, headers: &[(&str, &str)]) {
    let reply = get_bounded(&world.fixture.router(), &page_path(), headers, PAGE_LIMIT).await;

    assert_eq!(reply.status, StatusCode::OK);
    let leaf_rows = reply.body.matches("<tr><th scope=\"row\"").count();
    let mode_buttons = reply.body.matches("name=\"limit_mode\"").count();
    let mode_rows = reply
        .body
        .matches("<tr><td colspan=\"6\"><span class=\"grid-note\">")
        .count();
    assert_eq!(leaf_rows, LEAVES, "{} bytes", reply.body.len());
    assert_eq!(mode_buttons, TYPE_LEVEL_MODE_ROWS);
    assert_eq!(mode_rows, TYPE_LEVEL_MODE_ROWS);
}

/// Measure the sync build of the row model and the render of the edit page on a loaded view.
///
/// Solves the problem that `parameter_page` has no sync step that a test can call alone. Its tail
/// after the two queries is inline in `parameter_page` and `form_or_grid` of `page.rs`. This
/// function loads the view and the form with the same queries, outside the interval. It then
/// repeats the sync calls of `page.rs` inside `peak_in`: `stored_shown`, `fault_errors`,
/// `ParameterFormView::new`, `Layout::for_session`, the page literal, and `render_exact`.
/// The caller compares the length and the hash with the answer of the router, so a drift between
/// this copy and `page.rs` fails the test.
pub(super) async fn measure_render(world: &World, headers: &[(&str, &str)]) -> Rendered {
    let state = &world.fixture.state;
    let loaded = loaded_of(world, ENGINEER_SESSION, headers).await;
    let ctx = loaded.ctx(state);
    let view = read_parameter(ctx).await.unwrap();
    let form = load(ctx, ctx.target.form_purpose(), None, None)
        .await
        .unwrap();

    let started = Instant::now();
    let (html, peak) = peak_in(|| {
        let t = ctx.texts();
        let shown = stored_shown(ctx, &view).unwrap();
        let errors = shown
            .grid
            .as_ref()
            .map(|grid| fault_errors(t, &view.rule_faults, grid))
            .unwrap_or_default();
        let shown = Shown {
            grid: shown.grid.map(|grid| grid.with_errors(&errors)),
            ..shown
        };
        let edit = ParameterFormView::new(
            t,
            &form,
            (shown, Place::Fragment),
            ctx.request.csrf_token(),
            errors,
        );
        let title = parameter_title(ctx, &view.parameter.tag);
        let layout = Layout::for_session(t, ctx.request, title, Some(NavTarget::Recipes)).unwrap();
        let page = ParameterPage {
            layout,
            t,
            summary: Some(SummaryView::stored(&view).with_address_lines(Vec::new())),
            form: Some(edit),
            notice: None,
            values: None,
            faults: FormErrors::default(),
            translations: Some(translations_of(ctx, &view, true)),
            remove: Some(RemoveView {
                action: format!("{}/delete", page_path()),
                tag: view.parameter.tag.to_string(),
                csrf_token: ctx.request.csrf_token(),
                row_version: view.detail.row_version.to_string(),
            }),
            version_href: ctx.target.version_href(),
            version_text: version_text(ctx),
        };
        render_exact(&page, ctx.request.request_id()).unwrap()
    });
    let time = started.elapsed();

    let (len, sha256) = loaded.digest_of(&html.0);
    Rendered {
        len,
        sha256,
        peak,
        time,
    }
}

/// Measure the `GET` of the edit page with the language and the extra headers.
///
/// The answer must match `pin`.
async fn measure_page(pin: &Pin, lang: (&str, &str), extra: &[(&str, &str)]) {
    let (world, row_version) = nested_form_world().await;
    assert_eq!(row_version, SAVED_ROW_VERSION);
    let headers = headers_of(&world.engineer.cookie, lang, extra);
    assert_rows(&world, &headers).await;

    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || page_request(&headers)).await;
    let request_time = started.elapsed();
    let rendered = measure_render(&world, &headers).await;

    check_answer(pin, &measured, &rendered, request_time);
}

/// The full page in `en`.
const PAGE_EN: Pin = Pin {
    name: "largest_page_en",
    len: 22_569_867,
    sha256: "991cd75f4ba221611535ac9c030258efe389a55f05b78489a893785992409852",
    request_peak: 194_881_859,
    render_peak: 106_201_524,
};

/// The full page in `pl`.
const PAGE_PL: Pin = Pin {
    name: "largest_page_pl",
    len: 22_521_311,
    sha256: "1b8855e386884083942e766a6665d08e90fd462ce536cfe0b15cd678fd433ca7",
    request_peak: 194_881_859,
    render_peak: 106_201_524,
};

/// The fragment in `pl` with `HX-Request: true` and `HX-Target: content`.
const FRAGMENT_PL: Pin = Pin {
    name: "largest_page_fragment_pl",
    len: 22_518_784,
    sha256: "108e46f54ddd7e62de384c4a1acb6b26213df0f59cc15ff43dc412cd7fca6474",
    request_peak: 194_881_866,
    render_peak: 106_201_524,
};

#[tokio::test]
#[ignore = "builds a page of about 23 MB: run alone with --ignored --test-threads=1"]
async fn largest_page_en() {
    measure_page(&PAGE_EN, ENGLISH, &[]).await;
}

#[tokio::test]
#[ignore = "builds a page of about 23 MB: run alone with --ignored --test-threads=1"]
async fn largest_page_pl() {
    measure_page(&PAGE_PL, POLISH, &[]).await;
}

#[tokio::test]
#[ignore = "builds a page of about 23 MB: run alone with --ignored --test-threads=1"]
async fn largest_page_fragment_pl() {
    measure_page(&FRAGMENT_PL, POLISH, &HTMX_CONTENT).await;
}

/// Assert the status, the 16 384 leaf rows, and the mode rows of the read-only page.
///
/// ADR-0223 (i) says that the read-only grid shows the mode text of each type level, with no
/// button and no notice. A mode row is a row with one cell and the text only.
async fn assert_read_rows(world: &World, headers: &[(&str, &str)]) {
    let reply = get_bounded(&world.fixture.router(), &page_path(), headers, PAGE_LIMIT).await;

    assert_eq!(reply.status, StatusCode::OK);
    let leaf_rows = reply.body.matches("<tr><th scope=\"row\">").count();
    let mode_rows = reply
        .body
        .matches("<tr><td colspan=\"6\"><span class=\"grid-note\">")
        .count();
    assert_eq!(leaf_rows, LEAVES, "{} bytes", reply.body.len());
    assert_eq!(mode_rows, TYPE_LEVEL_MODE_ROWS);
    assert_eq!(reply.body.matches("name=\"limit_mode\"").count(), 0);
    // The layout holds three hidden inputs: the language switch and the logout form. No other
    // input and no select stands in the read-only page.
    let inputs = reply.body.matches("<input").count();
    assert_eq!(inputs, reply.body.matches("<input type=\"hidden\"").count());
    assert_eq!(reply.body.matches("<select").count(), 0);
}

/// Measure the sync build of the read-only grid and the render of the page of a reader.
///
/// Solves the problem of `measure_render` for the branch of `form_or_grid` and `parameter_page`
/// that a reader takes. The copy repeats `stored_grid`, `fault_errors`, `address_lines`, and the
/// page literal with `form: None`, inside `peak_in`. The view exists before the interval.
async fn measure_read_render(world: &World, headers: &[(&str, &str)]) -> Rendered {
    let state = &world.fixture.state;
    let loaded = loaded_of(world, OPERATOR_SESSION, headers).await;
    let ctx = loaded.ctx(state);
    let view = read_parameter(ctx).await.unwrap();
    assert!(view.tree.is_some());

    let started = Instant::now();
    let (html, peak) = peak_in(|| {
        let t = ctx.texts();
        let texts = GridTexts::stored(&view).unwrap();
        let tag = view.parameter.tag.to_string();
        let grid = grid_of(t, texts, state.settings().form_array_chunk, &tag);
        let faults = fault_errors(t, &view.rule_faults, &grid);
        let address = address_lines(t, &view.rule_faults, &grid);
        let values = grid.with_errors(&faults);
        let title = parameter_title(ctx, &view.parameter.tag);
        let layout = Layout::for_session(t, ctx.request, title, Some(NavTarget::Recipes)).unwrap();
        let page = ParameterPage {
            layout,
            t,
            summary: Some(SummaryView::stored(&view).with_address_lines(address)),
            form: None,
            notice: None,
            values: Some(values),
            faults,
            translations: Some(translations_of(ctx, &view, false)),
            remove: None,
            version_href: ctx.target.version_href(),
            version_text: version_text(ctx),
        };
        render_exact(&page, ctx.request.request_id()).unwrap()
    });
    let time = started.elapsed();

    let (len, sha256) = loaded.digest_of(&html.0);
    Rendered {
        len,
        sha256,
        peak,
        time,
    }
}

/// The read-only page of `BIG` for the Operator in `pl`.
const READ_PAGE_PL: Pin = Pin {
    name: "largest_read_page_pl",
    len: 6_331_619,
    sha256: "114dbf384b1b6af8fcef1a5db9e354e5c77e0501ffaeeae53876a97199a71b00",
    request_peak: 164_015_093,
    render_peak: 106_200_912,
};

#[tokio::test]
#[ignore = "builds a page of about 23 MB: run alone with --ignored --test-threads=1"]
async fn largest_read_page_pl() {
    let (world, row_version) = nested_form_world().await;
    assert_eq!(row_version, SAVED_ROW_VERSION);
    let headers = headers_of(&world.operator.cookie, POLISH, &[]);
    assert_read_rows(&world, &headers).await;

    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || page_request(&headers)).await;
    let request_time = started.elapsed();
    let rendered = measure_read_render(&world, &headers).await;

    check_answer(&READ_PAGE_PL, &measured, &rendered, request_time);
}

/// Return the new value text of the leaf number `leaf`. It differs from the saved text.
///
/// The saved text of `nested_form_world` ends in `leaf % 800`, so the next number differs, stays
/// below the maximum of the limit, and keeps the length of a canonical `LDT` text.
fn new_value_text(leaf: usize) -> String {
    let next = leaf.checked_add(1).unwrap() % 800;
    format!("LDT#2262-04-11-23:47:16.854775{next:03}")
}

/// Return the pairs of the value route: the committed `row_version` and the new value leaves.
fn value_pairs(row_version: i64) -> Vec<(String, String)> {
    let mut pairs = vec![("row_version".to_owned(), row_version.to_string())];
    pairs.extend(
        (0..LEAVES).map(|leaf| (format!("value{}", leaf_path(leaf)), new_value_text(leaf))),
    );
    pairs
}

/// Build the HTMX `POST` of the value route. The body is built inside the measured interval.
fn value_request(
    headers: &[(&str, &str)],
    csrf: &str,
    pairs: &[(String, String)],
) -> Request<Body> {
    let mut builder = request(Method::POST, &format!("{}/value", page_path()))
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("x-csrf-token", csrf);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    builder.body(Body::from(form(&refs))).unwrap()
}

/// Collect the numbers of the leaves of a nested array value in the order of the leaves.
fn leaves_of(value: &Value, out: &mut Vec<i64>) {
    match value {
        Value::Array(items) => items.iter().for_each(|item| leaves_of(item, out)),
        leaf => out.push(leaf.as_i64().unwrap()),
    }
}

/// Return the leaves of the stored value of `BIG` in the world.
fn stored_leaves(world: &World) -> Vec<i64> {
    let stored = world
        .fixture
        .store
        .state()
        .parameters_of(RecipeVersionId::from(VERSION_ID));
    let parameter = stored.first().unwrap();
    assert_eq!(parameter.limits.len(), LEAVES);
    let mut leaves = Vec::new();
    leaves_of(parameter.value.as_json(), &mut leaves);
    leaves
}

/// Measure the render of the answer of a saved value: `values_body` on a loaded fragment.
///
/// Solves the problem that the handler `set_value` has no sync step that a test can call alone.
/// The function reads the committed view, builds the grid, the sentence, and the toast with the
/// same calls as `values_answer`, outside the interval, and measures only `values_body` inside
/// `peak_in`. The caller compares the length and the hash with the answer of the router.
async fn measure_save_render(
    world: &World,
    headers: &[(&str, &str)],
    row_version: i64,
) -> Rendered {
    let state = &world.fixture.state;
    let loaded = loaded_of(world, ENGINEER_SESSION, headers).await;
    let ctx = loaded.ctx(state);
    let view = read_parameter(ctx).await.unwrap();
    let t = ctx.texts();
    let tag = view.parameter.tag.to_string();
    let texts = GridTexts::stored(&view).unwrap();
    let grid = grid_of(t, texts, state.settings().form_array_chunk, &tag);
    let output = DraftOutput {
        version: RecipeVersionId::from(VERSION_ID),
        row_version,
    };
    let sentence = with_change_number(t, &t.with_arg("value-saved", "tag", &tag), &output);
    let request_id = ctx.request.request_id();
    let toast = toast_html(sentence, true, request_id).unwrap();
    let fragment = GridValuesFragment { t, view: &grid };

    let started = Instant::now();
    let (body, peak) = peak_in(|| values_body(&fragment, &toast, request_id).unwrap());
    let time = started.elapsed();

    let (len, sha256) = loaded.digest_of(&body.0);
    Rendered {
        len,
        sha256,
        peak,
        time,
    }
}

/// The HTMX answer of a value save on `/value` in `pl`.
const READ_SAVE_PL: Pin = Pin {
    name: "largest_read_save_pl",
    len: 6_327_677,
    sha256: "d8c249b5862fcc362eab2246b832ebce4420b1bae0fd23ecbdfb1bcf43b6470a",
    request_peak: 318_200_978,
    render_peak: 6_327_757,
};

#[tokio::test]
#[ignore = "builds an answer of about 22 MB: run alone with --ignored --test-threads=1"]
async fn largest_read_value_save_pl() {
    let (world, row_version) = nested_form_world().await;
    assert_eq!(row_version, SAVED_ROW_VERSION);
    let before = stored_leaves(&world);
    let pairs = value_pairs(row_version);
    let headers = headers_of(&world.engineer.cookie, POLISH, &[HTMX]);

    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || {
        value_request(&headers, &world.engineer.csrf, &pairs)
    })
    .await;
    let request_time = started.elapsed();

    assert_eq!(measured.status, StatusCode::OK);
    let committed = world
        .fixture
        .store
        .state()
        .version_row_version(RecipeVersionId::from(VERSION_ID))
        .unwrap();
    assert_eq!(committed, row_version.checked_add(1).unwrap());
    let trigger = measured
        .headers
        .get("hx-trigger")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(trigger, format!("{{\"rowVersion\": {committed}}}"));
    let after = stored_leaves(&world);
    let expected: Vec<i64> = (0..LEAVES)
        .map(|leaf| ldt_count(&new_value_text(leaf)))
        .collect();
    assert_eq!(after, expected);
    assert!(before.iter().zip(&after).all(|(old, new)| old != new));
    let rendered = measure_save_render(&world, &headers, committed).await;

    check_answer(&READ_SAVE_PL, &measured, &rendered, request_time);
}
