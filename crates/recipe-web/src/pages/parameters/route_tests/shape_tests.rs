//! Measurement of the shapes of a parameter that differ from the fixture (tasks 5.8 to 5.10,
//! design D2 and D6, ADR-0223).
//!
//! A shape has its own type, leaves, and limits. The helper `shape_world` saves the shape through
//! `/definition` as the ProcessEngineer and returns the committed `row_version`. Each test pins the
//! answer of the code after groups 3 and 4, so a pin is a regression check and not a red test.
//! Run the tests alone: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test
//! -p recipe-web --all-features --locked wide_shape -- --ignored --test-threads=1`. Replace the
//! filter `wide_shape` with `deep_shape` for task 5.9, and with `string_shape` for task 5.10.

use std::collections::BTreeSet;
use std::time::Instant;

use axum::http::StatusCode;
use recipe_domain::recipe::RecipeVersionId;
use serde_json::{Value, json};

use super::largest_form_tests::{
    LEAVES, MAX_TEXT, MIN_TEXT, PAGE_LIMIT, VERSION_ID, array_levels, largest_form_world,
    ldt_count, value_text,
};
use super::largest_measure_tests::{measure_render, page_request};
use super::largest_refused_tests::measure_render as measure_refused_render;
use super::measure_kit::{
    ANSWER_LIMIT, Pin, change_a_sum, check_answer, check_status, count, definition_path,
    definition_request, headers_of, page_path, report,
};
use crate::pages::parameters::test_kit::form;
use crate::settings::WebSettings;
use crate::test_support::heap::{BodyDigest, peak_of};
use crate::test_support::plant::{POLISH, World};
use crate::test_support::{call_bounded, get_bounded};

/// The mark of the leaf rows of the grid.
const LEAF_ROW: &str = "<tr><th scope=\"row\"";
/// The mark of the mode rows of the grid.
const MODE_ROW: &str = "<tr><td colspan=\"6\"><span class=\"grid-note\">";
/// The mark of the rows `[*]` of the grid: the row of the limit of all elements of an array level.
const ALL_ROW: &str = "<tr class=\"row-all\">";
/// The number of mode rows of the wide shape: one for each of its 2 type levels.
const WIDE_MODE_ROWS: usize = 2;
/// The text of the wide shape: 16 384 elements, and in each one array of one `LDT`.
const WIDE_TYPE: &str = "ARRAY[1..16384] OF ARRAY[1..1] OF LDT";
/// The number of array levels of the wide shape.
const WIDE_ARRAY_LEVELS: usize = 2;
/// The number of type levels of one element under the outer type level of the deep shape.
const DEEP_INNER_LEVELS: usize = 30;
/// The number of array levels of the deep shape: the outer one and the 30 of one element.
const DEEP_ARRAY_LEVELS: usize = 31;
/// The number of mode rows of the deep shape: one for each of its 31 type levels (design D2).
const DEEP_MODE_ROWS: usize = 31;

/// The type, the leaves, and the limits of a shape.
struct Shape {
    /// The name of the shape in the printed lines.
    name: &'static str,
    /// The number of array levels of the type.
    array_levels: usize,
    /// The text of the type of the parameter.
    type_text: String,
    /// The concrete path of each leaf, in the order of the values.
    leaf_paths: Vec<String>,
    /// The path of each limit row. Each limit row has the lowest and the highest `LDT`.
    limit_paths: Vec<String>,
}

impl Shape {
    /// Return the wide shape: a limit for all elements `[*]` under each outer element.
    ///
    /// The outer type level has no limit row of its own. The inner type level holds one limit row
    /// `[i][*]` for each of the 16 384 outer elements, so it shows a limit for each element.
    fn wide() -> Self {
        Self {
            name: "wide_shape",
            array_levels: WIDE_ARRAY_LEVELS,
            type_text: WIDE_TYPE.to_owned(),
            leaf_paths: (1..=LEAVES).map(|outer| format!("[{outer}][1]")).collect(),
            limit_paths: (1..=LEAVES).map(|outer| format!("[{outer}][*]")).collect(),
        }
    }

    /// Return the deep shape: 16 384 elements, and under each one 30 type levels of one element.
    ///
    /// The outer type level is the level of 16 384 elements. Design "Context" counts the mode rows
    /// of this shape before D2 as 1 + 30 x 16 384, so the outer level is the first and each outer
    /// element holds the 30 inner type levels. A leaf path is `[i]` and 30 times `[1]`. The limit
    /// row of an outer element is `[i]`, 29 times `[1]`, and `[*]`: one limit for all elements of
    /// the innermost array level, as the wide shape has it. Each leaf thus has a limit.
    fn deep() -> Self {
        let inner = "ARRAY[1..1] OF ".repeat(DEEP_INNER_LEVELS);
        let ones = |count: usize| "[1]".repeat(count);
        Self {
            name: "deep_shape",
            array_levels: DEEP_ARRAY_LEVELS,
            type_text: format!("ARRAY[1..16384] OF {inner}LDT"),
            leaf_paths: (1..=LEAVES)
                .map(|outer| format!("[{outer}]{}", ones(DEEP_INNER_LEVELS)))
                .collect(),
            limit_paths: (1..=LEAVES)
                .map(|outer| format!("[{outer}]{}[*]", ones(DEEP_INNER_LEVELS - 1)))
                .collect(),
        }
    }

    /// Return the value of a leaf as the JSON of the stored parameter: the count in one array for
    /// each inner type level.
    fn stored_leaf(&self, leaf: usize) -> Value {
        let inner = self.array_levels.saturating_sub(1);
        (0..inner).fold(json!(ldt_count(&value_text(leaf))), |value, _level| {
            json!([value])
        })
    }

    /// Return every pair of the edit form of `BIG` with the type, the values, and the limits.
    fn pairs(&self) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = [
            ("row_version", "1"),
            ("unit_path", "U1"),
            ("tag", "BIG"),
            ("name_en", "Big"),
            ("description_en", "The largest edit form"),
            ("s7_address", ""),
            ("ordinal", "0"),
        ]
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
        pairs.push(("type".to_owned(), self.type_text.clone()));
        pairs.push(("value_type".to_owned(), self.type_text.clone()));
        for (leaf, path) in self.leaf_paths.iter().enumerate() {
            pairs.push((format!("value{path}"), value_text(leaf)));
        }
        for (row, path) in self.limit_paths.iter().enumerate() {
            pairs.push((format!("limits[{row}].leaf_path"), path.clone()));
            pairs.push((format!("limits[{row}].min"), MIN_TEXT.to_owned()));
            pairs.push((format!("limits[{row}].max"), MAX_TEXT.to_owned()));
        }
        pairs
    }

    /// Return the form body of [`Shape::pairs`].
    fn body(&self) -> String {
        let pairs = self.pairs();
        let refs: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        form(&refs)
    }
}

/// Save the shape to `/definition` of `BIG` as the ProcessEngineer.
///
/// Solves the problem that a test of a page needs a stored parameter with the type and the limits
/// of the shape, which only a save can make. The helper requires `303`. It returns the world and
/// the committed `row_version`, which is 2: the seed has 1 and the save adds 1.
async fn shape_world(shape: &Shape) -> (World, i64) {
    let world = largest_form_world();
    let body = shape.body();

    let reply = world
        .post(&world.engineer, &definition_path(), &body, false)
        .await;

    let code_at = reply.body.find("E-").unwrap_or(0);
    let shown: String = reply.body.chars().skip(code_at).take(300).collect();
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{shown}");
    let row_version = world
        .fixture
        .store
        .state()
        .version_row_version(RecipeVersionId::from(VERSION_ID))
        .unwrap();
    (world, row_version)
}

/// Assert that the store holds the shape and its limits, and print the limit rows.
fn assert_stored(world: &World, shape: &Shape) {
    let state = world.fixture.store.state();
    let stored = state.parameters_of(RecipeVersionId::from(VERSION_ID));
    assert_eq!(stored.len(), 1);
    let parameter = stored.first().unwrap();
    assert_eq!(array_levels(&state, parameter.plc_type), shape.array_levels);
    let expected: Value = Value::Array((0..LEAVES).map(|leaf| shape.stored_leaf(leaf)).collect());
    assert_eq!(parameter.value.as_json(), &expected);
    let paths: BTreeSet<String> = parameter
        .limits
        .iter()
        .map(|limit| limit.leaf_path.to_string())
        .collect();
    let wanted: BTreeSet<String> = shape.limit_paths.iter().cloned().collect();
    assert_eq!(parameter.limits.len(), LEAVES);
    assert_eq!(paths, wanted);
    let (low, high) = (
        ldt_count(MIN_TEXT).to_string(),
        ldt_count(MAX_TEXT).to_string(),
    );
    for limit in &parameter.limits {
        assert_eq!(limit.min.map(|bound| bound.to_string()), Some(low.clone()));
        assert_eq!(limit.max.map(|bound| bound.to_string()), Some(high.clone()));
    }
    let first = parameter.limits.first().unwrap().leaf_path.to_string();
    let last = parameter.limits.last().unwrap().leaf_path.to_string();
    report(&format!(
        "{} stored: {} limit rows, {} distinct paths, first {first}, last {last}, \
         array levels {}",
        shape.name,
        parameter.limits.len(),
        paths.len(),
        shape.array_levels,
    ));
}

/// The full page of the wide shape in `pl`.
const WIDE_PAGE_PL: Pin = Pin {
    name: "wide_shape_page_pl",
    len: 22_222_695,
    sha256: "75e3d47bdfcb15bb3240c145b91b5bb7bc956e181b990aa2c18dbbd58e2697ca",
    request_peak: 68_457_492,
    render_peak: 64_723_738,
};

/// Measure the `GET` of the edit page of the wide shape in `pl` as the ProcessEngineer.
#[tokio::test]
#[ignore = "builds a page of about 23 MB: run alone with --ignored --test-threads=1"]
async fn wide_shape_page_pl() {
    let shape = Shape::wide();
    let (world, row_version) = shape_world(&shape).await;
    assert_eq!(row_version, 2);
    assert_stored(&world, &shape);
    let headers = headers_of(&world.engineer.cookie, POLISH, &[]);

    let reply = get_bounded(&world.fixture.router(), &page_path(), &headers, PAGE_LIMIT).await;
    let shown = |needle: &str| reply.body.matches(needle).count();
    report(&format!(
        "wide_shape rows: status {} len {} leaf rows {} mode rows {} mode buttons {} rows [*] {}",
        reply.status,
        reply.body.len(),
        shown(LEAF_ROW),
        shown(MODE_ROW),
        shown("name=\"limit_mode\""),
        shown(ALL_ROW),
    ));
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(shown(MODE_ROW), WIDE_MODE_ROWS);
    assert_eq!(shown("name=\"limit_mode\""), WIDE_MODE_ROWS);
    assert_eq!(shown(ALL_ROW), LEAVES);
    assert_eq!(shown(LEAF_ROW), LEAVES);
    drop(reply);

    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || page_request(&headers)).await;
    let request_time = started.elapsed();
    let rendered = measure_render(&world, &headers).await;

    check_answer(&WIDE_PAGE_PL, &measured, &rendered, request_time);
}

/// The full page of the deep shape in `pl`.
const DEEP_PAGE_PL: Pin = Pin {
    name: "deep_shape_page_pl",
    len: 29_362_342,
    sha256: "b05337f36e0f12b62cb88f0ecbc7117849f51aca1374aa2d7044ca6176c87d19",
    request_peak: 214_406_323,
    render_peak: 116_729_361,
};

/// Measure the `GET` of the edit page of the deep shape in `pl` as the ProcessEngineer.
///
/// The type has one type level of 16 384 elements and 30 type levels of one element. Before D2 its
/// grid has about 491 521 mode rows. After D2 it must have 31.
#[tokio::test]
#[ignore = "builds a page of tens of MB: run alone with --ignored --test-threads=1"]
async fn deep_shape_page_pl() {
    let shape = Shape::deep();
    let (world, row_version) = shape_world(&shape).await;
    assert_eq!(row_version, 2);
    assert_stored(&world, &shape);
    let headers = headers_of(&world.engineer.cookie, POLISH, &[]);

    let reply = get_bounded(&world.fixture.router(), &page_path(), &headers, PAGE_LIMIT).await;
    let shown = |needle: &str| reply.body.matches(needle).count();
    report(&format!(
        "deep_shape rows: status {} len {} leaf rows {} mode rows {} mode buttons {} rows [*] {}",
        reply.status,
        reply.body.len(),
        shown(LEAF_ROW),
        shown(MODE_ROW),
        shown("name=\"limit_mode\""),
        shown(ALL_ROW),
    ));
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(shown(MODE_ROW), DEEP_MODE_ROWS);
    assert_eq!(shown("name=\"limit_mode\""), DEEP_MODE_ROWS);
    assert_eq!(shown(LEAF_ROW), LEAVES);
    assert_eq!(shown(ALL_ROW), LEAVES);
    drop(reply);

    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || page_request(&headers)).await;
    let request_time = started.elapsed();
    let rendered = measure_render(&world, &headers).await;

    check_answer(&DEEP_PAGE_PL, &measured, &rendered, request_time);
}

/// The text of the string shape: 16 384 elements of the longest seed string.
const STRING_TYPE: &str = "ARRAY[1..16384] OF STRING(254)";
/// The most characters of a `STRING(254)` leaf.
const STRING_LENGTH: usize = 254;
/// The size of a `"` in the body: `%22`.
const QUOTE_BODY_BYTES: usize = 3;
/// The cause of the refusal: the field `ordinal` is no whole number. It is no value leaf.
const BAD_ORDINAL: &str = "x";
/// The code of the line under `ordinal`.
const ORDINAL_CODE: &str = "E-WEB-008";
/// The mode rows of the string shape: one for its one type level (ADR-0223).
const STRING_MODE_ROWS: usize = 1;
/// How the HTML shows one `"` (askama): `&#34;`, which is 5 bytes.
const QUOTE_HTML: &str = "&#34;";
/// The length of the string shape body in bytes.
const STRING_BODY_BYTES: usize = 12_768_627;
/// The SHA-256 of the string shape body in lowercase hexadecimal.
const STRING_BODY_SHA256: &str = "bde9e9ab50f6e22ff36fececf998783f88eac9bd9231195bb9fc4361b677fe5b";

/// Return the pairs of the refused form: the fixture fields, a bad `ordinal`, and a text of
/// `quotes` times `"` in each of the 16 384 leaves `[1]` to `[16384]`.
fn string_pairs(quotes: usize) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = [
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "BIG"),
        ("name_en", "Big"),
        ("description_en", "The largest edit form"),
        ("s7_address", ""),
        ("ordinal", BAD_ORDINAL),
        ("type", STRING_TYPE),
        ("value_type", STRING_TYPE),
    ]
    .iter()
    .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
    .collect();
    let text = "\"".repeat(quotes);
    pairs.extend((1..=LEAVES).map(|leaf| (format!("value[{leaf}]"), text.clone())));
    pairs
}

/// Return the body of the string shape form with `quotes` times `"` in each leaf.
fn string_body(quotes: usize) -> String {
    let pairs = string_pairs(quotes);
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    form(&refs)
}

/// Return the largest count of `"` for each leaf that the value body limit permits, at most 254.
///
/// The body grows by 3 bytes for each `"` of each leaf, so the count follows from the length of
/// the body without any `"`.
fn quote_count() -> usize {
    let limit = WebSettings::default().body_limit_value;
    let free = limit.checked_sub(string_body(0).len()).unwrap();
    let per_quote = LEAVES.checked_mul(QUOTE_BODY_BYTES).unwrap();
    free.checked_div(per_quote).unwrap().min(STRING_LENGTH)
}

/// Assert the refusal of the string shape form and the escape of the quotes in each value input.
async fn assert_string_refusal(world: &World, headers: &[(&str, &str)], body: &str, quotes: usize) {
    let request = definition_request(headers, &world.engineer.csrf, body);
    let reply = call_bounded(&world.fixture.router(), request, ANSWER_LIMIT).await;
    let page = &reply.body;
    let shown = |needle: &str| count(page, needle);
    let input = format!("value=\"{}\" aria-labelledby=", QUOTE_HTML.repeat(quotes));
    report(&format!(
        "string_shape refusal: status {} len {} leaf rows {} mode buttons {} mode rows {} \
         code {ORDINAL_CODE} {} ordinal line {} value inputs with {quotes} quotes {} form rows {}",
        reply.status,
        page.len(),
        shown(LEAF_ROW),
        shown("name=\"limit_mode\""),
        shown(MODE_ROW),
        shown(ORDINAL_CODE),
        shown("id=\"ordinal-error\""),
        shown(&input),
        shown("<form"),
    ));
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(shown("id=\"ordinal-error\""), 1);
    assert!(shown(ORDINAL_CODE) >= 1);
    assert_eq!(shown(LEAF_ROW), LEAVES);
    assert_eq!(shown("name=\"limit_mode\""), STRING_MODE_ROWS);
    assert_eq!(shown(MODE_ROW), STRING_MODE_ROWS);
    assert_eq!(shown(&input), LEAVES);
    assert!(
        page.len() > body.len(),
        "the answer is not larger than the body"
    );
}

/// The refused page without `HX-Request` in `pl`.
const STRING_PAGE_PL: Pin = Pin {
    name: "string_shape_page_pl",
    len: 27_510_269,
    sha256: "d0d3cb79cd89f0e2db9fc4b043a7c7db69346270d1b3b56afc6a2ebdf9ed710c",
    request_peak: 65_133_372,
    render_peak: 57_740_881,
};

/// Measure the refused form of `ARRAY[1..16384] OF STRING(254)` in `pl` as the ProcessEngineer.
///
/// Each leaf holds `"` as often as the body limit permits. The form is refused because of its
/// field `ordinal`, so the answer echoes the 16 384 value leaves. A `"` costs 3 bytes in the body
/// (`%22`) and 5 bytes in the HTML (`&#34;`), so the answer is larger than the body.
#[tokio::test]
#[ignore = "builds a body of 13 MB and an answer of about 22 MB: run alone with --ignored"]
async fn string_shape_refused_page_pl() {
    let world = largest_form_world();
    let quotes = quote_count();
    let body = string_body(quotes);
    let limit = WebSettings::default().body_limit_value;
    assert!(body.len() <= limit, "{} bytes, limit {limit}", body.len());
    assert!(quotes == STRING_LENGTH || body.len() + LEAVES * QUOTE_BODY_BYTES > limit);
    let mut digest = BodyDigest::new("");
    digest.push(body.as_bytes());
    let (body_len, body_sha256) = digest.finish();
    report(&format!(
        "string_shape body: {quotes} quotes in each leaf, len {body_len} sha256 {body_sha256}, \
         limit {limit}"
    ));
    assert_eq!(body_len, STRING_BODY_BYTES);
    assert_eq!(body_sha256, STRING_BODY_SHA256);
    let headers = headers_of(&world.engineer.cookie, POLISH, &[]);
    assert_string_refusal(&world, &headers, &body, quotes).await;
    let before = world.fixture.snapshot();

    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || {
        definition_request(&headers, &world.engineer.csrf, &body)
    })
    .await;
    let request_time = started.elapsed();
    assert_eq!(world.fixture.snapshot(), before);
    assert!(measured.len > body.len());
    let rendered = measure_refused_render(&world, &headers, &body).await;

    report(&format!(
        "{}: request peak {} | render peak {} | allowance {} | render time {} ms | \
         body len {} | answer len {} | sum of change A {}",
        STRING_PAGE_PL.name,
        measured.peak,
        rendered.peak,
        measured.peak.saturating_sub(rendered.peak),
        rendered.time.as_millis(),
        body.len(),
        measured.len,
        change_a_sum(rendered.peak),
    ));
    check_status(
        &STRING_PAGE_PL,
        StatusCode::UNPROCESSABLE_ENTITY,
        &measured,
        &rendered,
        request_time,
    );
}
