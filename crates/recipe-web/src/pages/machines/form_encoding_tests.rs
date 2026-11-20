//! Tests of the field `string_encoding` of `GET /machines/new` and `POST /machines`
//! (task 11.11 of `codesys-twincat-type-rules`, ADR-0202, spec "STRING encoding field of the
//! machine form").

use axum::http::StatusCode;

use crate::i18n::Lang;
use crate::pages::test_helpers::{assert_clean_html, assert_htmx_error};
use crate::test_support::plant::{cookie_of, world};
use crate::test_support::text;

/// The opening tag of the create form when an error answer swaps it out of band.
const OOB_FORM_TAG: &str = "<form id=\"machine-new-form\" method=\"post\" action=\"/machines\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\" hx-swap-oob=\"true\">";

/// The part of the page that holds the select of the encoding.
fn encoding_select(body: &str) -> &str {
    let (_before, rest) = body.split_once("<select id=\"string_encoding\"").unwrap();
    rest.split("</select>").next().unwrap()
}

#[tokio::test]
async fn adr_0202_machine_form_offers_latin1_and_utf8_and_selects_latin1() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.engineer, "/machines/new", &[cookie_of(lang)])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let label = format!(
            "<label for=\"string_encoding\">{}",
            text(lang, "field-string-encoding")
        );
        assert!(reply.body.contains(&label), "{lang}: {}", reply.body);
        let hint = format!(
            "<span class=\"field-hint\" id=\"string_encoding-hint\">{}</span>",
            text(lang, "machine-string-encoding-hint")
        );
        assert!(reply.body.contains(&hint), "{lang}: {}", reply.body);
        let select = encoding_select(&reply.body);
        assert!(
            select.contains("<option value=\"Latin1\" selected>Latin1</option>"),
            "{lang}: {select}"
        );
        assert!(
            select.contains("<option value=\"Utf8\">Utf8</option>"),
            "{lang}: {select}"
        );
        assert_eq!(select.matches("<option").count(), 2, "{select}");
        assert_eq!(select.matches(" selected").count(), 1, "{select}");
    }
}

#[tokio::test]
async fn adr_0202_machine_form_creates_a_utf8_machine_on_an_iec_platform() {
    let world = world();
    let before = world.audit_count();
    let body =
        "code=M10&name=Press&plc_platform=CodesysIec&string_encoding=Utf8&main_unit_name=Infeed";
    let reply = world.post(&world.engineer, "/machines", body, false).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M10");
    let entries = world.audit_count().checked_sub(before).unwrap();
    assert_eq!(entries, 2, "one entry for the machine and one for U1");
}

#[tokio::test]
async fn adr_0202_machine_form_puts_e_plt_053_under_the_field_and_keeps_the_sent_encoding() {
    let world = world();
    let before = world.audit_count();
    let body = "code=M7&name=Press&plc_platform=S7_1500&db_number=100&string_encoding=Utf8&main_unit_name=Infeed";
    let reply = world.post(&world.engineer, "/machines", body, true).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-053");
    assert!(
        reply.body.contains("href=\"#string_encoding\""),
        "{}",
        reply.body
    );
    let form = reply.body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert_eq!(form.matches("aria-invalid=\"true\"").count(), 1, "{form}");
    let (_before, line) = form
        .split_once("<div class=\"field-error\" id=\"string_encoding-error\">")
        .unwrap();
    assert!(line.contains("E-PLT-053"), "{form}");
    let select = encoding_select(form);
    assert!(
        select.contains("<option value=\"Utf8\" selected>Utf8</option>"),
        "{select}"
    );
    assert_eq!(select.matches(" selected").count(), 1, "{select}");
    assert_clean_html(&reply.body);
    assert_eq!(
        world.audit_count(),
        before,
        "a refused form creates nothing"
    );
}

#[tokio::test]
async fn adr_0202_machine_form_puts_an_unknown_encoding_under_the_field_without_echo() {
    let world = world();
    let body = "code=M7&name=Press&plc_platform=TwinCat3&string_encoding=%3Cimg+src%3Dx%3E&main_unit_name=Infeed";
    let reply = world.post(&world.engineer, "/machines", body, true).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    let form = reply.body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert!(form.contains("id=\"string_encoding-error\""), "{form}");
    assert!(!reply.body.contains("<img src=x>"), "{}", reply.body);
    let select = encoding_select(form);
    assert_eq!(select.matches(" selected").count(), 0, "{select}");
}

#[tokio::test]
async fn adr_0202_machine_form_rejects_a_missing_encoding_like_a_missing_platform() {
    let world = world();
    let bodies = [
        "code=M7&name=Press&plc_platform=TwinCat3&main_unit_name=Infeed",
        "code=M7&name=Press&string_encoding=Latin1&main_unit_name=Infeed",
    ];
    let mut codes = Vec::new();
    for body in bodies {
        let reply = world.post(&world.engineer, "/machines", body, true).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert!(reply.body.contains("E-WEB-008"), "{body}: {}", reply.body);
        codes.push(reply.body.contains(OOB_FORM_TAG));
    }
    assert_eq!(codes, [false, false], "a missing key swaps no form");
}
