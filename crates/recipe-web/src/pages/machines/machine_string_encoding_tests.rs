//! Tests of the encoding on `GET /machines/{code}` and of `POST /machines/{code}/string-encoding`
//! (task 11.14, spec "STRING encoding on the machine page", ADR-0202).

use axum::body::Body;
use axum::http::{Method, StatusCode};
use recipe_domain::audit::SecurityEventKind;
use recipe_domain::plant::PlcPlatform;

use super::test_world::{seed_archived_machine, seed_live_machine, seed_live_machine_with_draft};
use crate::i18n::Lang;
use crate::pages::draft_reply::refusal_tests::assert_operator_refused;
use crate::pages::test_helpers::{assert_clean_html, assert_htmx_error, summary_lines};
use crate::test_support::plant::{HTMX, World, cookie_of, main_of, world};
use crate::test_support::{TestSession, call, icon_tag, post_fields, request};

/// The route of the command for `M9`.
const ROUTE: &str = "/machines/M9/string-encoding";
/// The opening tag of the form of `M9` when the page shows it.
const FORM_TAG: &str = "<form id=\"machine-string-encoding-form\" method=\"post\" action=\"/machines/M9/string-encoding\" hx-boost=\"true\" hx-target=\"#machine-string-encoding\" hx-swap=\"innerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\">";
/// The same tag when an answer swaps the form out of band.
const OOB_FORM_TAG: &str = "<form id=\"machine-string-encoding-form\" method=\"post\" action=\"/machines/M9/string-encoding\" hx-boost=\"true\" hx-target=\"#machine-string-encoding\" hx-swap=\"innerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\" hx-swap-oob=\"true\">";
/// The number of the archived machine of these tests.
const ARCHIVED_NUMBER: u32 = 31;

/// Return the fact that the page shows for the encoding `encoding`.
fn fact(encoding: &str) -> String {
    format!("<span id=\"machine-string-encoding\">{encoding}</span>")
}

/// Return true when `markup` holds a form that sends to a route of the encoding.
fn has_encoding_form(markup: &str) -> bool {
    markup.contains("/string-encoding\"")
}

/// Build the world with the live `CodesysIec` machine `M9` that has no recipe.
fn world_with_m9() -> World {
    let world = world();
    seed_live_machine(&world.fixture, 9, PlcPlatform::CodesysIec, None);
    world
}

/// Build the world with the live `CodesysIec` machine `M9` that has the draft version 1.
fn world_with_m9_and_draft() -> World {
    let world = world();
    seed_live_machine_with_draft(&world.fixture, 9, PlcPlatform::CodesysIec);
    world
}

/// Assert that `M9` still shows the encoding `Latin1` and that the refused request left no trace.
async fn assert_unchanged(world: &World, before: &(String, usize)) {
    assert_eq!(&world.fixture.snapshot(), before);
    let page = world.get_en(&world.engineer, "/machines/M9").await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.body);
    assert!(page.body.contains(&fact("Latin1")), "{}", page.body);
}

#[tokio::test]
async fn adr_0202_machine_page_shows_the_encoding() {
    let world = world();
    seed_live_machine(&world.fixture, 5, PlcPlatform::S7_1500, Some("5"));
    for lang in [Lang::En, Lang::Pl] {
        for session in [&world.operator, &world.engineer, &world.admin] {
            let reply = world.get(session, "/machines/M5", &[cookie_of(lang)]).await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
            let main = main_of(&reply.body);
            let label = crate::test_support::text(lang, "field-string-encoding");
            let row = format!(
                "<dt>{label}</dt>\n<dd class=\"data\">{}</dd>",
                fact("Latin1")
            );
            assert!(main.contains(&row), "{lang}: {main}");
        }
    }
}

#[tokio::test]
async fn adr_0202_machine_page_shows_the_encoding_of_an_archived_machine_and_no_form() {
    let world = world();
    seed_archived_machine(
        &world.fixture,
        ARCHIVED_NUMBER,
        PlcPlatform::CodesysIec,
        None,
    );
    let path = super::test_world::by_id_path(ARCHIVED_NUMBER);
    let reply = world.get_en(&world.engineer, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains(&fact("Latin1")), "{}", reply.body);
    assert!(!has_encoding_form(&reply.body), "{}", reply.body);
}

#[tokio::test]
async fn adr_0202_machine_page_shows_the_form_to_the_engineer_and_not_to_the_operator() {
    let world = world_with_m9();
    for session in [&world.engineer, &world.admin] {
        let reply = world.get_en(session, "/machines/M9").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        let (_before, rest) = main.split_once(FORM_TAG).expect(main);
        let form = rest.split("</form>").next().unwrap();
        let token = format!(
            "<input type=\"hidden\" name=\"csrf_token\" value=\"{}\">",
            session.csrf
        );
        assert!(form.contains(&token), "{form}");
        assert!(form.contains("<select id=\"string_encoding\" name=\"string_encoding\""));
        assert!(
            form.contains("<option value=\"Latin1\" selected>Latin1</option>"),
            "{form}"
        );
        assert!(
            form.contains("<option value=\"Utf8\">Utf8</option>"),
            "{form}"
        );
        assert!(
            form.contains("id=\"machine-string-encoding-submit\""),
            "{form}"
        );
        assert!(!form.contains("hx-swap-oob"), "{form}");
        assert_clean_html(&reply.body);
    }
    let reply = world.get_en(&world.operator, "/machines/M9").await;
    assert!(reply.body.contains(&fact("Latin1")), "{}", reply.body);
    assert!(!has_encoding_form(&reply.body), "{}", reply.body);
}

#[tokio::test]
async fn adr_0202_machine_page_set_through_htmx_answers_the_encoding_the_toast_and_the_form() {
    for lang in [Lang::En, Lang::Pl] {
        let world = world_with_m9();
        let before = world.audit_count();
        let reply = post_fields(
            &world.fixture.router(),
            ROUTE,
            Some(&world.engineer),
            &[cookie_of(lang), HTMX],
            "string_encoding=Utf8",
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(
            reply.body.starts_with("Utf8<div id=\"toast\""),
            "{}",
            reply.body
        );
        let toast = format!(
            "<div id=\"toast\" hx-swap-oob=\"innerHTML\"><div class=\"toast\">{}",
            icon_tag("alert-success")
        );
        assert!(reply.body.contains(&toast), "{lang}: {}", reply.body);
        let sentence = {
            let bundles = crate::i18n::Bundles::load().unwrap();
            let t = crate::pages::Texts::new(&bundles, lang);
            let args = fluent_args(&[("code", "M9"), ("encoding", "Utf8")]);
            t.with("machine-string-encoding-set", &args)
        };
        assert!(
            reply.body.contains(&format!("<span>{sentence}</span>")),
            "{lang}: {}",
            reply.body
        );
        assert!(
            reply
                .body
                .contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>"),
            "{lang}: {}",
            reply.body
        );
        let form = reply.body.split(OOB_FORM_TAG).nth(1).expect(&reply.body);
        assert!(
            form.contains("<option value=\"Utf8\" selected>Utf8</option>"),
            "{form}"
        );
        for mark in ["aria-invalid", "field-error"] {
            assert!(!form.contains(mark), "{mark}: {form}");
        }
        assert_clean_html(&reply.body);
        assert_eq!(world.audit_count(), before.checked_add(1).unwrap());
        let page = world.get_en(&world.engineer, "/machines/M9").await;
        assert!(page.body.contains(&fact("Utf8")), "{}", page.body);
    }
}

/// Build the arguments of a text from pairs of a name and a value.
fn fluent_args<'a>(pairs: &[(&'a str, &'a str)]) -> fluent::FluentArgs<'a> {
    let mut args = fluent::FluentArgs::new();
    for (name, value) in pairs {
        args.set(*name, *value);
    }
    args
}

#[tokio::test]
async fn adr_0202_machine_page_set_without_htmx_redirects_to_the_machine_page() {
    let world = world_with_m9();
    let reply = world
        .post(&world.engineer, ROUTE, "string_encoding=Utf8", false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M9");
    let page = world.get_en(&world.engineer, "/machines/M9").await;
    assert!(page.body.contains(&fact("Utf8")), "{}", page.body);
}

#[tokio::test]
async fn adr_0202_machine_page_has_no_form_on_a_machine_with_a_version() {
    let world = world_with_m9_and_draft();
    for session in [&world.engineer, &world.admin] {
        let reply = world.get_en(session, "/machines/M9").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(reply.body.contains(&fact("Latin1")), "{}", reply.body);
        assert!(!has_encoding_form(&reply.body), "{}", reply.body);
    }
}

#[tokio::test]
async fn adr_0202_machine_page_frozen_encoding_through_htmx_gives_409_e_plt_054() {
    let world = world_with_m9_and_draft();
    let before = world.fixture.snapshot();
    let reply = world
        .post(&world.engineer, ROUTE, "string_encoding=Utf8", true)
        .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-PLT-054");
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert_eq!(reply.header("hx-push-url"), "false");
    assert_unchanged(&world, &before).await;
}

#[tokio::test]
async fn adr_0202_machine_page_has_no_form_on_an_s7_machine() {
    let world = world();
    seed_live_machine(&world.fixture, 5, PlcPlatform::S7_1500, Some("5"));
    for session in [&world.engineer, &world.admin] {
        let reply = world.get_en(session, "/machines/M5").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(reply.body.contains(&fact("Latin1")), "{}", reply.body);
        assert!(!has_encoding_form(&reply.body), "{}", reply.body);
    }
}

#[tokio::test]
async fn adr_0202_machine_page_utf8_on_an_s7_machine_gives_422_e_plt_053_with_the_line() {
    let world = world();
    seed_live_machine(&world.fixture, 5, PlcPlatform::S7_1500, Some("5"));
    let before = world.fixture.snapshot();
    let reply = world
        .post(
            &world.engineer,
            "/machines/M5/string-encoding",
            "string_encoding=Utf8",
            true,
        )
        .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-053");
    assert_eq!(summary_lines(&reply.body), 1, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<div class=\"field-error\" id=\"string_encoding-error\">"),
        "{}",
        reply.body
    );
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn adr_0202_machine_page_read_only_mode_hides_the_form_and_refuses_a_post() {
    let world = world_with_m9();
    world.fixture.state.read_only().set(true);
    let before = world.fixture.snapshot();
    let page = world.get_en(&world.engineer, "/machines/M9").await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.body);
    assert!(page.body.contains(&fact("Latin1")), "{}", page.body);
    assert!(!has_encoding_form(&page.body), "{}", page.body);
    for htmx in [true, false] {
        let reply = world
            .post(&world.engineer, ROUTE, "string_encoding=Utf8", htmx)
            .await;
        assert_eq!(
            reply.status,
            StatusCode::SERVICE_UNAVAILABLE,
            "{}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-006"), "{}", reply.body);
        assert!(!reply.body.contains("<form"), "{}", reply.body);
        assert_unchanged(&world, &before).await;
    }
}

#[tokio::test]
async fn adr_0202_machine_page_operator_post_gives_403_e_web_002_and_a_security_event() {
    let world = world_with_m9();
    for body in [
        "string_encoding=Utf8",
        "string_encoding=Latin1",
        "string_encoding=",
    ] {
        assert_operator_refused(&world, ROUTE, body).await;
    }
    let page = world.get_en(&world.engineer, "/machines/M9").await;
    assert!(page.body.contains(&fact("Latin1")), "{}", page.body);
}

/// Send a same-origin `POST` as `session` with the token of `token` in the header, or with none.
async fn post_with_token(world: &World, session: &TestSession, token: Option<&str>, htmx: bool) {
    let mut builder = request(Method::POST, ROUTE)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", session.cookie.as_str());
    if let Some(token) = token {
        builder = builder.header("x-csrf-token", token);
    }
    if htmx {
        builder = builder.header(HTMX.0, HTMX.1);
    }
    let before = world.fixture.snapshot();
    let events = world.fixture.sink.events().len();
    let reply = call(
        &world.fixture.router(),
        builder.body(Body::from("string_encoding=Utf8")).unwrap(),
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-007"), "{}", reply.body);
    let after = world.fixture.sink.events();
    assert_eq!(
        after.len(),
        events.checked_add(1).unwrap(),
        "{token:?} {htmx}"
    );
    let kind = after.last().map(|event| event.kind());
    assert_eq!(
        kind,
        Some(SecurityEventKind::CsrfRejected),
        "{token:?} {htmx}"
    );
    assert_unchanged(world, &before).await;
}

#[tokio::test]
async fn adr_0202_machine_page_post_without_a_valid_csrf_token_gives_403_e_web_007() {
    let world = world_with_m9();
    let forged = "0".repeat(world.engineer.csrf.len());
    for htmx in [true, false] {
        post_with_token(&world, &world.engineer, None, htmx).await;
        post_with_token(&world, &world.engineer, Some(&forged), htmx).await;
    }
}

#[tokio::test]
async fn adr_0202_machine_page_post_of_an_unknown_or_archived_machine_gives_404_e_web_005() {
    let world = world();
    seed_archived_machine(
        &world.fixture,
        ARCHIVED_NUMBER,
        PlcPlatform::CodesysIec,
        None,
    );
    let before = world.fixture.snapshot();
    for code in ["M99", "M31"] {
        let path = format!("/machines/{code}/string-encoding");
        let reply = world
            .post(&world.engineer, &path, "string_encoding=Utf8", true)
            .await;
        assert_htmx_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005");
    }
    assert_eq!(world.fixture.snapshot(), before);
}

/// ADR-0138: a changed form body gives one `E-WEB-008` line and no out-of-band form.
#[tokio::test]
async fn adr_0202_machine_page_post_with_a_changed_body_gives_one_e_web_008_line_and_no_form() {
    let world = world_with_m9();
    let before = world.fixture.snapshot();
    for body in [
        "string_encoding=Utf8&extra=1",
        "other=Utf8",
        "",
        "string_encoding=Utf8&string_encoding=Latin1",
        "string_encoding[0]=Utf8",
    ] {
        let reply = world.post(&world.engineer, ROUTE, body, true).await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
        assert_eq!(summary_lines(&reply.body), 1, "{body}: {}", reply.body);
        assert!(!reply.body.contains("<form"), "{body}: {}", reply.body);
        assert_eq!(world.fixture.snapshot(), before, "{body}");
    }
}

#[tokio::test]
async fn adr_0202_machine_page_post_with_a_bad_encoding_gives_a_line_and_never_shows_the_text() {
    let world = world_with_m9();
    let before = world.fixture.snapshot();
    for body in [
        "string_encoding=utf8",
        "string_encoding=",
        "string_encoding=%3Cb%3E",
    ] {
        let reply = world.post(&world.engineer, ROUTE, body, true).await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
        assert_eq!(summary_lines(&reply.body), 1, "{body}: {}", reply.body);
        assert!(reply.body.contains(OOB_FORM_TAG), "{body}: {}", reply.body);
        assert!(
            reply
                .body
                .contains("<div class=\"field-error\" id=\"string_encoding-error\">"),
            "{body}: {}",
            reply.body
        );
        assert!(!reply.body.contains("<b>"), "{body}: {}", reply.body);
        assert!(!reply.body.contains("utf8"), "{body}: {}", reply.body);
        assert_eq!(world.fixture.snapshot(), before, "{body}");
    }
}
