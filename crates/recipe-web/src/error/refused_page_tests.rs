//! Tests of the full page of a refused form and of the toast of an error answer (task 17.3).
//!
//! K5: a request without `HX-Request` that a form refuses gets the summary and the form with the
//! sent texts in the session layout (`api.md` section 5.3, ADR-0041 (e)). DR-5 (a): every HTMX
//! error answer clears `#toast` out of band (`design-system.md` section 10.7).

use axum::http::StatusCode;

use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::Reply;
use crate::test_support::heap::BodyDigest;
use crate::test_support::plant::{HTMX, World, main_of};
use crate::test_support::recipes::{LINE_KEY, SPEED_KEY};

/// The element of an HTMX error answer that empties the status region out of band.
const TOAST_CLEAR: &str = "<div id=\"toast\" hx-swap-oob=\"innerHTML\"></div>";
/// The save path of the edit form of the parameter `SPEED` of the draft 3 of `R-100`.
const DEFINITION: &str =
    "/recipes/R-100/versions/3/parameters/73570007-0000-0000-0000-000000000011/definition";

/// Return the edit form body of `SPEED` (`INT`, limit 0 to 3000) with `value` and `row_version`.
fn speed_body(value: &str, row_version: u8) -> String {
    format!(
        "row_version={row_version}&unit_path=U1&tag=SPEED&name_en=SPEED&description_en=A+parameter\
         &type=INT&value_type=INT&s7_address=DB2.DBW0&ordinal=1&value={value}\
         &limits%5B0%5D.leaf_path=&limits%5B0%5D.min=0&limits%5B0%5D.max=3000"
    )
}

#[test]
fn error_refused_page_the_save_path_names_the_key_of_speed() {
    assert!(DEFINITION.contains(&SPEED_KEY.to_string()));
}

/// Assert the full page of a refused form and return its form, from the form tag to `</main>`.
///
/// The page has the status of the error, the session layout, the summary with links, one `h1`,
/// and the form after the `h1`. The form has no out-of-band attribute.
fn refused_page<'a>(world: &World, reply: &'a Reply, status: StatusCode, form_id: &str) -> &'a str {
    assert_eq!(reply.status, status, "{}", reply.body);
    assert!(reply.body.starts_with("<!DOCTYPE html>"), "{}", reply.body);
    assert_clean_html(&reply.body);
    assert_eq!(reply.header("hx-retarget"), "", "{form_id}");
    let headers = format!(
        "hx-headers='{{\"X-CSRF-Token\":\"{}\"}}'",
        world.engineer.csrf
    );
    assert!(reply.body.contains(&headers), "{}", reply.body);
    assert!(
        reply.body.contains("<nav aria-label=\"Main\">"),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("Signed in as"), "{}", reply.body);
    assert!(!reply.body.contains("action=\"/lang\""), "{}", reply.body);
    assert!(!reply.body.contains("hx-swap-oob"), "{}", reply.body);
    let main = main_of(&reply.body);
    assert_eq!(main.matches("<h1").count(), 1, "{main}");
    assert!(main.contains("<a class=\"target-link\" href=\"#"), "{main}");
    let (before, rest) = main.split_once(&format!("<form id=\"{form_id}\"")).unwrap();
    assert!(before.contains("<h1"), "{main}");
    assert!(
        before.contains("<div class=\"alert alert-error\">"),
        "{main}"
    );
    assert!(!rest.contains("<form id="), "{main}");
    rest
}

#[tokio::test]
async fn error_refused_page_of_the_machine_form_keeps_the_sent_texts_in_the_session_layout() {
    let world = recipe_world();
    let body =
        "code=X1&name=Kept+line&plc_platform=S7_300&string_encoding=Latin1&main_unit_name=Infeed";
    let reply = world.post(&world.engineer, "/machines", body, false).await;
    let form = refused_page(
        &world,
        &reply,
        StatusCode::UNPROCESSABLE_ENTITY,
        "machine-new-form",
    );
    assert!(form.contains("value=\"Kept line\""), "{form}");
    assert!(form.contains("aria-invalid=\"true\""), "{form}");
    assert!(form.contains(&world.engineer.csrf), "{form}");
}

#[tokio::test]
async fn error_refused_page_of_the_recipe_form_shows_every_line_under_its_input() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let body = "machine_code=&reference_number=-bad%3C&name=Kept+name&change_note=";
    let reply = world.post(&world.engineer, "/recipes", body, false).await;
    let form = refused_page(
        &world,
        &reply,
        StatusCode::UNPROCESSABLE_ENTITY,
        "recipe-new-form",
    );
    assert!(reply.body.contains("Errors: 3."), "{}", reply.body);
    assert!(form.contains("value=\"-bad&#60;\""), "{form}");
    assert!(form.contains("value=\"Kept name\""), "{form}");
    assert_eq!(form.matches("aria-invalid=\"true\"").count(), 3, "{form}");
    for id in ["machine_code", "reference_number", "change_note"] {
        let line = format!("<div class=\"field-error\" id=\"{id}-error\">");
        assert!(form.contains(&line), "{id}: {form}");
        assert!(reply.body.contains(&format!("href=\"#{id}\"")), "{id}");
    }
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn error_refused_page_of_the_parameter_form_keeps_the_bad_value_under_its_input() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let reply = world
        .post(&world.engineer, DEFINITION, &speed_body("abc", 1), false)
        .await;
    let form = refused_page(
        &world,
        &reply,
        StatusCode::UNPROCESSABLE_ENTITY,
        "parameter-edit-form",
    );
    assert!(reply.body.contains("E-RCP-005"), "{}", reply.body);
    assert!(form.contains("id=\"value-0-error\""), "{form}");
    assert!(form.contains("value=\"abc\""), "{form}");
    assert!(form.contains("name=\"row_version\" value=\"1\""), "{form}");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn error_refused_page_another_error_without_htmx_keeps_the_full_error_page() {
    let world = recipe_world();
    let body = "machine_code=M7&reference_number=R-502&name=New&change_note=Note";
    let reply = world.post(&world.operator, "/recipes", body, false).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"), "{}", reply.body);
    assert!(!reply.body.contains("<form id="), "{}", reply.body);
    assert!(!reply.body.contains("<nav"), "{}", reply.body);
    assert!(
        main_of(&reply.body).contains("<a class=\"target-link\" href=\"/recipes\">"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn error_refused_page_every_htmx_error_answer_clears_the_toast() {
    let world = recipe_world();
    let missing = "/recipes/R-100/versions/3/parameters/x/delete";
    let cases = [
        (
            DEFINITION,
            speed_body("abc", 1),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (DEFINITION, speed_body("1200", 9), StatusCode::CONFLICT),
        (missing, "row_version=1".to_owned(), StatusCode::NOT_FOUND),
    ];
    for (path, body, status) in cases {
        let reply = world.post_with(&world.engineer, path, &body, &[HTMX]).await;
        assert_eq!(reply.status, status, "{path}: {}", reply.body);
        assert_eq!(reply.header("hx-retarget"), "#error-panel", "{path}");
        assert_eq!(
            reply.body.matches(TOAST_CLEAR).count(),
            1,
            "{status}: {}",
            reply.body
        );
    }
}

/// Assert that each `hx-target` of `page` names an element of `page` by its `id`.
///
/// The page of a refused form has only `#content`, `#error-panel`, and `#toast` of the layout. A
/// form that targets an element of the page that sent it finds no target there, so HTMX sends
/// nothing (RR1-1, ADR-0047 (a)).
fn assert_targets_on_page(page: &str, label: &str) {
    let targets = page
        .split("hx-target=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next());
    for target in targets {
        assert!(target.starts_with('#'), "{label}: target {target:?}");
        let id = format!("id=\"{}\"", target.trim_start_matches('#'));
        assert!(page.contains(&id), "{label}: no element for {target}");
    }
}

/// Send `body` to `path` as the engineer without HTMX and assert the page of the refused form.
///
/// The form on the page is a plain form: its first tag holds no `hx-*` attribute, so a second
/// save sends a normal `POST` also while HTMX runs (RR1-1).
async fn assert_plain_refused_form(world: &World, (path, body, form_id): (&str, &str, &str)) {
    let reply = world.post(&world.engineer, path, body, false).await;
    assert!(reply.body.starts_with("<!DOCTYPE html>"), "{path}");
    let main = main_of(&reply.body);
    let tag = format!("<form id=\"{form_id}\"");
    let (_before, rest) = main.split_once(&tag).unwrap_or((main, "missing form"));
    assert_ne!(rest, "missing form", "{path}: {}", reply.body);
    assert_targets_on_page(&reply.body, path);
    let first_tag = rest.split('>').next().unwrap();
    assert!(!first_tag.contains("hx-"), "{path}: {tag}{first_tag}>");
}

#[tokio::test]
async fn error_refused_page_of_each_plant_form_has_a_plain_form_and_no_missing_target() {
    let world = recipe_world();
    let udt = "name=ST_Press&members%5B0%5D.name=9a&members%5B0%5D.type=INT";
    let cases = [
        (
            "/machines/M7/units",
            "parent_path=U1.2.4.5.6&segment=7&name=Deep",
            "unit-add-form",
        ),
        (
            "/machines",
            "code=X1&name=Kept+line&plc_platform=S7_300&string_encoding=Latin1&main_unit_name=Infeed",
            "machine-new-form",
        ),
        ("/machines/M7/rename", "name=+", "machine-rename-form"),
        (
            "/machines/M7/units/U1.2/rename",
            "name=+",
            "unit-rename-form",
        ),
        ("/machines/M7/types", udt, "udt-form"),
    ];
    for case in cases {
        assert_plain_refused_form(&world, case).await;
    }
}

#[tokio::test]
async fn error_refused_page_of_each_recipe_form_has_a_plain_form_and_no_missing_target() {
    let world = recipe_world();
    let translation = DEFINITION.replace("/definition", "/translations/pl");
    let add = "row_version=1&unit_path=U1&tag=NEW_P&name_en=New&description_en=A+parameter\
               &type=INT&value_type=INT&s7_address=DB9.DBW0&ordinal=5&value=abc";
    let edit = speed_body("abc", 1);
    let cases = [
        ("/recipes/R-100/rename", "name=", "recipe-rename-form"),
        (
            "/recipes",
            "machine_code=&reference_number=-bad%3C&name=Kept+name&change_note=",
            "recipe-new-form",
        ),
        (
            "/recipes/R-100/versions",
            "based_on=abc&change_note=",
            "version-new-form",
        ),
        (
            translation.as_str(),
            "row_version=1&name=&description=Opis",
            "translation-pl-form",
        ),
        (
            "/recipes/R-100/versions/3/parameters",
            add,
            "parameter-new-form",
        ),
        (DEFINITION, edit.as_str(), "parameter-edit-form"),
    ];
    for case in cases {
        assert_plain_refused_form(&world, case).await;
    }
}

#[tokio::test]
async fn error_refused_page_a_full_error_page_has_one_status_region() {
    let world = recipe_world();
    let reply = world
        .post(&world.engineer, DEFINITION, &speed_body("abc", 1), false)
        .await;
    assert_eq!(
        reply.body.matches("id=\"toast\"").count(),
        1,
        "{}",
        reply.body
    );
    assert!(!reply.body.contains(TOAST_CLEAR), "{}", reply.body);
}

/// The save path of the edit form of the parameter `LINE` (`ST_Line@1`) of the draft 3 of `R-100`.
fn line_definition() -> String {
    format!("/recipes/R-100/versions/3/parameters/{LINE_KEY}/definition")
}

/// Return the edit form body of `LINE` with a bad value in the leaf `.zones[1].speed`.
///
/// The form sends the four zones and two limit rows, as the save of the route matrix does.
fn line_body() -> String {
    let zones: String = (1..=4)
        .map(|zone| {
            let speed = if zone == 1 { "abc" } else { "0" };
            format!("&value.zones%5B{zone}%5D.speed={speed}&value.zones%5B{zone}%5D.temp_max=0")
        })
        .collect();
    format!(
        "row_version=1&unit_path=U1&tag=LINE&name_en=line&description_en=The+zones+of+the+press+line\
         &type=ST_Line%401&value_type=ST_Line%401&s7_address=DB1.DBB0&ordinal=0{zones}\
         &limits%5B1%5D.leaf_path=.zones%5B*%5D.speed&limits%5B2%5D.leaf_path=.zones%5B*%5D.temp_max"
    )
}

/// Return the SHA-256 of the body of `reply` in hex, with each copy of the request id replaced.
///
/// The digest of the heap test support replaces the id, so a pin does not depend on it. The test
/// asserts that the body holds the id, so the replacement runs.
fn pinned_digest(reply: &Reply) -> (usize, String) {
    let request_id = reply.header("x-request-id");
    assert!(!request_id.is_empty(), "no request id: {}", reply.status);
    assert!(reply.body.contains(&request_id), "{}", reply.body);
    let mut digest = BodyDigest::new(&request_id);
    digest.push(reply.body.as_bytes());
    digest.finish()
}

/// Assert that the answer holds the grid, its mode row, and at least one limit row of `LINE`.
///
/// A pin of an answer without the grid would guard nothing.
fn assert_holds_the_line_grid(reply: &Reply) {
    let body = &reply.body;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert!(body.contains("<table class=\"leaf-grid\">"), "{body}");
    assert!(body.contains("name=\"limit_mode\""), "{body}");
    assert!(body.contains("<tr class=\"row-all\">"), "{body}");
    assert!(body.contains("name=\"limits[1].min\""), "{body}");
    assert!(body.contains("value=\"abc\""), "{body}");
}

/// The SHA-256 of the refused page of `LINE` without `HX-Request`, as hex.
///
/// The bytes are those of the code after group 3 of `parameter-page-memory`. The hash input
/// replaces each copy of the request id. Tasks 4.3 and 4.4 change the render path and must keep
/// this value.
const REFUSED_PAGE_SHA256: &str =
    "c91c4c4cc3519fef6eb9fa8d800925093b2a2e94aef6a7760c2153853b7077cd";
/// The SHA-256 of the HTMX refused answer of `LINE`, as hex.
///
/// The bytes are those of the code after group 3 of `parameter-page-memory`. The hash input
/// replaces each copy of the request id. Tasks 4.3 and 4.4 change the render path and must keep
/// this value.
const REFUSED_HTMX_SHA256: &str =
    "a470077ee0551e4b2c015f1a97c8954c84e555b4993dc731b19f8aea30dad224";

#[tokio::test]
async fn error_refused_page_pin_of_the_full_page_of_a_refused_line_grid() {
    let world = recipe_world();
    let reply = world
        .post(&world.engineer, &line_definition(), &line_body(), false)
        .await;
    assert_holds_the_line_grid(&reply);
    assert!(reply.body.starts_with("<!DOCTYPE html>"), "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "");
    let (_len, sha256) = pinned_digest(&reply);
    assert_eq!(sha256, REFUSED_PAGE_SHA256, "{}", reply.body);
}

#[tokio::test]
async fn error_refused_page_pin_of_the_htmx_answer_of_a_refused_line_grid() {
    let world = recipe_world();
    let reply = world
        .post_with(&world.engineer, &line_definition(), &line_body(), &[HTMX])
        .await;
    assert_holds_the_line_grid(&reply);
    assert!(!reply.body.starts_with("<!DOCTYPE html>"), "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert_eq!(reply.body.matches(TOAST_CLEAR).count(), 1, "{}", reply.body);
    let (_len, sha256) = pinned_digest(&reply);
    assert_eq!(sha256, REFUSED_HTMX_SHA256, "{}", reply.body);
}
