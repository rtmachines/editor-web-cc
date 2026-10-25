//! Tests of the `INFO` line of "no webhook" (ADR-0063 (a), design D7).
//!
//! The loader writes one `INFO` line that names the key when the operator configures no webhook.
//! The line shows no path, no part of a URL, and no file content.

use figment::Figment;

use super::tests::{URL, base, load, set, text};
use crate::config::test_support::{LogCapture, Sandbox};

const HOOK: &str = "alerts.webhook_url_file";
const SECRET_NAME: &str = "hook_secret_name";

/// Load with a capture and return the log lines of the loader.
fn lines_of_load(figment: Figment) -> Vec<String> {
    let capture = LogCapture::start();

    load(figment).unwrap();

    capture.lines()
}

/// Assert that `lines` hold exactly one `INFO` line of "no webhook" and nothing secret.
fn assert_one_clean_info_line(lines: &[String]) {
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = lines.first().unwrap();
    assert!(line.contains("\"level\":\"INFO\""), "{line}");
    assert!(line.contains("alerts have no webhook"), "{line}");
    assert!(line.contains(&format!("\"key\":\"{HOOK}\"")), "{line}");
    for secret in [
        "recipe-server-config",
        SECRET_NAME,
        "/run/secrets",
        "alerts_webhook_url",
        "hooks",
        "tokenvalue",
    ] {
        assert!(!line.contains(secret), "{secret} in {line}");
    }
}

#[test]
fn serve_unset_webhook_key_without_default_file_gives_one_info_line() {
    // No test host has the default file, as in the test of the default key file path.
    let dir = Sandbox::new();

    let lines = lines_of_load(base(&dir));

    assert_one_clean_info_line(&lines);
}

#[test]
fn serve_empty_webhook_key_gives_one_info_line() {
    let dir = Sandbox::new();

    let lines = lines_of_load(set(base(&dir), HOOK, ""));

    assert_one_clean_info_line(&lines);
}

#[test]
fn serve_empty_webhook_file_gives_one_info_line() {
    let dir = Sandbox::new();
    for content in ["", "\n"] {
        let file = dir.write(SECRET_NAME, content);

        let lines = lines_of_load(set(base(&dir), HOOK, text(&file)));

        assert_one_clean_info_line(&lines);
    }
}

#[test]
fn serve_valid_webhook_gives_no_info_line_and_no_url() {
    let dir = Sandbox::new();
    let file = dir.write(SECRET_NAME, &format!("{URL}\n"));

    let lines = lines_of_load(set(base(&dir), HOOK, text(&file)));

    assert!(
        lines.iter().all(|line| !line.contains("no webhook")),
        "{lines:?}"
    );
    assert!(
        lines.iter().all(|line| !line.contains("tokenvalue")),
        "{lines:?}"
    );
}
