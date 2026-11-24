//! Measurement of the refused form of round 1 of the paused change (task 5.3) and of the refused
//! fixture form with a line under each value input (task 5.4, design D6, ADR-0223).
//!
//! The form body has 16 758 074 bytes and holds 4 600 000 `x` in `description_en`. The tests send
//! it to the scalar `BIG` of `largest_form_world`. The server refuses the form with a line under
//! `description_en`. Each test pins the answer of the code after groups 3 and 4, so a pin is a
//! regression check and not a red test. Run the tests alone:
//! `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web
//! --all-features --locked largest_refused -- --ignored --test-threads=1`.

use std::time::{Duration, Instant};

use axum::body::to_bytes;
use axum::http::StatusCode;
use recipe_app::ports::NonEmpty;
use recipe_app::recipe::{RetypeInput, UpdateParameterDefinition, UpdateParameterDefinitionInput};
use recipe_domain::recipe::{ParameterKey, ValueInput};

use super::largest_form_tests::{
    KEY, LEAVES, TYPE_LEVEL_MODE_ROWS, form_pairs, largest_form_world,
};
use super::measure_kit::{
    ANSWER_LIMIT, ENGINEER_SESSION, Pin, Rendered, change_a_sum, check_status, count,
    definition_request, headers_of, loaded_of, report,
};
use crate::error::{ErrorContext, answer};
use crate::pages::parameters::definition::check;
use crate::pages::parameters::fields::{FieldSet, read_body};
use crate::pages::parameters::reply::{command_lines, load_shown, refuse, sent_shown};
use crate::pages::parameters::test_kit::form;
use crate::test_support::call_bounded;
use crate::test_support::heap::{BodyDigest, peak_in, peak_of};
use crate::test_support::plant::{ENGLISH, HTMX, POLISH, World};

/// The number of `x` in `description_en` of the body.
pub(super) const PADDING: usize = 4_600_000;
/// The long prefix of `LDT` (ADR-0205). It is 11 bytes longer than `LDT#`.
const ALIAS_PREFIX: &str = "LDATE_AND_TIME#";
/// The length of the body in bytes (`design.md`, round 1 of the paused change).
pub(super) const BODY_BYTES: usize = 16_758_074;
/// The SHA-256 of the body in lowercase hexadecimal.
pub(super) const BODY_SHA256: &str =
    "bef1574257ef4baa65a9f61e4b96f981255100359efeedfee6d3a7b1077dd6bd";
/// Return `form_pairs` with `description_en` and the alias prefix.
///
/// Solves the problem that the round 1 body and the round 2 body need one alias map. Each `LDT`
/// text of a value, a minimum, and a maximum gets the prefix `LDATE_AND_TIME#`.
pub(super) fn aliased_pairs(description_en: &str) -> Vec<(String, String)> {
    form_pairs()
        .into_iter()
        .map(|(name, text)| {
            let aliased =
                name.starts_with("value[") || name.ends_with(".min") || name.ends_with(".max");
            match name.as_str() {
                "description_en" => (name, description_en.to_owned()),
                _other if aliased => (name, text.replacen("LDT#", ALIAS_PREFIX, 1)),
                _other => (name, text),
            }
        })
        .collect()
}

/// Return the pairs of the form of round 1: the alias form with the long text.
fn refused_pairs() -> Vec<(String, String)> {
    aliased_pairs(&"x".repeat(PADDING))
}

/// Return the body of round 1 and check its length and SHA-256.
fn refused_body() -> String {
    let pairs = refused_pairs();
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let body = form(&refs);
    let mut digest = BodyDigest::new("");
    digest.push(body.as_bytes());
    let (len, sha256) = digest.finish();
    assert_eq!(len, BODY_BYTES);
    assert_eq!(sha256, BODY_SHA256);
    body
}

/// Assert the refusal: the status, the code, the line, the leaf rows, and the mode rows.
///
/// ADR-0223 gives one mode row for each type level, so the refused grid has 32 of them. The leaf
/// rows are the 16 384 leaves of the sent value. A request with `HX-Request` gets the form out of
/// band, and a request without it gets the page with the plain form.
async fn assert_refusal(world: &World, headers: &[(&str, &str)], body: &str, htmx: bool) {
    let request = definition_request(headers, &world.engineer.csrf, body);
    let reply = call_bounded(&world.fixture.router(), request, ANSWER_LIMIT).await;
    let page = &reply.body;
    let shown = |needle: &str| count(page, needle);
    report(&format!(
        "refusal htmx {htmx}: status {} len {} leaf rows {} mode buttons {} mode rows {} oob {} \
         code E-WEB-008 {} description line {} form rows {}",
        reply.status,
        page.len(),
        shown("<tr><th scope=\"row\""),
        shown("name=\"limit_mode\""),
        shown("<tr><td colspan=\"6\"><span class=\"grid-note\">"),
        shown("hx-swap-oob=\"true\""),
        shown("E-WEB-008"),
        shown("id=\"description_en-error\""),
        shown("<form"),
    ));
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(shown("id=\"description_en-error\""), 1);
    assert!(shown("E-WEB-008") >= 1);
    assert_eq!(shown("<tr><th scope=\"row\""), LEAVES);
    assert_eq!(shown("name=\"limit_mode\""), TYPE_LEVEL_MODE_ROWS);
    assert_eq!(
        shown("<tr><td colspan=\"6\"><span class=\"grid-note\">"),
        TYPE_LEVEL_MODE_ROWS
    );
    assert_eq!(shown("hx-swap-oob=\"true\"") >= 1, htmx);
}

/// Measure the sync build of the refused answer on a loaded view.
///
/// Solves the problem that the handler `save` has no sync step that a test can call alone. The
/// function reads the body, checks the definition fields, and runs the form query outside the
/// interval. Inside `peak_in` it repeats the sync tail of `ready` in `prepare.rs`: the leaf texts,
/// the limit rows, `sent_shown`, and `refuse`. It then drops those values, as the handler does
/// when it returns the error, and calls `answer` of the layer `render_errors`. The caller compares
/// the length and the hash with the answer of the router, so a drift fails the test.
pub(super) async fn measure_render(
    world: &World,
    headers: &[(&str, &str)],
    body: &str,
) -> Rendered {
    let state = &world.fixture.state;
    let loaded = loaded_of(world, ENGINEER_SESSION, headers).await;
    let ctx = loaded.ctx(state);
    let sent = read_body(body.as_bytes(), FieldSet::Parameter).unwrap();
    let checked = check(&sent);
    let type_expr = checked.type_expr.clone().unwrap();
    assert_eq!(sent.value_type(), Some(&type_expr));
    let view = load_shown(ctx, Some(type_expr), None).await.unwrap();
    assert!(view.line.is_none());
    let errors = ErrorContext {
        request_id: ctx.request.request_id(),
        lang: ctx.request.lang(),
        htmx: ctx.request.is_htmx(),
        retry: None,
        state,
    };

    let started = Instant::now();
    let (response, peak) = peak_in(|| {
        let error = {
            let mut lines = checked.lines;
            lines.extend(view.line);
            let tree = view.form.preview.as_ref().unwrap();
            let value = sent.leaf_texts(tree).unwrap();
            let limits = sent.limit_list(None);
            assert!(limits.is_ok());
            let shown = sent_shown(ctx, &view.form, &sent, view.previewed.as_ref());
            let error = refuse(ctx, &view.form, shown, NonEmpty::try_from(lines).unwrap());
            drop((value, limits));
            error
        };
        answer(&error, &errors)
    });
    let time = started.elapsed();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let bytes = to_bytes(response.into_body(), ANSWER_LIMIT).await.unwrap();
    let (len, sha256) = loaded.digest_of(std::str::from_utf8(&bytes).unwrap());
    Rendered {
        len,
        sha256,
        peak,
        time,
    }
}

/// Send the body of round 1 to `BIG` and check the answer against `pin`.
async fn measure_refused(pin: &Pin, lang: (&str, &str), extra: &[(&str, &str)]) {
    let world = largest_form_world();
    let body = refused_body();
    let headers = headers_of(&world.engineer.cookie, lang, extra);
    assert_refusal(&world, &headers, &body, !extra.is_empty()).await;
    let before = world.fixture.snapshot();

    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || {
        definition_request(&headers, &world.engineer.csrf, &body)
    })
    .await;
    let request_time: Duration = started.elapsed();
    assert_eq!(world.fixture.snapshot(), before);
    let rendered = measure_render(&world, &headers, &body).await;

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
    check_status(
        pin,
        StatusCode::UNPROCESSABLE_ENTITY,
        &measured,
        &rendered,
        request_time,
    );
}

/// The page without `HX-Request` in `en`.
const REFUSED_PAGE_EN: Pin = Pin {
    name: "largest_refused_page_en",
    len: 27_708_942,
    sha256: "a15ffd16259bcb32fd1c89f281ec361f71aee609c1b2e4086b77aa6bb9dc95c3",
    request_peak: 232_947_212,
    render_peak: 173_268_053,
};

/// The page without `HX-Request` in `pl`.
const REFUSED_PAGE_PL: Pin = Pin {
    name: "largest_refused_page_pl",
    len: 27_660_433,
    sha256: "5d7f0a6ef3b9806a14f04c08675b38a5123d236b76577a2e459c4496afa2b807",
    request_peak: 232_947_212,
    render_peak: 173_268_053,
};

/// The HTMX answer in `en`.
const REFUSED_HTMX_EN: Pin = Pin {
    name: "largest_refused_htmx_en",
    len: 27_706_793,
    sha256: "e8f36d7c1bf45593d972b7103ef9dfca8ab6a3fa063097bf2b2db29a99cf4258",
    request_peak: 232_947_212,
    render_peak: 173_268_053,
};

/// The HTMX answer in `pl`.
const REFUSED_HTMX_PL: Pin = Pin {
    name: "largest_refused_htmx_pl",
    len: 27_658_249,
    sha256: "b97f39be81de685502b9b33083dbd1926deb5b40beabdb496be1a4efdd3a8089",
    request_peak: 232_947_212,
    render_peak: 173_268_053,
};

#[tokio::test]
#[ignore = "builds a body of 16 MB and an answer of about 28 MB: run alone with --ignored"]
async fn largest_refused_page_en() {
    measure_refused(&REFUSED_PAGE_EN, ENGLISH, &[]).await;
}

#[tokio::test]
#[ignore = "builds a body of 16 MB and an answer of about 28 MB: run alone with --ignored"]
async fn largest_refused_page_pl() {
    measure_refused(&REFUSED_PAGE_PL, POLISH, &[]).await;
}

#[tokio::test]
#[ignore = "builds a body of 16 MB and an answer of about 28 MB: run alone with --ignored"]
async fn largest_refused_htmx_en() {
    measure_refused(&REFUSED_HTMX_EN, ENGLISH, &[HTMX]).await;
}

#[tokio::test]
#[ignore = "builds a body of 16 MB and an answer of about 28 MB: run alone with --ignored"]
async fn largest_refused_htmx_pl() {
    measure_refused(&REFUSED_HTMX_PL, POLISH, &[HTMX]).await;
}

/// The bad text of a value leaf: the lowest `LDT` of the type, which is below the minimum of every
/// limit row. The text parses, so the command gives `E-RCP-004` for each of the 16 384 leaves.
const BELOW_MINIMUM: &str = "LDT#1970-01-01-00:00:00.000000000";
/// The code of the line under each value input.
const LINE_CODE: &str = "E-RCP-004";
/// The length of the body with the bad texts in bytes.
const LINES_BODY_BYTES: usize = 11_617_429;
/// The SHA-256 of the body with the bad texts in lowercase hexadecimal.
const LINES_BODY_SHA256: &str = "cf20788f138197f3f8396cf5b6fd378b9e6bbc931f5c974ae857cdb90a43e1a7";

/// Return the body of the fixture form with `BELOW_MINIMUM` in each value leaf.
fn lines_body() -> String {
    let pairs: Vec<(String, String)> = form_pairs()
        .into_iter()
        .map(|(name, text)| {
            let bad = name.starts_with("value[");
            (name, if bad { BELOW_MINIMUM.to_owned() } else { text })
        })
        .collect();
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let body = form(&refs);
    let mut digest = BodyDigest::new("");
    digest.push(body.as_bytes());
    let (len, sha256) = digest.finish();
    report(&format!("lines body: len {len} sha256 {sha256}"));
    assert_eq!(len, LINES_BODY_BYTES);
    assert_eq!(sha256, LINES_BODY_SHA256);
    body
}

/// Assert the refusal with a line under each value input.
///
/// The leaf rows are the 16 384 leaves, and the grid has one mode row for each type level. A
/// request with `HX-Request` gets the form out of band.
async fn assert_lines_refusal(world: &World, headers: &[(&str, &str)], body: &str, htmx: bool) {
    let request = definition_request(headers, &world.engineer.csrf, body);
    let reply = call_bounded(&world.fixture.router(), request, ANSWER_LIMIT).await;
    let page = &reply.body;
    let shown = |needle: &str| count(page, needle);
    let value_lines = shown("<div class=\"field-error\" id=\"value-");
    report(&format!(
        "lines refusal htmx {htmx}: status {} len {} leaf rows {} mode buttons {} mode rows {} \
         oob {} code {LINE_CODE} {} value lines {value_lines} form rows {}",
        reply.status,
        page.len(),
        shown("<tr><th scope=\"row\""),
        shown("name=\"limit_mode\""),
        shown("<tr><td colspan=\"6\"><span class=\"grid-note\">"),
        shown("hx-swap-oob=\"true\""),
        shown(LINE_CODE),
        shown("<form"),
    ));
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(value_lines, LEAVES);
    assert!(shown(LINE_CODE) >= LEAVES);
    assert_eq!(shown("<tr><th scope=\"row\""), LEAVES);
    assert_eq!(shown("name=\"limit_mode\""), TYPE_LEVEL_MODE_ROWS);
    assert_eq!(
        shown("<tr><td colspan=\"6\"><span class=\"grid-note\">"),
        TYPE_LEVEL_MODE_ROWS
    );
    assert_eq!(shown("hx-swap-oob=\"true\"") >= 1, htmx);
}

/// Measure the sync build of the refused answer with a line under each value input.
///
/// Solves the problem that the handler `save` has no sync step that a test can call alone. The
/// function repeats `save` up to the command outside the interval, and runs the command that
/// refuses. Inside `peak_in` it repeats the sync tail of `failed` in `reply.rs`: the sent grid,
/// the lines of the command, and `refuse`. It then calls `answer` of the layer `render_errors`.
/// The list of faults of the command is live before the interval, so the peak excludes it. The
/// caller compares the length and the hash with the answer of the router, so a drift fails.
async fn measure_lines_render(world: &World, headers: &[(&str, &str)], body: &str) -> Rendered {
    let state = &world.fixture.state;
    let loaded = loaded_of(world, ENGINEER_SESSION, headers).await;
    let ctx = loaded.ctx(state);
    let sent = read_body(body.as_bytes(), FieldSet::Parameter).unwrap();
    let checked = check(&sent);
    assert!(checked.lines.is_empty());
    let type_expr = checked.type_expr.clone().unwrap();
    let definition = checked.definition.unwrap();
    let view = load_shown(ctx, Some(type_expr), None).await.unwrap();
    assert!(view.line.is_none());
    let tree = view.form.preview.as_ref().unwrap();
    let value = sent.leaf_texts(tree).unwrap();
    let limits = sent.limit_list(None).unwrap();
    let input = UpdateParameterDefinitionInput {
        reference: loaded.target.reference.clone(),
        number: loaded.target.number,
        expected_row_version: definition.row_version,
        key: ParameterKey::from(KEY),
        unit_path: definition.unit_path,
        tag: definition.tag,
        name_en: definition.name_en,
        description_en: definition.description_en,
        type_expr: definition.type_expr,
        s7_address: definition.s7_address,
        ordinal: definition.ordinal,
        retype: Some(RetypeInput {
            value: ValueInput::Texts(value),
            limits,
        }),
    };
    let user = loaded
        .request
        .auth()
        .map_err(|_error| "no session")
        .unwrap()
        .user();
    let error = UpdateParameterDefinition::new(state.identity().app().clone())
        .execute(user, &loaded.request.meta(), input)
        .await
        .unwrap_err();
    let errors = ErrorContext {
        request_id: ctx.request.request_id(),
        lang: ctx.request.lang(),
        htmx: ctx.request.is_htmx(),
        retry: None,
        state,
    };

    let started = Instant::now();
    let (response, peak) = peak_in(|| {
        let refused = {
            let shown = sent_shown(ctx, &view.form, &sent, view.previewed.as_ref());
            let lines = {
                let grid = shown.grid.as_ref();
                command_lines(&error, &|fault| grid.and_then(|grid| grid.input_of(fault)))
            };
            refuse(ctx, &view.form, shown, lines.unwrap())
        };
        answer(&refused, &errors)
    });
    let time = started.elapsed();
    drop(error);

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let bytes = to_bytes(response.into_body(), ANSWER_LIMIT).await.unwrap();
    let (len, sha256) = loaded.digest_of(std::str::from_utf8(&bytes).unwrap());
    Rendered {
        len,
        sha256,
        peak,
        time,
    }
}

/// Send the body with the bad texts to `BIG` and check the answer against `pin`.
async fn measure_lines(pin: &Pin, lang: (&str, &str), extra: &[(&str, &str)]) {
    let world = largest_form_world();
    let body = lines_body();
    let headers = headers_of(&world.engineer.cookie, lang, extra);
    assert_lines_refusal(&world, &headers, &body, !extra.is_empty()).await;
    let before = world.fixture.snapshot();

    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || {
        definition_request(&headers, &world.engineer.csrf, &body)
    })
    .await;
    let request_time: Duration = started.elapsed();
    assert_eq!(world.fixture.snapshot(), before);
    let rendered = measure_lines_render(&world, &headers, &body).await;

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
    check_status(
        pin,
        StatusCode::UNPROCESSABLE_ENTITY,
        &measured,
        &rendered,
        request_time,
    );
}

/// The page without `HX-Request` in `pl`.
const LINES_PAGE_PL: Pin = Pin {
    name: "largest_refused_lines_page_pl",
    len: 33_318_063,
    sha256: "a77215b4e02596bc291c384607837cac0f0ae9a800f5874f48aa2a4b66e8c354",
    request_peak: 239_892_126,
    render_peak: 106_203_306,
};

/// The HTMX answer in `pl`.
const LINES_HTMX_PL: Pin = Pin {
    name: "largest_refused_lines_htmx_pl",
    len: 33_315_879,
    sha256: "ede31c25af9f3d09ab33db2c8d3bc72566ba1d196d59495daf3890af275b19a4",
    request_peak: 239_892_126,
    render_peak: 106_203_306,
};

#[tokio::test]
#[ignore = "builds a body of 12 MB and an answer of about 33 MB: run alone with --ignored"]
async fn largest_refused_lines_page_pl() {
    measure_lines(&LINES_PAGE_PL, POLISH, &[]).await;
}

#[tokio::test]
#[ignore = "builds a body of 12 MB and an answer of about 33 MB: run alone with --ignored"]
async fn largest_refused_lines_htmx_pl() {
    measure_lines(&LINES_HTMX_PL, POLISH, &[HTMX]).await;
}
