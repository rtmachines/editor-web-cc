//! Tests of step 2: the webhook client from the URL and the CA file.

use std::path::Path;

use super::build_webhook;
use crate::config::webhook::{WebhookUrl, parse_webhook_text};
use crate::serve::test_support::{LogCapture, Sandbox};

const KEY: &str = "alerts.webhook_ca_file";

/// The test authority of `webhook/tests/pki.rs`. The key behind it protects nothing.
const CERTIFICATE: &str = "-----BEGIN CERTIFICATE-----
MIIBqjCCAVGgAwIBAgIUW75TdZ3NGsgVBvWcifKDHo9cUy8wCgYIKoZIzj0EAwIw
IjEgMB4GA1UEAwwXUmVjaXBlIGVkaXRvciB0ZXN0IENBIEEwIBcNMjYxMDI1MDcz
MDEyWhgPMjEyNjEwMDEwNzMwMTJaMCIxIDAeBgNVBAMMF1JlY2lwZSBlZGl0b3Ig
dGVzdCBDQSBBMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE0qAeXKHGBg6SaBol
SREuFeSPnl1J7Z1ZpiznFxtYoCkJhmJN4ZGPyuC33o1ytAmn2MLtaik3NTLh2llz
7F8ilKNjMGEwHQYDVR0OBBYEFKryriKwrTfG/YpmGIaj+hkhd+X+MB8GA1UdIwQY
MBaAFKryriKwrTfG/YpmGIaj+hkhd+X+MA8GA1UdEwEB/wQFMAMBAf8wDgYDVR0P
AQH/BAQDAgEGMAoGCCqGSM49BAMCA0cAMEQCIBv4oCyAHZw++t4Qn5Rx2ZlqTQH7
hDYss8b+YjyKsVg7AiBZI2R3ZUv2aQerD8UDzn2d1Q+kd7WhSUZoJ2aQJTNJCg==
-----END CERTIFICATE-----
";

fn url() -> WebhookUrl {
    parse_webhook_text("https://hooks.example/alert")
        .unwrap()
        .unwrap()
}

/// The error text of a refused build, with every cause.
fn refusal(url: Option<&WebhookUrl>, ca_file: Option<&Path>) -> String {
    build_webhook(url, ca_file)
        .err()
        .map(|error| format!("{error:#}"))
        .expect("the build must refuse")
}

fn assert_names_the_key_only(message: &str, path: &Path) {
    assert!(message.contains(KEY), "{message}");
    let path_text = path.to_string_lossy();
    assert!(!message.contains(path_text.as_ref()), "{message}");
}

#[test]
fn no_url_and_no_ca_file_give_a_port_without_a_line() {
    let logs = LogCapture::start();
    assert!(build_webhook(None, None).is_ok());
    assert!(logs.lines().is_empty(), "{:?}", logs.lines());
}

#[test]
fn a_url_without_a_ca_file_gives_a_client() {
    assert!(build_webhook(Some(&url()), None).is_ok());
}

#[test]
fn a_url_with_a_ca_file_that_holds_a_certificate_gives_a_client() {
    let sandbox = Sandbox::new();
    let ca_file = sandbox.write("ca.pem", CERTIFICATE);
    assert!(build_webhook(Some(&url()), Some(&ca_file)).is_ok());
}

#[test]
fn a_ca_file_with_a_certificate_and_no_url_passes_with_one_warning() {
    let sandbox = Sandbox::new();
    let ca_file = sandbox.write("ca.pem", CERTIFICATE);
    let logs = LogCapture::start();
    assert!(build_webhook(None, Some(&ca_file)).is_ok());
    let lines = logs.lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("\"level\":\"WARN\""), "{lines:?}");
    assert!(lines[0].contains(KEY), "{lines:?}");
    let path_text = ca_file.to_string_lossy();
    assert!(!lines[0].contains(path_text.as_ref()), "{lines:?}");
}

#[test]
fn a_ca_file_without_a_certificate_and_no_url_refuses_and_names_the_key_only() {
    let sandbox = Sandbox::new();
    let ca_file = sandbox.write("ca.pem", "this is not a certificate\n");
    let logs = LogCapture::start();
    let message = refusal(None, Some(&ca_file));
    assert_names_the_key_only(&message, &ca_file);
    let lines = logs.lines();
    assert!(lines.is_empty(), "{lines:?}");
}

#[test]
fn a_missing_ca_file_refuses_and_names_the_key_only() {
    let sandbox = Sandbox::new();
    let ca_file = sandbox.missing("absent.pem");
    let message = refusal(None, Some(&ca_file));
    assert_names_the_key_only(&message, &ca_file);
}

#[test]
fn a_ca_file_without_a_certificate_and_a_url_refuses_and_names_the_key_only() {
    let sandbox = Sandbox::new();
    let ca_file = sandbox.write("ca.pem", "-----BEGIN NOTHING-----\n");
    let message = refusal(Some(&url()), Some(&ca_file));
    assert_names_the_key_only(&message, &ca_file);
}

#[test]
fn a_refusal_never_shows_a_part_of_the_url() {
    let sandbox = Sandbox::new();
    let ca_file = sandbox.write("ca.pem", "x\n");
    let message = refusal(Some(&url()), Some(&ca_file));
    assert!(!message.contains("hooks.example"), "{message}");
}
