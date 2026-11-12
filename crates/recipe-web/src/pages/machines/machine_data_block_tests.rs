//! Tests of the data block on `GET /machines/{code}` (task 6.2, specs "Machine page" and "Data
//! block form of a machine", design D8).

use axum::http::StatusCode;

use super::test_world::{
    by_id_path, seed_archived_machine, seed_live_machine, seed_machine_with_archived_unit,
};
use crate::i18n::Lang;
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{cookie_of, main_of, world};
use crate::test_support::text;
use recipe_domain::plant::PlcPlatform;

/// The opening tag of the data block form of `M7` (design D8).
const FORM_TAG: &str = "<form id=\"machine-data-block-form\" method=\"post\" action=\"/machines/M7/data-block\" hx-boost=\"true\" hx-target=\"#machine-db-number\" hx-swap=\"innerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\">";
/// The fixed number of the archived S7 machine with a number.
const ARCHIVED_WITH_NUMBER: u32 = 31;
/// The fixed number of the archived S7 machine without a number.
const ARCHIVED_WITHOUT_NUMBER: u32 = 32;

/// Return true when `markup` holds a form that sends to a data block route.
fn has_data_block_form(markup: &str) -> bool {
    markup.contains("/data-block\"")
}

#[tokio::test]
async fn machines_page_of_an_operator_shows_the_data_block_number_of_an_s7_machine() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.operator, "/machines/M7", &[cookie_of(lang)])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        let label = text(lang, "field-db-number");
        let fact = format!(
            "<dt>{label}</dt>\n<dd class=\"data\"><span id=\"machine-db-number\">1</span></dd>"
        );
        assert!(main.contains(&fact), "{main}");
        assert!(!has_data_block_form(main), "{main}");
    }
}

#[tokio::test]
async fn machines_page_of_an_iec_machine_shows_no_data_block_fact_and_no_form() {
    let world = world();
    seed_machine_with_archived_unit(&world.fixture);
    for session in [&world.engineer, &world.admin] {
        let reply = world.get_en(session, "/machines/M20").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        assert!(!main.contains("Data block"), "{main}");
        assert!(!main.contains("machine-db-number"), "{main}");
        assert!(!has_data_block_form(main), "{main}");
    }
}

#[tokio::test]
async fn machines_page_of_an_archived_s7_machine_without_a_number_shows_the_none_text() {
    let world = world();
    seed_archived_machine(
        &world.fixture,
        ARCHIVED_WITHOUT_NUMBER,
        PlcPlatform::S7_1200,
        None,
    );
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(
                &world.engineer,
                &by_id_path(ARCHIVED_WITHOUT_NUMBER),
                &[cookie_of(lang)],
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        let fact = format!(
            "<dt>{}</dt>\n<dd>{}</dd>",
            text(lang, "field-db-number"),
            text(lang, "machine-db-number-none")
        );
        assert!(main.contains(&fact), "{main}");
        assert!(!main.contains("machine-db-number\""), "{main}");
        assert!(
            !main.contains("<form"),
            "an archived machine has no form: {main}"
        );
    }
}

#[tokio::test]
async fn machines_page_of_an_archived_s7_machine_with_a_number_shows_it_and_no_form() {
    let world = world();
    seed_archived_machine(
        &world.fixture,
        ARCHIVED_WITH_NUMBER,
        PlcPlatform::S7_1500,
        Some("42"),
    );
    let reply = world
        .get_en(&world.engineer, &by_id_path(ARCHIVED_WITH_NUMBER))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains("<span id=\"machine-db-number\">42</span>"),
        "{main}"
    );
    assert!(!has_data_block_form(main), "{main}");
}

#[tokio::test]
async fn machines_page_of_an_engineer_has_the_data_block_form_with_the_current_number() {
    let world = world();
    for session in [&world.engineer, &world.admin] {
        let reply = world.get_en(session, "/machines/M7").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        assert!(main.contains(FORM_TAG), "{main}");
        let (_before, rest) = main.split_once(FORM_TAG).unwrap();
        let form = rest.split("</form>").next().unwrap();
        let token = format!(
            "<input type=\"hidden\" name=\"csrf_token\" value=\"{}\">",
            session.csrf
        );
        assert!(form.contains(&token), "{form}");
        assert!(
            form.contains("<label for=\"db_number\">Data block"),
            "{form}"
        );
        assert!(
            form.contains("id=\"db_number\" name=\"db_number\""),
            "{form}"
        );
        assert!(form.contains(" value=\"1\""), "{form}");
        assert!(
            form.contains("<button type=\"submit\" id=\"machine-data-block-submit\">"),
            "{form}"
        );
        assert!(!form.contains("method=\"get\""));
        assert!(!form.contains("hx-swap-oob"), "{form}");
        assert_clean_html(&reply.body);
    }
}

/// The text of the hint of the data block form in `main`.
fn data_block_hint_of(main: &str) -> &str {
    let (_before, rest) = main
        .split_once("<form id=\"machine-data-block-form\"")
        .unwrap();
    let form = rest.split("</form>").next().unwrap();
    let (_label, hint) = form
        .split_once("<span class=\"field-hint\" id=\"db_number-hint\">")
        .unwrap();
    hint.split("</span>").next().unwrap()
}

#[tokio::test]
async fn machines_page_data_block_hint_names_only_the_range_of_the_platform() {
    let world = world();
    seed_live_machine(&world.fixture, 8, PlcPlatform::S7_1500, Some("5"));
    for lang in [Lang::En, Lang::Pl] {
        let s7_300 = world
            .get(&world.engineer, "/machines/M7", &[cookie_of(lang)])
            .await;
        let hint = data_block_hint_of(main_of(&s7_300.body));
        assert!(hint.contains("16000"), "{hint}");
        assert!(!hint.contains("59999"), "{hint}");
        let s7_1500 = world
            .get(&world.engineer, "/machines/M8", &[cookie_of(lang)])
            .await;
        let hint = data_block_hint_of(main_of(&s7_1500.body));
        assert!(hint.contains("59999"), "{hint}");
        assert!(!hint.contains("16000"), "{hint}");
        for hint in [hint, data_block_hint_of(main_of(&s7_300.body))] {
            assert!(!hint.contains("empty"), "{hint}");
            assert!(!hint.contains("puste"), "{hint}");
            assert_ne!(hint, text(lang, "machine-db-number-hint"));
        }
    }
}
