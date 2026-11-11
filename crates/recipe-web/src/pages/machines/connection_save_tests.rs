//! Tests of `POST /machines/{code}/connection` (task 7.2 of `plc-connection-config`, ADR-0154,
//! design D6 and D8). The tests of the request struct (7.3), the field lines (7.4), the leak check
//! (7.5), and the refusals (7.6) come in their own tasks.

use axum::http::StatusCode;
use recipe_app::memory::{CipherCall, JournalEntry};
use recipe_app::ports::Isolation;
use recipe_domain::plant::PlcPlatform;

use super::connection_tests::seed;
use crate::i18n::Lang;
use crate::pages::test_helpers::{assert_clean_html, assert_error, field_value};
use crate::test_support::plant::{HTMX, World, cookie_of, main_of, text_with, world};
use crate::test_support::text;

/// A valid body for the S7 machine `M7` without a record.
const S7_BODY: &str = "row_version=0&host=10.0.0.5&rack=0&slot=2&db_number=1";
/// A valid body for a `TwinCat3` machine without a record: mode `Ads` without a login.
const ADS_BODY: &str = "row_version=0&host=10.0.0.6&port=48898&address=192.168.1.10.1.1&comm_mode=Ads&login=&password=";
/// A valid body for a `CodesysIec` machine without a record: mode `OpcUa` with a login.
const OPC_BODY: &str = "row_version=0&host=10.0.0.7&port=4840&address=opc.tcp%3A%2F%2Fplc&comm_mode=OpcUa&login=adapter&password=S3cret-PLC";

// --- Success: 200 with HX-Trigger, 303 without HX-Request --------------------------------------

#[tokio::test]
async fn connection_save_with_htmx_answers_200_with_the_new_row_version_in_hx_trigger() {
    let world = world();
    let reply = world
        .post(&world.engineer, "/machines/M7/connection", S7_BODY, true)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 1}");
    assert_eq!(
        reply.header("hx-retarget"),
        "",
        "a success retargets nothing"
    );
    assert_clean_html(&reply.body);
    // The second save sends the new token and gets the next one.
    let again = S7_BODY.replace("row_version=0", "row_version=1");
    let second = world
        .post(&world.admin, "/machines/M7/connection", &again, true)
        .await;
    assert_eq!(second.status, StatusCode::OK, "{}", second.body);
    assert_eq!(second.header("hx-trigger"), "{\"rowVersion\": 2}");
}

#[tokio::test]
async fn connection_save_writes_one_audit_entry_and_the_form_shows_the_saved_values() {
    let world = world();
    let before = world.audit_count();
    let reply = world
        .post(&world.engineer, "/machines/M7/connection", S7_BODY, true)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(world.audit_count(), before.checked_add(1).unwrap());
    let page = world
        .get_en(&world.engineer, "/machines/M7/connection/edit")
        .await;
    assert_eq!(field_value(&page.body, "row_version"), "1");
    let form = main_of(&page.body);
    for value in ["10.0.0.5", "0", "2", "1"] {
        assert!(form.contains(&format!(" value=\"{value}\"")), "{value}");
    }
}

#[tokio::test]
async fn connection_save_answers_the_toast_and_clears_the_error_panel_in_both_languages() {
    for lang in [Lang::En, Lang::Pl] {
        let world = world();
        let headers = [cookie_of(lang), HTMX];
        let reply = crate::test_support::post_fields(
            &world.fixture.router(),
            "/machines/M7/connection",
            Some(&world.engineer),
            &headers,
            S7_BODY,
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{lang}: {}", reply.body);
        let sentence = text_with(lang, "connection-saved", "code", "M7");
        assert!(
            reply.body.contains(&format!("<span>{sentence}</span>")),
            "{lang}: {}",
            reply.body
        );
        assert!(
            reply
                .body
                .contains("<div id=\"toast\" hx-swap-oob=\"innerHTML\">"),
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
    }
}

#[tokio::test]
async fn connection_save_without_htmx_redirects_to_the_machine_page_and_the_record_exists() {
    let world = world();
    let before = world.audit_count();
    let reply = world
        .post(&world.admin, "/machines/M7/connection", S7_BODY, false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M7");
    assert_eq!(reply.header("hx-trigger"), "");
    assert_eq!(world.audit_count(), before.checked_add(1).unwrap());
    let page = world
        .get_en(&world.engineer, "/machines/M7/connection/edit")
        .await;
    assert_eq!(field_value(&page.body, "row_version"), "1");
}

#[tokio::test]
async fn connection_save_of_an_iec_machine_with_a_password_seals_it_once() {
    let world = world();
    seed(&world, 102, PlcPlatform::CodesysIec);
    let reply = world
        .post(&world.engineer, "/machines/M102/connection", OPC_BODY, true)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 1}");
    let calls = world.fixture.cipher.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(
        matches!(calls.first(), Some(CipherCall::Seal(_))),
        "{calls:?}"
    );
    assert!(!reply.body.contains("S3cret-PLC"), "{}", reply.body);
}

#[tokio::test]
async fn connection_save_treats_empty_texts_of_the_form_as_no_value() {
    let world = world();
    seed(&world, 101, PlcPlatform::TwinCat3);
    // The empty login and the empty password of the Ads mode are no values.
    let reply = world
        .post(&world.engineer, "/machines/M101/connection", ADS_BODY, true)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(world.fixture.cipher.calls().is_empty());
}

// --- Request shape -------------------------------------------------------------------------------

#[tokio::test]
async fn connection_save_refuses_an_unknown_field_with_e_web_008_and_writes_nothing() {
    let world = world();
    let before = world.fixture.snapshot();
    let body = format!("{S7_BODY}&extra=1");
    let reply = world
        .post(&world.engineer, "/machines/M7/connection", &body, true)
        .await;
    assert_error(
        &reply,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
        "unknown field",
    );
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn connection_save_refuses_a_row_version_that_is_not_a_number_and_writes_nothing() {
    let world = world();
    let before = world.fixture.snapshot();
    for bad in ["x", "", "-1", "1.5"] {
        let body = S7_BODY.replace("row_version=0", &format!("row_version={bad}"));
        let reply = world
            .post(&world.engineer, "/machines/M7/connection", &body, true)
            .await;
        assert_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008", bad);
    }
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn connection_save_of_an_unknown_machine_gives_404() {
    let world = world();
    let reply = world
        .post(&world.engineer, "/machines/M404/connection", S7_BODY, true)
        .await;
    assert_error(
        &reply,
        StatusCode::NOT_FOUND,
        "E-WEB-005",
        "unknown machine",
    );
}

// --- Success: the answer swaps the form out of band (ADR-0041 (h), R-H2) -----------------------------

/// The start of the opening tag of the connection form.
const FORM_START: &str = "<form id=\"connection-form\" ";
/// The attribute that swaps the form out of band (ADR-0041 (e)).
const OOB: &str = " hx-swap-oob=\"true\"";

/// Return the markup of the out-of-band form of an answer: from its tag to its end.
fn oob_form(body: &str) -> &str {
    let (_before, rest) = body.split_once(FORM_START).unwrap();
    let (tag, form) = rest.split_once('>').unwrap();
    assert!(tag.ends_with(OOB), "the form is not out of band: {tag}");
    form.split("</form>").next().unwrap()
}

/// Return the opening tag of the input with the `id` `id`.
fn input_tag<'a>(form: &'a str, id: &str) -> &'a str {
    let (_before, rest) = form.split_once(&format!("<input id=\"{id}\"")).unwrap();
    rest.split('>').next().unwrap()
}

#[tokio::test]
async fn a_valid_htmx_save_swaps_the_form_out_of_band_with_the_saved_state() {
    let world = world();
    seed(&world, 102, PlcPlatform::CodesysIec);
    let reply = world
        .post(&world.engineer, "/machines/M102/connection", OPC_BODY, true)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 1}");
    let form = oob_form(&reply.body);
    // The saved texts stand in the inputs and the hidden field holds the new token.
    for value in ["10.0.0.7", "4840", "opc.tcp://plc", "adapter"] {
        assert!(
            form.contains(&format!(" value=\"{value}\"")),
            "{value}: {form}"
        );
    }
    assert!(form.contains("name=\"row_version\" value=\"1\""), "{form}");
    let token = format!("name=\"csrf_token\" value=\"{}\"", world.engineer.csrf);
    assert!(form.contains(&token), "{form}");
    // The password input is empty. The hint tells that the system stores a password.
    let password = input_tag(form, "password");
    assert!(!password.contains(" value="), "{password}");
    assert!(
        form.contains(&text(Lang::En, "connection-password-stored")),
        "{form}"
    );
    assert!(!reply.body.contains("S3cret-PLC"), "{}", reply.body);
    // The form has no error line.
    assert!(!form.contains("field-error"), "{form}");
    assert!(!form.contains("aria-invalid"), "{form}");
    // The toast and the empty error panel stay in the answer.
    assert!(
        reply
            .body
            .contains("<div id=\"toast\" hx-swap-oob=\"innerHTML\">")
    );
    assert!(
        reply
            .body
            .contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>")
    );
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn a_valid_htmx_save_without_a_password_shows_the_no_password_state() {
    let world = world();
    seed(&world, 101, PlcPlatform::TwinCat3);
    let reply = world
        .post(&world.engineer, "/machines/M101/connection", ADS_BODY, true)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let form = oob_form(&reply.body);
    assert!(
        form.contains(&text(Lang::En, "connection-password-none")),
        "{form}"
    );
    assert!(!input_tag(form, "password").contains(" value="), "{form}");
}

#[tokio::test]
async fn a_valid_save_after_a_refused_save_answers_a_form_without_error_lines() {
    let world = world();
    let refused_body = "row_version=0&host=-bad-&rack=0&slot=2&db_number=1";
    let refused = world
        .post(
            &world.engineer,
            "/machines/M7/connection",
            refused_body,
            true,
        )
        .await;
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(oob_form(&refused.body).contains("aria-invalid=\"true\""));
    let saved = world
        .post(&world.engineer, "/machines/M7/connection", S7_BODY, true)
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    let form = oob_form(&saved.body);
    assert!(!form.contains("field-error"), "{form}");
    assert!(!form.contains("aria-invalid"), "{form}");
    assert!(form.contains("value=\"10.0.0.5\""), "{form}");
    assert!(!form.contains("-bad-"), "{form}");
}

// --- Refusal: the re-enter hint under the password input (design D8) -----------------------------------

/// The `id` of the hint that asks the user to type the password again.
const REENTER_ID: &str = "password-reenter";

/// Return `OPC_BODY` with a port that does not parse, so the save is refused.
fn refused_opc_body() -> String {
    OPC_BODY.replace("port=4840", "port=0")
}

/// Post `body` for the machine `M102` with the language `lang` and HTMX. Return the answer.
async fn refused_in(lang: Lang, body: &str) -> crate::test_support::Reply {
    let world = world();
    seed(&world, 102, PlcPlatform::CodesysIec);
    let headers = [cookie_of(lang), HTMX];
    crate::test_support::post_fields(
        &world.fixture.router(),
        "/machines/M102/connection",
        Some(&world.engineer),
        &headers,
        body,
    )
    .await
}

#[tokio::test]
async fn a_refused_save_with_a_password_shows_the_reenter_hint_under_the_password_input() {
    let reply = refused_in(Lang::En, &refused_opc_body()).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let form = oob_form(&reply.body);
    let password = input_tag(form, "password");
    assert!(!password.contains(" value="), "{password}");
    assert!(
        password.contains(&format!("aria-describedby=\"password-hint {REENTER_ID}\"")),
        "{password}"
    );
    assert!(!password.contains("aria-invalid"), "{password}");
    let hint = format!(
        "<span class=\"field-hint\" id=\"{REENTER_ID}\">{}</span>",
        text(Lang::En, "connection-password-reenter")
    );
    let (_before, after_input) = form.split_once("<input id=\"password\"").unwrap();
    assert!(after_input.contains(&hint), "{after_input}");
    assert!(!reply.body.contains("S3cret-PLC"), "{}", reply.body);
    // The hint is no field error: the password has no error line.
    assert!(!form.contains("id=\"password-error\""), "{form}");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn a_save_refused_by_the_domain_with_a_password_shows_the_reenter_hint() {
    // The mode Ads is no mode of a CODESYS platform, so the domain refuses after the parse.
    let body = OPC_BODY.replace("comm_mode=OpcUa", "comm_mode=Ads");
    let reply = refused_in(Lang::En, &body).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let form = oob_form(&reply.body);
    assert!(form.contains(&format!("id=\"{REENTER_ID}\"")), "{form}");
    assert!(!reply.body.contains("S3cret-PLC"), "{}", reply.body);
}

#[tokio::test]
async fn a_refused_save_without_a_password_shows_no_reenter_hint() {
    let empty = refused_opc_body().replace("password=S3cret-PLC", "password=");
    let absent = refused_opc_body().replace("&password=S3cret-PLC", "");
    for body in [empty, absent] {
        let reply = refused_in(Lang::En, &body).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.body
        );
        let form = oob_form(&reply.body);
        assert!(!form.contains(REENTER_ID), "{form}");
        let password = input_tag(form, "password");
        assert!(
            password.contains("aria-describedby=\"password-hint\""),
            "{password}"
        );
    }
}

#[tokio::test]
async fn the_reenter_hint_comes_from_the_bundle_in_both_languages() {
    for lang in [Lang::Pl, Lang::En] {
        let reply = refused_in(lang, &refused_opc_body()).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{lang}");
        let sentence = text(lang, "connection-password-reenter");
        assert!(
            !sentence.is_empty() && !sentence.starts_with("connection-"),
            "{lang}: {sentence}"
        );
        assert!(reply.body.contains(&sentence), "{lang}: {}", reply.body);
    }
    assert_ne!(
        text(Lang::Pl, "connection-password-reenter"),
        text(Lang::En, "connection-password-reenter")
    );
}

#[tokio::test]
async fn a_valid_save_after_a_refusal_with_a_password_shows_no_reenter_hint() {
    let world = world();
    seed(&world, 102, PlcPlatform::CodesysIec);
    let refused = world
        .post(
            &world.engineer,
            "/machines/M102/connection",
            &refused_opc_body(),
            true,
        )
        .await;
    assert_eq!(
        refused.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        refused.body
    );
    assert!(oob_form(&refused.body).contains(REENTER_ID));
    let saved = world
        .post(&world.engineer, "/machines/M102/connection", OPC_BODY, true)
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    assert!(
        !oob_form(&saved.body).contains(REENTER_ID),
        "{}",
        saved.body
    );
}

/// Return the transactions that the use cases of a request open to read without a write.
fn read_only_begins_since(world: &World, before: usize) -> usize {
    world
        .fixture
        .store
        .journal()
        .into_iter()
        .skip(before)
        .filter(|entry| matches!(entry, JournalEntry::Begin(Isolation::ReadOnly)))
        .count()
}

#[tokio::test]
async fn connection_save_builds_the_form_from_the_command_output_and_reads_nothing_after_the_commit()
 {
    let world = world();
    let before = world.fixture.store.journal().len();
    let reply = world
        .post(&world.engineer, "/machines/M7/connection", S7_BODY, true)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    // The only read is the unlocked read of `SetPlcConnection`. The form needs no second query.
    assert_eq!(read_only_begins_since(&world, before), 1);
    let form = oob_form(&reply.body);
    assert!(form.contains("name=\"row_version\" value=\"1\""), "{form}");
    for value in ["10.0.0.5", "0", "2", "1"] {
        assert!(form.contains(&format!(" value=\"{value}\"")), "{value}");
    }
}
