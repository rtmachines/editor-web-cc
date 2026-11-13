//! Tests of the request struct of `POST /machines/{code}/connection` (task 7.3 of
//! `plc-connection-config`, ADR-0154, design D2, D8, and D12). The test of the field lines (7.4),
//! the leak check (7.5), and the refusals (7.6) come in their own tasks.

use axum::body::Body;
use axum::extract::{Form, FromRequest};
use axum::http::{Request, StatusCode, header};
use recipe_domain::plant::PlcPlatform;

use super::connection_request::{ConnectionRequest, SecretField};
use super::connection_tests::seed;
use crate::pages::test_helpers::assert_error;
use crate::test_support::plant::world;

/// The secret that the tests type into the password field.
const SECRET: &str = "S3cret-PLC";
/// A body in which every text field is empty.
const EMPTY_BODY: &str =
    "row_version=0&host=&rack=&slot=&port=&address=&comm_mode=&login=&password=";

/// Read the form body of a request into the struct, as the router does.
async fn read(body: &str) -> ConnectionRequest {
    let request = Request::builder()
        .method("POST")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body.to_owned()))
        .unwrap();
    let Form(fields) = Form::<ConnectionRequest>::from_request(request, &())
        .await
        .unwrap();
    fields
}

// --- An empty text is no value ---------------------------------------------------------------------

#[tokio::test]
async fn an_empty_text_of_every_field_gives_no_value() {
    let request = read(EMPTY_BODY).await;
    assert!(request.password.is_none());
    let fields = request.fields().unwrap();
    assert!(fields.host.is_none());
    assert!(fields.rack.is_none());
    assert!(fields.slot.is_none());
    assert!(fields.port.is_none());
    assert!(fields.address.is_none());
    assert!(fields.mode.is_none());
    assert!(fields.login.is_none());
    assert!(fields.password.is_none());
}

#[tokio::test]
async fn an_absent_field_gives_no_value() {
    let request = read("row_version=3").await;
    assert!(request.password.is_none());
    let fields = request.fields().unwrap();
    assert!(fields.host.is_none() && fields.mode.is_none() && fields.password.is_none());
}

#[tokio::test]
async fn a_text_of_a_field_gives_its_value() {
    let request = read(&format!(
        "row_version=1&host=10.0.0.5&port=4840&comm_mode=OpcUa&login=adapter&password={SECRET}"
    ))
    .await;
    let fields = request.fields().unwrap();
    assert_eq!(fields.host.unwrap().to_string(), "10.0.0.5");
    assert_eq!(fields.port.unwrap().value(), 4840);
    assert_eq!(fields.login.unwrap().to_string(), "adapter");
    assert_eq!(fields.password.unwrap().expose(), SECRET);
}

#[tokio::test]
async fn a_password_of_spaces_is_a_value_and_is_not_trimmed() {
    let request = read("row_version=1&password=+++").await;
    let fields = request.fields().unwrap();
    assert_eq!(fields.password.unwrap().expose(), "   ");
}

// --- The password field --------------------------------------------------------------------------

#[tokio::test]
async fn the_password_field_is_an_optional_secret_field() {
    let request = read(&format!("row_version=1&password={SECRET}")).await;
    let secret: &SecretField = request.password.as_ref().unwrap();
    assert_eq!(secret.expose(), SECRET);
}

#[test]
fn the_debug_of_a_secret_field_hides_its_text() {
    let secret = SecretField::from(SECRET.to_owned());
    let shown = format!("{secret:?} {:?}", Some(&secret));
    assert!(!shown.contains(SECRET), "{shown}");
    assert!(shown.contains("redacted"), "{shown}");
}

#[tokio::test]
async fn the_debug_of_the_request_hides_the_password_and_every_text() {
    let request = read(&format!("row_version=1&host=10.0.0.5&password={SECRET}")).await;
    let shown = format!("{request:?}");
    assert!(!shown.contains(SECRET), "{shown}");
    assert!(!shown.contains("10.0.0.5"), "{shown}");
    let fields = format!("{:?}", request.fields().unwrap());
    assert!(!fields.contains(SECRET), "{fields}");
}

// --- The request on the route --------------------------------------------------------------------

#[tokio::test]
async fn an_empty_host_gives_e_plt_023_and_writes_nothing() {
    let world = world();
    let before = world.fixture.snapshot();
    let body = "row_version=0&host=&rack=0&slot=2";
    let reply = world
        .post(&world.engineer, "/machines/M7/connection", body, true)
        .await;
    assert_error(
        &reply,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-023",
        "empty host",
    );
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn an_empty_port_on_an_s7_1500_machine_has_no_effect() {
    let world = world();
    seed(&world, 105, PlcPlatform::S7_1500);
    let body =
        "row_version=0&host=10.0.0.5&rack=0&slot=1&port=&address=&comm_mode=&login=&password=";
    let reply = world
        .post(&world.engineer, "/machines/M105/connection", body, true)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 1}");
}

#[tokio::test]
async fn a_port_with_a_value_on_an_s7_1500_machine_gives_e_plt_022() {
    let world = world();
    seed(&world, 105, PlcPlatform::S7_1500);
    let body = "row_version=0&host=10.0.0.5&rack=0&slot=1&port=4840";
    let reply = world
        .post(&world.engineer, "/machines/M105/connection", body, true)
        .await;
    assert_error(
        &reply,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-022",
        "port on S7",
    );
}
