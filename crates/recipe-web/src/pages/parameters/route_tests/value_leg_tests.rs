//! Tests of the two bodies of the value leg of the load job (design `value-body-memory-load` D6).
//!
//! Round 1 sends the alias form with a long `description_en`. The server refuses it with one
//! field line. Round 2 sends the alias form with the normal description. The server saves it.
//! Each test pins the length and the SHA-256 of its body, so a change of the body fails a test.

use axum::http::StatusCode;
use recipe_domain::recipe::RecipeVersionId;

use super::largest_form_tests::{
    KEY, REFERENCE, TYPE_LEVEL_MODE_ROWS, VERSION_ID, largest_form_world, nested,
};
use super::largest_refused_tests::aliased_pairs;
use super::measure_kit::{count, definition_path, report};
use super::value_leg_kit::{body_of, digest_of, field_lines};
use crate::pages::parameters::test_kit::form;
use crate::test_support::plant::{ENGLISH, World};
use crate::test_support::{Reply, post_fields_bounded};

/// The description of the round 2 body. It is the description of `form_pairs`.
const DESCRIPTION: &str = "The largest edit form";
/// The shortest round 1 body: 16 MiB minus 64 KiB.
const WINDOW_LOW: usize = 16_711_680;
/// The longest round 1 body: 16 MiB, `body_limit_value`.
const WINDOW_HIGH: usize = 16_777_216;
/// The longest answer that the tests read: 32 MiB.
/// The longest round 1 page has 19 478 174 bytes (design D6).
const ANSWER_LIMIT: usize = 32 * 1024 * 1024;
/// The number of wide levels of the value leg: 2^12 = 4 096 leaves (design D9).
const VALUE_WIDE_LEVELS: usize = 12;
/// The number of leaves of the value leg and of its stored limit rows.
const VALUE_LEAVES: usize = 1 << VALUE_WIDE_LEVELS;
/// The bad `max_length` pair that the order test adds to a limit row.
const BAD_LENGTH_PAIR: (&str, &str) = ("limits[0].max_length", "x");
/// The number of `x` in `description_en` of the round 1 body.
///
/// It is the multiple of 10 000 that gives the body length nearest to 16 744 448 bytes.
const FULL_FORM_PADDING: usize = 13_710_000;
/// The length of the round 1 body in bytes.
const FULL_FORM_BYTES: usize = 16_743_514;
/// The SHA-256 of the round 1 body in lowercase hexadecimal.
const FULL_FORM_SHA256: &str = "354b1a3e4c5ff56400dc92f6f3ddf84fc104390e3334269d852b3ebae599ccc9";
/// The length of the round 2 body in bytes.
const WRITE_FORM_BYTES: usize = 3_033_541;
/// The SHA-256 of the round 2 body in lowercase hexadecimal.
const WRITE_FORM_SHA256: &str = "631700fb82002bb379b37cc2cc9a7d6d9ef2dd628769e4ac3dd9a517f3076022";
/// The length of the compact JSON text of the stored round 2 value.
const VALUE_JSON_BYTES: usize = 253_949;
/// The SHA-256 of the compact JSON text of the stored round 2 value.
const VALUE_JSON_SHA256: &str = "3f6aacb71efb84742bbb8fc0ccdc435e9869550da22c89f1b74fec934d3d3527";

/// Send the round 1 body `body` as the engineer and read the answer up to `ANSWER_LIMIT`.
async fn refused_page(world: &World, body: &str) -> Reply {
    post_fields_bounded(
        &world.fixture.router(),
        &definition_path(),
        Some(&world.engineer),
        &[ENGLISH],
        body,
        ANSWER_LIMIT,
    )
    .await
}

/// Return the part of `page` between the first `<ul class="field-lines">` and its end.
fn summary_of(page: &str) -> &str {
    let after = page.split("<ul class=\"field-lines\">").nth(1).unwrap();
    after.split("</ul>").next().unwrap()
}

/// Assert that the refused `page` shows every sent leaf row and one mode row for each type level.
///
/// The refusal must keep the whole grid, so a short page with only the `description_en` line
/// fails here.
fn assert_grid_rows(page: &str) {
    let shown = |needle: &str| count(page, needle);
    assert_eq!(shown("<tr><th scope=\"row\""), VALUE_LEAVES);
    assert_eq!(shown("name=\"limit_mode\""), TYPE_LEVEL_MODE_ROWS);
    assert_eq!(
        shown("<tr><td colspan=\"6\"><span class=\"grid-note\">"),
        TYPE_LEVEL_MODE_ROWS
    );
}

#[tokio::test]
async fn value_leg_write_form() {
    let world = largest_form_world();
    let body = body_of(&aliased_pairs(DESCRIPTION, VALUE_WIDE_LEVELS));
    let (len, sha256) = digest_of(&body);
    assert_eq!(len, WRITE_FORM_BYTES);
    assert_eq!(sha256, WRITE_FORM_SHA256);

    let reply = world
        .post(&world.engineer, &definition_path(), &body, false)
        .await;

    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    let page = format!("/recipes/{REFERENCE}/versions/1/parameters/{KEY}");
    assert_eq!(reply.header("location"), page);
    let stored = world
        .fixture
        .store
        .state()
        .parameters_of(RecipeVersionId::from(VERSION_ID));
    let parameter = stored.first().unwrap();
    assert_eq!(parameter.limits.len(), VALUE_LEAVES);
    let expected = nested(0, &mut 0, VALUE_WIDE_LEVELS);
    assert_eq!(parameter.value.as_json(), &expected);
    let json = serde_json::to_string(parameter.value.as_json()).unwrap();
    let (json_len, json_sha256) = digest_of(&json);
    assert_eq!(json_len, VALUE_JSON_BYTES);
    assert_eq!(json_sha256, VALUE_JSON_SHA256);
}

#[tokio::test]
async fn value_leg_full_form() {
    let world = largest_form_world();
    let body = body_of(&aliased_pairs(
        &"x".repeat(FULL_FORM_PADDING),
        VALUE_WIDE_LEVELS,
    ));
    assert!(
        (WINDOW_LOW..=WINDOW_HIGH).contains(&body.len()),
        "{}",
        body.len()
    );
    let (len, sha256) = digest_of(&body);
    assert_eq!(len, FULL_FORM_BYTES);
    assert_eq!(sha256, FULL_FORM_SHA256);
    let before = world.fixture.snapshot();

    let reply = refused_page(&world, &body).await;

    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    let page_len = reply.body.len();
    report(&format!(
        "full form page: {page_len} margin {}",
        ANSWER_LIMIT.saturating_sub(page_len)
    ));
    let lines = field_lines(&reply.body);
    let only = ("description_en".to_owned(), "E-WEB-008".to_owned());
    assert_eq!(lines, vec![only], "{} bytes", reply.body.len());
    assert_grid_rows(&reply.body);
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn value_leg_refusal_order() {
    let world = largest_form_world();
    let extra = form(&[BAD_LENGTH_PAIR]).len() + 1;
    let mut pairs = aliased_pairs(&"x".repeat(FULL_FORM_PADDING - extra), VALUE_WIDE_LEVELS);
    pairs.push((BAD_LENGTH_PAIR.0.to_owned(), BAD_LENGTH_PAIR.1.to_owned()));
    let body = body_of(&pairs);
    assert_eq!(body.len(), FULL_FORM_BYTES);

    let reply = refused_page(&world, &body).await;

    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    let lines = field_lines(&reply.body);
    let fields: Vec<&str> = lines.iter().map(|(field, _code)| field.as_str()).collect();
    assert!(fields.contains(&"description_en"), "{fields:?}");
    assert!(!fields.contains(&"form"), "{fields:?}");
    assert_grid_rows(&reply.body);
    // An `LDT` row has no `max_length` input, so only the summary lists its line.
    let summary = summary_of(&reply.body);
    assert!(summary.contains("href=\"#description_en\""));
    assert!(summary.contains("href=\"#limits-0-max_length\""));
}
