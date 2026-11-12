//! Tests of the field lines of `POST /machines/{code}/connection` (task 7.4 of
//! `plc-connection-config`, ADR-0041, ADR-0138, ADR-0154, design D2 and D8). A refused request
//! gives one `422` with a line under each bad input and the form with the sent texts. The leak
//! check (7.5) and the refusals (7.6) come in their own tasks.

use axum::http::StatusCode;
use recipe_domain::plant::PlcPlatform;

use super::connection_tests::seed;
use crate::i18n::Lang;
use crate::pages::test_helpers::{
    assert_clean_html, assert_error, assert_htmx_error, summary_lines,
};
use crate::test_support::plant::{World, world};
use crate::test_support::text;

/// The start of the opening tag of the connection form.
const FORM_START: &str = "<form id=\"connection-form\" ";
/// The attribute that swaps the form out of band (ADR-0041 (e)).
const OOB: &str = " hx-swap-oob=\"true\"";
/// A body for the S7 machine `M7` with a bad host and a rack of 8.
const S7_BAD_BODY: &str = "row_version=0&host=-bad-&rack=8&slot=2";
/// A secret that a test types into the password field.
const SECRET: &str = "S3cret-PLC";

/// Return the markup of the out-of-band form of an error answer: from its tag to its end.
fn oob_form(body: &str) -> &str {
    let (_before, rest) = body.split_once(FORM_START).unwrap();
    let (tag, form) = rest.split_once('>').unwrap();
    assert!(tag.ends_with(OOB), "the form is not out of band: {tag}");
    form.split("</form>").next().unwrap()
}

/// Assert that the form shows one line `E-...` under the input `id`.
fn assert_line_under(form: &str, id: &str, code: &str) {
    let start = format!("<div class=\"field-error\" id=\"{id}-error\">");
    assert!(form.contains(&start), "no line under {id}: {form}");
    let (_before, line) = form.split_once(&start).unwrap();
    let line = line.split("</div>").next().unwrap();
    assert!(line.contains(code), "{id}: {line}");
}

/// Post `body` to the connection route of `path` as the ProcessEngineer with HTMX.
async fn post(world: &World, machine: &str, body: &str) -> crate::test_support::Reply {
    let path = format!("/machines/{machine}/connection");
    world.post(&world.engineer, &path, body, true).await
}

// --- A text that does not parse: E-WEB-008 under its input ---------------------------------------

#[tokio::test]
async fn a_bad_host_gives_one_422_with_one_line_under_the_host() {
    let world = world();
    let before = world.fixture.snapshot();
    let body = "row_version=0&host=-bad-&rack=0&slot=2";
    let reply = post(&world, "M7", body).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(summary_lines(&reply.body), 1, "{}", reply.body);
    assert!(reply.body.contains("href=\"#host\""), "{}", reply.body);
    assert!(
        reply
            .body
            .contains(&text(Lang::En, "reason-connection-host")),
        "{}",
        reply.body
    );
    let form = oob_form(&reply.body);
    assert_line_under(form, "host", "E-WEB-008");
    assert_eq!(form.matches("class=\"field-error\"").count(), 1, "{form}");
    assert_eq!(form.matches("aria-invalid=\"true\"").count(), 1, "{form}");
    assert!(form.contains("value=\"-bad-\""), "{form}");
    assert_clean_html(&reply.body);
    assert_eq!(
        world.fixture.snapshot(),
        before,
        "a refused form writes nothing"
    );
}

#[tokio::test]
async fn a_rack_of_8_gives_one_422_with_one_line_under_the_rack() {
    let world = world();
    let body = "row_version=0&host=10.0.0.5&rack=8&slot=2";
    let reply = post(&world, "M7", body).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(summary_lines(&reply.body), 1, "{}", reply.body);
    assert!(
        reply
            .body
            .contains(&text(Lang::En, "reason-connection-rack")),
        "{}",
        reply.body
    );
    let form = oob_form(&reply.body);
    assert_line_under(form, "rack", "E-WEB-008");
    assert!(form.contains("value=\"8\""), "{form}");
}

#[tokio::test]
async fn a_bad_host_and_a_rack_of_8_give_one_422_with_both_lines_and_the_sent_texts() {
    let world = world();
    let reply = post(&world, "M7", S7_BAD_BODY).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert!(reply.body.contains("Errors: 2."), "{}", reply.body);
    assert_eq!(summary_lines(&reply.body), 2, "{}", reply.body);
    for id in ["host", "rack"] {
        assert!(reply.body.contains(&format!("href=\"#{id}\"")), "{id}");
    }
    let form = oob_form(&reply.body);
    assert_line_under(form, "host", "E-WEB-008");
    assert_line_under(form, "rack", "E-WEB-008");
    assert_eq!(form.matches("aria-invalid=\"true\"").count(), 2, "{form}");
    // The texts that parse stay in their inputs.
    assert!(form.contains("value=\"2\""), "{form}");
    assert!(form.contains("name=\"row_version\" value=\"0\""), "{form}");
    let token = format!("name=\"csrf_token\" value=\"{}\"", world.engineer.csrf);
    assert!(form.contains(&token), "{form}");
}

#[tokio::test]
async fn every_text_field_of_the_iec_shape_gets_its_own_line_and_reason() {
    let world = world();
    seed(&world, 102, PlcPlatform::CodesysIec);
    let body = "row_version=0&host=-bad-&port=0&address=%01&comm_mode=Telnet&login=%01";
    let reply = post(&world, "M102", body).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(summary_lines(&reply.body), 5, "{}", reply.body);
    let form = oob_form(&reply.body);
    for (id, key) in [
        ("host", "reason-connection-host"),
        ("port", "reason-connection-port"),
        ("address", "reason-connection-address"),
        ("comm_mode", "reason-connection-mode"),
        ("login", "reason-connection-login"),
    ] {
        assert_line_under(form, id, "E-WEB-008");
        assert!(reply.body.contains(&text(Lang::En, key)), "{key}");
    }
}

#[tokio::test]
async fn a_password_that_is_too_long_gives_a_line_under_the_password_and_no_echo() {
    let world = world();
    seed(&world, 102, PlcPlatform::CodesysIec);
    let long = "p".repeat(129);
    let body = format!(
        "row_version=0&host=10.0.0.7&port=4840&address=opc&comm_mode=OpcUa&login=adapter&password={long}"
    );
    let reply = post(&world, "M102", &body).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    let form = oob_form(&reply.body);
    assert_line_under(form, "password", "E-WEB-008");
    assert!(!reply.body.contains(&long), "{}", reply.body);
    let (_before, input) = form.split_once("<input id=\"password\"").unwrap();
    let input = input.split('>').next().unwrap();
    assert!(!input.contains(" value="), "{input}");
}

#[tokio::test]
async fn the_sent_password_never_comes_back_in_the_refused_form() {
    let world = world();
    seed(&world, 102, PlcPlatform::CodesysIec);
    let body = format!(
        "row_version=0&host=-bad-&port=4840&address=opc&comm_mode=OpcUa&login=adapter&password={SECRET}"
    );
    let reply = post(&world, "M102", &body).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert!(!reply.body.contains(SECRET), "{}", reply.body);
    let form = oob_form(&reply.body);
    assert!(form.contains("value=\"adapter\""), "{form}");
    assert!(form.contains("<option value=\"OpcUa\" selected>"), "{form}");
}

#[tokio::test]
async fn a_refused_request_without_htmx_gives_a_422_page_with_the_form_and_its_lines() {
    let world = world();
    let reply = world
        .post(
            &world.engineer,
            "/machines/M7/connection",
            S7_BAD_BODY,
            false,
        )
        .await;
    assert_error(
        &reply,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
        "no HTMX",
    );
    assert_line_under(&reply.body, "host", "E-WEB-008");
    assert_line_under(&reply.body, "rack", "E-WEB-008");
    assert!(reply.header("hx-retarget").is_empty());
}

#[tokio::test]
async fn the_lines_follow_the_language_of_the_request() {
    let world = world();
    let headers = [crate::test_support::plant::cookie_of(Lang::Pl)];
    let reply = crate::test_support::post_fields(
        &world.fixture.router(),
        "/machines/M7/connection",
        Some(&world.engineer),
        &headers,
        S7_BAD_BODY,
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    let reason = text(Lang::Pl, "reason-connection-rack");
    assert!(reply.body.contains(&reason), "{}", reply.body);
}

// --- A fault of the domain: PlantErrors under the fields -----------------------------------------

#[tokio::test]
async fn a_port_on_an_s7_1500_machine_gives_one_422_e_plt_022_in_the_summary() {
    let world = world();
    seed(&world, 105, PlcPlatform::S7_1500);
    let before = world.fixture.snapshot();
    let body = "row_version=0&host=10.0.0.5&rack=0&slot=1&port=4840";
    let reply = post(&world, "M105", body).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-022");
    assert_eq!(summary_lines(&reply.body), 1, "{}", reply.body);
    // The S7 form has no input for a port, so the line has no link and no line under an input.
    assert!(!reply.body.contains("href=\"#port\""), "{}", reply.body);
    let form = oob_form(&reply.body);
    assert!(!form.contains("class=\"field-error\""), "{form}");
    assert!(form.contains("name=\"row_version\" value=\"0\""), "{form}");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn every_fault_of_the_domain_is_a_line_of_the_same_answer() {
    let world = world();
    // An S7 machine with a port, an address, and a login: three faults in one answer.
    let body = "row_version=0&host=10.0.0.5&rack=0&slot=1&port=4840&address=a&login=b";
    let reply = post(&world, "M7", body).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-022");
    assert!(reply.body.contains("Errors: 3."), "{}", reply.body);
    assert_eq!(summary_lines(&reply.body), 3, "{}", reply.body);
}

#[tokio::test]
async fn a_missing_login_and_a_mode_off_the_platform_put_their_lines_under_the_inputs() {
    let world = world();
    seed(&world, 102, PlcPlatform::CodesysIec);
    let missing = "row_version=0&host=10.0.0.7&port=4840&address=opc&comm_mode=OpcUa&password=x";
    let reply = post(&world, "M102", missing).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-023");
    assert_line_under(oob_form(&reply.body), "login", "E-PLT-023");
    let off_platform = "row_version=0&host=10.0.0.7&port=48898&address=1.2.3.4.1.1&comm_mode=Ads";
    let reply = post(&world, "M102", off_platform).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-025");
    assert_line_under(oob_form(&reply.body), "comm_mode", "E-PLT-025");
}

#[tokio::test]
async fn an_ads_address_that_is_no_ams_net_id_puts_e_plt_026_under_the_address() {
    let world = world();
    seed(&world, 101, PlcPlatform::TwinCat3);
    let body = "row_version=0&host=10.0.0.6&port=48898&address=plc.local&comm_mode=Ads";
    let reply = post(&world, "M101", body).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-026");
    assert_line_under(oob_form(&reply.body), "address", "E-PLT-026");
}

#[tokio::test]
async fn a_login_in_the_mode_modbus_tcp_puts_e_plt_028_under_the_login() {
    let world = world();
    seed(&world, 102, PlcPlatform::CodesysIec);
    let body = "row_version=0&host=10.0.0.7&port=502&address=unit1&comm_mode=ModbusTcp&login=x";
    let reply = post(&world, "M102", body).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-PLT-028");
    assert_line_under(oob_form(&reply.body), "login", "E-PLT-028");
}

// --- A changed body: one E-WEB-008 line and no form (ADR-0138) -----------------------------------

#[tokio::test]
async fn an_unknown_or_repeated_key_gives_one_line_and_no_out_of_band_form() {
    let world = world();
    for body in [
        format!("{S7_BAD_BODY}&extra=1"),
        "row_version=0&host=10.0.0.5&host=10.0.0.6&rack=0&slot=2".to_owned(),
        "row_version=0&host=10.0.0.5&rack%5B%5D=0".to_owned(),
    ] {
        let reply = post(&world, "M7", &body).await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
        assert_eq!(summary_lines(&reply.body), 1, "{body}: {}", reply.body);
        assert!(!reply.body.contains("hx-swap-oob=\"true\""), "{body}");
        assert!(!reply.body.contains("class=\"field-error\""), "{body}");
    }
}

// --- The refusal of the form query wins --------------------------------------------------------

#[tokio::test]
async fn an_unknown_machine_with_a_bad_host_gives_404_and_no_form() {
    let world = world();
    let reply = post(&world, "M404", S7_BAD_BODY).await;
    assert_htmx_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005");
    assert!(!reply.body.contains("<form"), "{}", reply.body);
}
