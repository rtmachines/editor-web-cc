//! Tests of the connection record on `GET /machines/{code}` (task 7.7 of `plc-connection-config`,
//! ADR-0154, design D8). The page shows the record and the password state. It never shows a password,
//! a password input, or a sealed text.

use axum::http::StatusCode;
use recipe_app::ports::SealedSecret;
use recipe_domain::plant::{MachineId, PlcConnection, PlcHost, PlcPlatform, Rack, Slot};

use super::connection_tests::{ads_record, seed};
use crate::i18n::Lang;
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{M7_ID, M12_ID, cookie_of, main_of, world};
use crate::test_support::text;

/// The text that a sealed password of a test record holds. It must never reach a page.
const SEALED_TEXT: &str = "S3cret-PLC";

/// Return the value of the `dd` that follows the `dt` with `label`.
fn value_of<'a>(main: &'a str, label: &str) -> &'a str {
    let marker = format!("<dt>{label}</dt>");
    let found = main.split_once(&marker);
    assert!(found.is_some(), "no row {label}: {main}");
    let (_before, rest) = found.unwrap();
    let (_tag, after) = rest.split_once("<dd").unwrap();
    let (_attributes, value) = after.split_once('>').unwrap();
    value.split("</dd>").next().unwrap()
}

/// Assert that the markup has no password input and no sealed text.
fn assert_no_secret_surface(body: &str) {
    assert!(!body.contains("type=\"password\""), "{body}");
    assert!(!body.contains("name=\"password\""), "{body}");
    assert!(!body.contains(SEALED_TEXT), "{body}");
}

/// Assert that the markup has no row with the label of `key`.
fn assert_no_row(main: &str, lang: Lang, key: &str) {
    let label = format!("<dt>{}</dt>", text(lang, key));
    assert!(!main.contains(&label), "no row {key} expected: {main}");
}

/// A record of an S7 machine.
fn s7_record() -> PlcConnection {
    PlcConnection::S7 {
        host: PlcHost::parse("10.0.0.5").unwrap(),
        rack: Rack::parse("0").unwrap(),
        slot: Slot::parse("2").unwrap(),
    }
}

#[tokio::test]
async fn machine_page_shows_the_s7_record_without_a_password_state() {
    let world = world();
    world
        .fixture
        .store
        .seed_connection(MachineId::from(M7_ID), &s7_record(), None);
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.operator, "/machines/M7", &[cookie_of(lang)])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        assert_eq!(value_of(main, &text(lang, "field-plc-host")), "10.0.0.5");
        assert_eq!(value_of(main, &text(lang, "field-plc-rack")), "0");
        assert_eq!(value_of(main, &text(lang, "field-plc-slot")), "2");
        // The data block number is a fact of the machine, so it shows once and not in the record
        // (ADR-0156, design D8).
        let label = format!("<dt>{}</dt>", text(lang, "field-db-number"));
        assert_eq!(main.matches(&label).count(), 1, "{main}");
        assert!(!main.contains("Default data block"), "{main}");
        assert!(!main.contains("Domyślny blok danych"), "{main}");
        for key in ["field-plc-port", "field-plc-address", "field-plc-password"] {
            assert_no_row(main, lang, key);
        }
        assert!(
            !main.contains(&text(lang, "machine-connection-none")),
            "{main}"
        );
        assert_no_secret_surface(&reply.body);
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn machine_page_shows_the_iec_record_and_a_stored_password_state() {
    let world = world();
    let id = seed(&world, 101, PlcPlatform::TwinCat3);
    let sealed = SealedSecret::from_bytes(SEALED_TEXT.as_bytes().to_vec());
    world
        .fixture
        .store
        .seed_connection(id, &ads_record(), Some(sealed));
    for lang in [Lang::En, Lang::Pl] {
        // An Operator reads the page, so the record is part of `RecipeView` (design D8).
        for session in [&world.operator, &world.engineer, &world.admin] {
            let reply = world
                .get(session, "/machines/M101", &[cookie_of(lang)])
                .await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
            let main = main_of(&reply.body);
            assert_eq!(
                value_of(main, &text(lang, "field-plc-host")),
                "192.168.1.10"
            );
            assert_eq!(value_of(main, &text(lang, "field-plc-port")), "48898");
            assert_eq!(
                value_of(main, &text(lang, "field-plc-address")),
                "192.168.1.10.1.1"
            );
            assert_eq!(
                value_of(main, &text(lang, "field-plc-comm-mode")),
                text(lang, "connection-mode-ads")
            );
            assert_eq!(value_of(main, &text(lang, "field-plc-login")), "adapter");
            assert_eq!(
                value_of(main, &text(lang, "field-plc-password")),
                text(lang, "machine-connection-password-stored")
            );
            assert!(!main.contains(&text(lang, "machine-connection-password-none")));
            assert_no_secret_surface(&reply.body);
        }
    }
}

#[tokio::test]
async fn machine_page_shows_no_password_when_the_record_holds_none() {
    let world = world();
    let id = seed(&world, 101, PlcPlatform::TwinCat3);
    world.fixture.store.seed_connection(id, &ads_record(), None);
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.operator, "/machines/M101", &[cookie_of(lang)])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        assert_eq!(
            value_of(main, &text(lang, "field-plc-password")),
            text(lang, "machine-connection-password-none")
        );
        assert!(!main.contains(&text(lang, "machine-connection-password-stored")));
        assert_no_secret_surface(&reply.body);
    }
}

#[tokio::test]
async fn machine_page_without_a_record_says_so_and_shows_no_field() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.operator, "/machines/M7", &[cookie_of(lang)])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        assert!(
            main.contains(&text(lang, "machine-connection-title")),
            "{main}"
        );
        assert!(
            main.contains(&text(lang, "machine-connection-none")),
            "{main}"
        );
        for key in ["field-plc-host", "field-plc-password"] {
            assert_no_row(main, lang, key);
        }
        assert_no_secret_surface(&reply.body);
    }
}

// --- Link to the connection form (fix G8) --------------------------------------------------------

/// The link to the connection form of the machine `M7`.
const EDIT_LINK: &str = "href=\"/machines/M7/connection/edit\"";

/// Return the markup of the connection section: from its heading to the next heading.
fn connection_section(main: &str, lang: Lang) -> &str {
    let heading = format!("<h2>{}</h2>", text(lang, "machine-connection-title"));
    let found = main.split_once(&heading);
    assert!(found.is_some(), "no connection section: {main}");
    let (_before, rest) = found.unwrap();
    rest.split("<h2>").next().unwrap()
}

#[tokio::test]
async fn machine_page_links_a_manager_to_the_connection_form_with_and_without_a_record() {
    for stored in [false, true] {
        let world = world();
        if stored {
            world
                .fixture
                .store
                .seed_connection(MachineId::from(M7_ID), &s7_record(), None);
        }
        for lang in [Lang::En, Lang::Pl] {
            for session in [&world.admin, &world.engineer] {
                let reply = world.get(session, "/machines/M7", &[cookie_of(lang)]).await;
                assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
                let section = connection_section(main_of(&reply.body), lang);
                assert_eq!(section.matches(EDIT_LINK).count(), 1, "{section}");
                assert!(
                    section.contains(&text(lang, "machine-connection-edit")),
                    "{section}"
                );
                assert!(!section.contains("hx-"), "{section}");
                assert_clean_html(&reply.body);
            }
        }
    }
}

#[tokio::test]
async fn machine_page_shows_an_operator_no_link_to_the_connection_form() {
    let world = world();
    world
        .fixture
        .store
        .seed_connection(MachineId::from(M7_ID), &s7_record(), None);
    let reply = world.get_en(&world.operator, "/machines/M7").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("/connection/edit"), "{}", reply.body);
}

#[tokio::test]
async fn machine_page_of_an_archived_machine_has_no_link_to_the_connection_form() {
    let world = world();
    for session in [&world.admin, &world.engineer, &world.operator] {
        let reply = world
            .get_en(session, &format!("/machines/by-id/{M12_ID}"))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(!reply.body.contains("/connection/edit"), "{}", reply.body);
    }
}

/// Read-only mode follows the rule of the other edit links (ADR-0113 (b)): the page has no link.
#[tokio::test]
async fn machine_page_in_read_only_mode_has_no_link_to_the_connection_form() {
    let world = world();
    world.fixture.state.read_only().set(true);
    for session in [&world.admin, &world.engineer] {
        let reply = world.get_en(session, "/machines/M7").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(!reply.body.contains("/connection/edit"), "{}", reply.body);
    }
}
