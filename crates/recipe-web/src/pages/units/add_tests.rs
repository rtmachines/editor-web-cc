//! Tests of `POST /machines/{code}/units` (task 9.2, specs "Addition of a unit" and "Plant forms
//! without user data in an hx attribute").

use axum::http::StatusCode;

use super::test_world::world;
use crate::i18n::Lang;
use crate::pages::test_helpers::{
    assert_clean_html, assert_htmx_error, assert_no_user_text_in_hx_attributes, hx_pairs,
};
use crate::test_support::plant::{HTMX, World, cookie_of, text_with};
use crate::test_support::{icon_tag, text};

/// The path of the route.
const ROUTE: &str = "/machines/M7/units";

/// Send an HTMX `POST` as the engineer and return the status and the body.
async fn htmx_post(world: &World, path: &str, body: &str) -> crate::test_support::Reply {
    world.post(&world.engineer, path, body, true).await
}

// --- Success with HTMX --------------------------------------------------------------------------

#[tokio::test]
async fn units_add_answers_the_whole_tree_with_the_new_unit_as_the_focus_unit() {
    let world = world();
    let reply = htmx_post(&world, ROUTE, "parent_path=U1.2.4.5&segment=7&name=Added").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.starts_with("<ul class=\"tree\">"),
        "{}",
        reply.body
    );
    let link =
        "<a class=\"target-link\" href=\"/machines/M7/units/U1.2.4.5.7\" autofocus>Added</a>";
    assert!(reply.body.contains(link), "{}", reply.body);
    assert_eq!(
        reply.body.matches("href=\"/machines/M7/units/U").count(),
        65,
        "64 units and the new one"
    );
    assert_eq!(
        reply.body.matches("autofocus").count(),
        1,
        "only the new link takes the focus"
    );
    assert_eq!(
        reply.body.matches(" hidden>").count(),
        4,
        "the ancestors U1.2.4 and U1.2.4.5 are expanded: {}",
        reply.body
    );
}

#[tokio::test]
async fn units_add_puts_the_message_in_the_toast_and_empties_the_error_panel_out_of_band() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let headers = [cookie_of(lang), HTMX];
        let reply = crate::test_support::post_fields(
            &world.fixture.router(),
            ROUTE,
            Some(&world.engineer),
            &headers,
            &format!(
                "parent_path=U1&segment={}&name=Added",
                if lang == Lang::En { 5 } else { 6 }
            ),
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let path = if lang == Lang::En { "U1.5" } else { "U1.6" };
        let sentence = text_with(lang, "unit-added", "path", path);
        let toast = format!(
            "<div id=\"toast\" hx-swap-oob=\"innerHTML\"><div class=\"toast\">{}",
            icon_tag("alert-success")
        );
        assert!(reply.body.contains(&toast), "{lang}: {}", reply.body);
        assert!(
            reply.body.contains(&format!("<span>{sentence}</span>")),
            "{lang}"
        );
        let panel = "<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>";
        assert!(reply.body.contains(panel), "{lang}: {}", reply.body);
    }
}

#[tokio::test]
async fn units_add_renews_the_form_for_the_same_parent_without_autofocus() {
    let world = world();
    let reply = htmx_post(&world, ROUTE, "parent_path=U1.2&segment=5&name=Added").await;
    let (_before, rest) = reply
        .body
        .split_once("<div id=\"unit-form\" hx-swap-oob=\"innerHTML\">")
        .unwrap();
    let form = rest.split("</form>").next().unwrap();
    assert!(rest.starts_with("<h2>New unit under U1.2</h2>"), "{rest}");
    assert!(form.contains("<option value=\"U1.2\" selected>"), "{form}");
    let (_pre, segment) = form.split_once("<input id=\"segment\"").unwrap();
    let segment = segment.split('>').next().unwrap();
    assert!(
        segment.contains("value=\"6\""),
        "the next free number: {segment}"
    );
    assert!(!form.contains("autofocus"), "{form}");
    assert!(form.contains("hx-target=\"#unit-tree\""), "{form}");
    let token = format!("value=\"{}\"", world.engineer.csrf);
    assert!(form.contains(&token), "{form}");
}

#[tokio::test]
async fn units_add_of_a_main_unit_renews_the_form_for_a_main_unit() {
    let world = world();
    let reply = htmx_post(&world, ROUTE, "parent_path=&segment=2&name=Outfeed").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("<h2>New main unit</h2>"),
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .contains("href=\"/machines/M7/units/U2\" autofocus>Outfeed</a>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn units_add_answer_hx_attributes_are_literals_and_the_html_is_clean() {
    let world = world();
    let reply = htmx_post(&world, ROUTE, "parent_path=U1.2&segment=5&name=Added").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let pairs = hx_pairs(&reply.body);
    assert!(pairs.len() >= 7, "{pairs:?}");
    for (name, value) in pairs {
        let known = matches!(
            (name.as_str(), value.as_str()),
            ("hx-swap-oob", "innerHTML")
                | ("hx-boost", "true")
                | ("hx-target", "#unit-tree" | "#unit-form")
                | ("hx-swap", "innerHTML")
                | ("hx-push-url", "false")
                | ("hx-sync", "closest form:drop")
        );
        assert!(known, "{name}={value}");
    }
    for needle in ["M7", "U1.2", "Added"] {
        assert_no_user_text_in_hx_attributes(&reply.body, needle);
    }
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn units_add_escapes_the_name_in_the_tree_and_in_the_list_of_parents() {
    let world = world();
    let payload = "<img src=x onerror=alert(1)>";
    let body = "parent_path=U1.1&segment=9&name=%3Cimg+src%3Dx+onerror%3Dalert(1)%3E";
    let reply = htmx_post(&world, ROUTE, body).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains(payload), "{}", reply.body);
    assert!(
        reply.body.contains("&#60;img src=x onerror=alert(1)&#62;"),
        "{}",
        reply.body
    );
    assert_clean_html(&reply.body);
    let form = world
        .get_form(&world.engineer, "/machines/M7/units/new")
        .await;
    assert!(!form.body.contains(payload), "{}", form.body);
}

// --- Success without HTMX -----------------------------------------------------------------------

#[tokio::test]
async fn units_add_without_htmx_redirects_to_the_machine_page_and_the_unit_exists() {
    let world = world();
    let before = world.audit_count();
    let reply = world
        .post(
            &world.admin,
            ROUTE,
            "parent_path=U1.2&segment=5&name=Added",
            false,
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M7");
    assert_eq!(world.audit_count(), before.checked_add(1).unwrap());
    let page = world.get_en(&world.engineer, "/machines/M7").await;
    assert!(
        page.body.contains("/machines/M7/units/U1.2.5"),
        "{}",
        page.body
    );
}

#[tokio::test]
async fn units_add_accepts_the_token_field_and_an_empty_parent_for_a_main_unit() {
    let world = world();
    let body = format!(
        "csrf_token={}&parent_path=&segment=3&name=Third",
        world.engineer.csrf
    );
    let reply = world.post(&world.engineer, ROUTE, &body, false).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    let page = world.get_en(&world.engineer, "/machines/M7").await;
    assert!(page.body.contains("/machines/M7/units/U3"), "{}", page.body);
}

// --- Errors -------------------------------------------------------------------------------------

#[tokio::test]
async fn units_add_a_sixth_level_gives_422_e_plt_002_and_creates_no_unit() {
    let world = world();
    let before = world.audit_count();
    let reply = htmx_post(&world, ROUTE, "parent_path=U1.2.4.5.6&segment=7&name=Deep").await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-002");
    assert_eq!(world.audit_count(), before);
    assert!(!reply.body.contains("<ul class=\"tree\">"));
}

#[tokio::test]
async fn units_add_gives_the_code_of_each_domain_error() {
    let world = world();
    let cases = [
        (
            "parent_path=U1.9&segment=1&name=A",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-003",
        ),
        (
            "parent_path=U1.2&segment=1&name=A",
            StatusCode::CONFLICT,
            "E-PLT-004",
        ),
        (
            "parent_path=U1.2&segment=x&name=A",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-017",
        ),
        (
            "parent_path=U1.2&segment=65&name=A",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-017",
        ),
        (
            "parent_path=X&segment=1&name=A",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-017",
        ),
        (
            "parent_path=U1.2.4.5.6.7&segment=1&name=A",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-002",
        ),
    ];
    for (body, status, code) in cases {
        let reply = htmx_post(&world, ROUTE, body).await;
        assert_htmx_error(&reply, status, code);
    }
}

#[tokio::test]
async fn units_add_under_an_archived_parent_gives_409_e_plt_009() {
    let world = world();
    let reply = htmx_post(
        &world,
        "/machines/M31/units",
        "parent_path=U2&segment=1&name=A",
    )
    .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-PLT-009");
}

#[tokio::test]
async fn units_add_for_an_unknown_or_archived_machine_gives_404_e_web_005() {
    let world = world();
    for code in ["M99", "M12", "X7"] {
        let path = format!("/machines/{code}/units");
        let reply = htmx_post(&world, &path, "parent_path=&segment=2&name=A").await;
        assert_htmx_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005");
    }
}

#[tokio::test]
async fn units_add_names_the_field_of_a_bad_name_and_creates_nothing() {
    let world = world();
    let before = world.audit_count();
    for name in ["+", "%01"] {
        let body = format!("parent_path=U1.2&segment=5&name={name}");
        let reply = htmx_post(&world, ROUTE, &body).await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
        assert!(
            reply.body.contains(&text(Lang::En, "field-unit-name")),
            "{name}"
        );
        assert!(reply.body.contains("href=\"#unit_name\""), "{name}");
        assert!(
            reply.body.contains(&text(Lang::En, "reason-name")),
            "{name}"
        );
    }
    assert_eq!(world.audit_count(), before);
}

#[tokio::test]
async fn units_add_rejects_a_missing_or_an_unknown_field() {
    let world = world();
    let bodies = [
        "parent_path=U1.2&segment=5",
        "segment=5&name=A",
        "parent_path=U1.2&segment=5&name=A&extra=1",
        "",
    ];
    for body in bodies {
        let reply = htmx_post(&world, ROUTE, body).await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    }
}

#[tokio::test]
async fn units_add_is_forbidden_for_an_operator_and_creates_nothing() {
    let world = world();
    let before = world.audit_count();
    let reply = world
        .post(
            &world.operator,
            ROUTE,
            "parent_path=U1.2&segment=5&name=A",
            true,
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"));
    assert_eq!(world.audit_count(), before);
}

#[tokio::test]
async fn units_add_without_htmx_shows_the_error_as_a_full_page() {
    let world = world();
    let reply = world
        .post(
            &world.engineer,
            ROUTE,
            "parent_path=U1.2&segment=1&name=A",
            false,
        )
        .await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert!(reply.body.contains("<html"), "{}", reply.body);
    assert!(reply.body.contains("E-PLT-004"));
}
