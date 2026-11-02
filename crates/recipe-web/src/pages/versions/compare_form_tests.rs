//! Tests of the compare form of the version page (task 4.4, ADR-0108 (d), design D3).
//!
//! The world is the approval world. `R-500` has the approved version 1 and the draft 2, and the
//! draft is based on version 1. `R-600` has one retired version. A test that needs a third version
//! runs the real commands over HTTP, as a user does.

use axum::http::StatusCode;

use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{ENGLISH, HTMX, TARGET_CONTENT, main_of};
use crate::test_support::{Reply, post_fields};

/// The headers of an HTMX request that targets `main`, in English.
const CONTENT: [(&str, &str); 3] = [ENGLISH, HTMX, TARGET_CONTENT];
/// The headers of a plain form request, in English.
const PLAIN: [(&str, &str); 1] = [ENGLISH];
/// The start of the compare form, without its attributes.
const FORM_START: &str = "<form method=\"get\" action=\"/recipes/";
/// The opening tag of the compare form of `R-500`.
const FORM_TAG: &str = "<form method=\"get\" action=\"/recipes/R-500/compare\" class=\"actions\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\">";
/// The address of version 2 of `R-500`.
const V2: &str = "/recipes/R-500/versions/2";

/// Send the form `body` to `path` as the Administrator with the headers `headers`.
async fn post(world: &AuditWorld, path: &str, body: &str, headers: &[(&str, &str)]) -> Reply {
    let router = world.fixture.router();
    post_fields(&router, path, Some(&world.data.admin), headers, body).await
}

/// Return the page `path` of the Administrator in the language `code`.
async fn page(world: &AuditWorld, code: &str, path: &str) -> Reply {
    let reply = world.get(&world.data.admin, code, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
    reply
}

/// Run the command `action` on version 2 of `R-500` with its current `row_version`.
async fn command(world: &AuditWorld, action: &str) {
    let row_version = world.row_version_of("R-500", 2).await;
    let body = format!("row_version={row_version}");
    let reply = post(world, &format!("{V2}/{action}"), &body, &CONTENT).await;
    assert_eq!(reply.status, StatusCode::OK, "{action}: {}", reply.body);
}

/// Approve version 2 of `R-500` and create the draft 3 based on version 1.
async fn add_third_version(world: &AuditWorld) {
    command(world, "approve").await;
    let body = "based_on=1&change_note=Third+version";
    let created = post(world, "/recipes/R-500/versions", body, &PLAIN).await;
    assert_eq!(created.status, StatusCode::SEE_OTHER, "{}", created.body);
}

/// Return the compare form of `main`, from its opening tag to its closing tag.
fn form_of(main: &str) -> &str {
    let (_before, rest) = main.split_once(FORM_START).expect(main);
    let end = rest.find("</form>").expect(main);
    rest.get(..end).expect(main)
}

/// Return the `select` element with the id `name` in `form`.
fn select_of<'a>(form: &'a str, name: &str) -> &'a str {
    let start = format!("<select id=\"{name}\" name=\"{name}\">");
    let (_before, rest) = form.split_once(&start).expect(form);
    rest.split_once("</select>").expect(form).0
}

/// Return the numbers of the options of `select`, in the order of the markup.
fn numbers_of(select: &str) -> Vec<&str> {
    select
        .split("<option value=\"")
        .skip(1)
        .filter_map(|option| option.split_once('"').map(|(number, _rest)| number))
        .collect()
}

/// Return the number of the selected option of `select`.
fn selected_of(select: &str) -> &str {
    let (before, _after) = select.split_once(" selected>").expect(select);
    let value = before.rsplit_once("<option value=\"").expect(select).1;
    value.trim_end_matches('"')
}

/// Assert that the lists `n` and `m` of the compare form in `main` select `n` and `m`.
fn assert_selected(main: &str, (n, m): (&str, &str)) {
    let form = form_of(main);
    assert_eq!(selected_of(select_of(form, "n")), n, "{form}");
    assert_eq!(selected_of(select_of(form, "m")), m, "{form}");
}

#[tokio::test]
async fn compare_form_draft_with_a_base_selects_the_shown_and_the_base_version() {
    let world = audit_world().await;
    add_third_version(&world).await;
    let reply = page(&world, "en", "/recipes/R-500/versions/3").await;
    let main = main_of(&reply.body);
    assert!(main.contains(FORM_TAG), "{main}");
    assert_selected(main, ("3", "1"));
    let form = form_of(main);
    for name in ["n", "m"] {
        assert_eq!(numbers_of(select_of(form, name)), ["3", "2", "1"], "{form}");
    }
    assert!(!form.contains("csrf_token"), "{form}");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn compare_form_has_a_visible_label_for_each_list_and_a_button() {
    let world = audit_world().await;
    for (code, labels) in [
        ("en", ["Version", "Compare with version", "Compare"]),
        ("pl", ["Wersja", "Porównaj z wersją", "Porównaj"]),
    ] {
        let reply = page(&world, code, V2).await;
        let form = form_of(main_of(&reply.body));
        let [n_label, m_label, button] = labels;
        let n = format!("<label for=\"n\">{n_label}</label>");
        let m = format!("<label for=\"m\">{m_label}</label>");
        assert!(form.contains(&n), "{form}");
        assert!(form.contains(&m), "{form}");
        assert!(form.contains(&format!(">{button}</button>")), "{form}");
    }
}

#[tokio::test]
async fn compare_form_recipe_with_one_version_shows_no_form() {
    let world = audit_world().await;
    let reply = page(&world, "en", "/recipes/R-600/versions/1").await;
    let main = main_of(&reply.body);
    assert!(!main.contains("/compare"), "{main}");
    assert!(!main.contains(FORM_START), "{main}");
}

#[tokio::test]
async fn compare_form_administrator_on_a_deleted_recipe_keeps_the_form() {
    let world = audit_world().await;
    command(&world, "approve").await;
    command(&world, "retire").await;
    let deleted = post(&world, "/recipes/R-500/delete", "", &PLAIN).await;
    assert_eq!(deleted.status, StatusCode::SEE_OTHER, "{}", deleted.body);
    let reply = page(&world, "en", V2).await;
    let main = main_of(&reply.body);
    assert!(main.contains("Deleted"), "{main}");
    assert!(main.contains(FORM_TAG), "{main}");
    assert_selected(main, ("2", "1"));
}

#[tokio::test]
async fn compare_form_version_without_a_base_selects_the_highest_other_version() {
    let world = audit_world().await;
    add_third_version(&world).await;
    // Version 1 has no base version. The highest other version is 3.
    let first = page(&world, "en", "/recipes/R-500/versions/1").await;
    assert_selected(main_of(&first.body), ("1", "3"));
    // Version 2 has the base version 1.
    let second = page(&world, "en", V2).await;
    assert_selected(main_of(&second.body), ("2", "1"));
}

#[tokio::test]
async fn compare_form_stands_after_the_row_of_links_and_outside_every_actions_paragraph() {
    let world = audit_world().await;
    let reply = page(&world, "en", V2).await;
    let main = main_of(&reply.body);
    let form_at = main.find(FORM_START).expect(main);
    let history_at = main.find("href=\"/recipes/R-500/history\"").expect(main);
    assert!(history_at < form_at, "{main}");
    for paragraph in main.split("<p class=\"actions\"").skip(1) {
        let inside = paragraph.split("</p>").next().expect(paragraph);
        assert!(!inside.contains("<form"), "{inside}");
    }
    let before_form = main.get(..form_at).expect(main);
    let last_row = before_form
        .rsplit_once("<p class=\"actions\"")
        .expect(main)
        .1;
    assert!(last_row.contains("</p>"), "inside a paragraph: {last_row}");
}

#[tokio::test]
async fn compare_form_has_the_three_hx_literals_and_no_other_hx_attribute() {
    let world = audit_world().await;
    let reply = page(&world, "en", V2).await;
    let main = main_of(&reply.body);
    assert!(main.contains(FORM_TAG), "{main}");
    let literals = "hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\"";
    assert!(FORM_TAG.contains(literals));
    assert_eq!(form_of(main).matches("hx-").count(), 3, "{}", form_of(main));
}

#[tokio::test]
async fn compare_form_no_hx_attribute_holds_the_reference_number() {
    let world = audit_world().await;
    let reply = page(&world, "en", V2).await;
    let main = main_of(&reply.body);
    let (_before, rest) = main.split_once(FORM_START).expect(main);
    let tag = rest.split_once('>').expect(rest).0;
    assert!(tag.starts_with("R-500/compare\""), "{tag}");
    let attributes: Vec<&str> = tag.split(" hx-").skip(1).collect();
    assert_eq!(attributes.len(), 3, "{tag}");
    for attribute in attributes {
        let value = attribute.split('"').nth(1).expect(attribute);
        assert!(!value.contains("500"), "{attribute}");
    }
}
