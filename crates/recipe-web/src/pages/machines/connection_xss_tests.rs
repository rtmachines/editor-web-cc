//! The XSS corpus through the host, address, and login fields of the connection form (task 7.9 of
//! `plc-connection-config`, ADR-0154, design D2 and D8).
//!
//! Each field gets every payload of `tests/web/xss-corpus.txt` and one NUL payload. A text that the
//! domain refuses gives the line `E-WEB-008` and shows again escaped in the refused form. A text that
//! the domain accepts shows escaped in the saved form and on the machine page. No answer holds the
//! raw payload, no `hx-*` attribute holds it, and each `<svg` is a fixed icon tag.

use axum::http::StatusCode;
use recipe_domain::plant::PlcPlatform;

use super::connection_tests::seed;
use crate::pages::test_helpers::{askama_escape, encode, payloads};
use crate::pages::xss_kit::{assert_error_panel, assert_refused, assert_safe, has_control};
use crate::test_support::Reply;
use crate::test_support::plant::{World, world};

/// The number of the `CodesysIec` machine that the tests use. Its mode `OpcUa` takes a free address.
const IEC_NUMBER: u32 = 102;
/// The longest login (`plc_login.rs` of `recipe-domain`).
const MAX_LOGIN_CHARS: usize = 64;
/// The longest address (`plc_address.rs` of `recipe-domain`).
const MAX_ADDRESS_CHARS: usize = 200;
/// A valid form of the IEC machine. The mode `OpcUa` needs a password. A test replaces one value.
const IEC_FIELDS: [(&str, &str); 7] = [
    ("row_version", "0"),
    ("host", "10.0.0.7"),
    ("port", "4840"),
    ("address", "opc.tcp://plc"),
    ("comm_mode", "OpcUa"),
    ("login", "adapter"),
    ("password", "S3cret-PLC"),
];
/// A valid form of the S7 machine `M7`. A test replaces the host.
const S7_FIELDS: [(&str, &str); 4] = [
    ("row_version", "0"),
    ("host", "10.0.0.5"),
    ("rack", "0"),
    ("slot", "2"),
];

/// Return the form body of `fields` with the value of `field` replaced by `value`.
fn body_of(fields: &[(&str, &str)], field: &str, value: &str) -> String {
    fields
        .iter()
        .map(|(name, valid)| {
            let text = if *name == field { value } else { valid };
            format!("{name}={}", encode(text))
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// Return true when the domain refuses the text: a control character or more than `max` characters.
fn is_refused(payload: &str, max: usize) -> bool {
    has_control(payload) || payload.chars().count() > max
}

/// Send the save form `body` for `code` as the Process Engineer, as an HTMX request.
async fn save(world: &World, code: &str, body: &str) -> Reply {
    let path = format!("/machines/{code}/connection");
    world.post(&world.engineer, &path, body, true).await
}

/// Assert the refused answer of a save: `E-WEB-008`, the error panel, and the sent text shown
/// escaped in the form of the answer when `shown_again` is true (ADR-0041 (e)).
fn assert_refused_form(reply: &Reply, field: &str, payload: &str, shown_again: bool) {
    let what = format!("POST connection {field}");
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{what}");
    assert_refused(reply, "E-WEB-008", payload, &what);
    assert_error_panel(reply, &what);
    if shown_again {
        let value = format!("value=\"{}\"", askama_escape(payload));
        assert!(reply.body.contains(&value), "{what}: {}", reply.body);
    }
}

#[tokio::test]
async fn connection_xss_host_payloads_are_refused_and_shown_escaped_in_the_form() {
    let cases = [("M7", &S7_FIELDS[..]), ("M102", &IEC_FIELDS[..])];
    for payload in payloads() {
        for (code, fields) in cases {
            let world = world();
            seed(&world, IEC_NUMBER, PlcPlatform::CodesysIec);
            let before = world.fixture.snapshot();
            let reply = save(&world, code, &body_of(fields, "host", &payload)).await;
            // No payload is a host name or an IPv4 address, so every one gives a line.
            assert_refused_form(&reply, "host", &payload, !has_control(&payload));
            assert_eq!(world.fixture.snapshot(), before, "{code}: nothing is saved");
        }
    }
}

#[tokio::test]
async fn connection_xss_address_and_login_are_stored_and_shown_escaped_on_both_pages() {
    let (mut accepted, mut refused) = (0_usize, 0_usize);
    for payload in payloads() {
        for (field, max) in [("address", MAX_ADDRESS_CHARS), ("login", MAX_LOGIN_CHARS)] {
            let world = world();
            seed(&world, IEC_NUMBER, PlcPlatform::CodesysIec);
            let reply = save(&world, "M102", &body_of(&IEC_FIELDS, field, &payload)).await;
            if is_refused(&payload, max) {
                refused = refused.checked_add(1).unwrap();
                assert_refused_form(&reply, field, &payload, !has_control(&payload));
                continue;
            }
            accepted = accepted.checked_add(1).unwrap();
            let what = format!("POST connection {field}");
            assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
            assert_safe(&reply, &payload, &what);
            assert_stored_and_shown(&world, field, &payload).await;
        }
    }
    assert!(accepted > 0, "no payload was accepted");
    assert!(refused > 0, "no payload was refused");
}

/// Assert that the edit form and the machine page show the saved text escaped.
async fn assert_stored_and_shown(world: &World, field: &str, payload: &str) {
    let escaped = askama_escape(payload);
    let form = world
        .get_en(&world.engineer, "/machines/M102/connection/edit")
        .await;
    let page = world.get_en(&world.engineer, "/machines/M102").await;
    for (reply, shown) in [
        (&form, format!("value=\"{escaped}\"")),
        (&page, format!("<dd class=\"data\">{escaped}</dd>")),
    ] {
        let what = format!("shows {field}");
        assert_eq!(reply.status, StatusCode::OK, "{what}");
        assert!(reply.body.contains(&shown), "{what}: {}", reply.body);
        assert_safe(reply, payload, &what);
    }
}
