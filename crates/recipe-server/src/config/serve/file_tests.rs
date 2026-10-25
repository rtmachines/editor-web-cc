//! Tests of the files that `serve` reads at the start: key files, webhook file, CA file.
//!
//! Every refusal must name the key and never the content, the path, or the URL (design D7).

use super::tests::{KEY_A, KEY_B, URL, base, canonical, load, refusal, set, text, value_of};
use crate::config::test_support::Sandbox;
use recipe_domain::setting::SettingValue;

const LOGIN: &str = "security.login_hmac_key_file";
const CSRF: &str = "security.csrf_signing_key_file";
const HOOK: &str = "alerts.webhook_url_file";
const CA: &str = "alerts.webhook_ca_file";

fn assert_clean(message: &str) {
    assert!(!message.contains("recipe-server-config"), "{message}");
    for part in ["hooks", "tokenvalue", "01234567", "fedcba98", "zzzz"] {
        assert!(!message.contains(part), "{part} in {message}");
    }
}

#[test]
fn serve_key_file_faults_name_the_key_and_not_the_content() {
    let dir = Sandbox::new();
    let zzzz = "z".repeat(64);
    let cases = [
        (LOGIN, format!("{KEY_A}\r\n"), "carriage return"),
        (CSRF, format!("{KEY_B}\r\n"), "carriage return"),
        (LOGIN, String::new(), "no key"),
        (CSRF, format!("{zzzz}\n"), "outside 0-9 and a-f"),
        (CSRF, format!("{KEY_B}\n\n{KEY_A}"), "is empty"),
        (LOGIN, KEY_A[..60].to_owned(), "64 characters"),
    ];
    for (key, content, words) in cases {
        let file = dir.write("faulty_key", &content);

        let message = refusal(set(base(&dir), key, text(&file)));

        assert!(message.contains(key), "{message}");
        assert!(message.contains(words), "{message}");
        assert_clean(&message);
    }
}

#[test]
fn serve_missing_or_unreadable_key_file_names_the_key() {
    let dir = Sandbox::new();
    let folder = dir.path("folder");
    std::fs::create_dir_all(&folder).unwrap();
    for key in [LOGIN, CSRF] {
        for path in [dir.path("absent_file"), folder.clone()] {
            let message = refusal(set(base(&dir), key, text(&path)));

            assert!(message.contains(key), "{message}");
            assert!(message.contains("cannot be read"), "{message}");
            assert_clean(&message);
        }
    }
}

#[test]
fn serve_default_key_file_path_is_read_and_a_missing_file_stops_the_start() {
    // No test host has the default files, so the plain defaults must give a refusal.
    let message = refusal(figment::Figment::new());

    assert!(message.contains(LOGIN), "{message}");
}

#[test]
fn serve_key_files_with_older_keys_are_accepted() {
    let dir = Sandbox::new();
    let both = dir.write("rotated", &format!("{KEY_A}\n{KEY_B}\n"));

    let config = load(set(base(&dir), CSRF, text(&both))).unwrap();

    assert!(
        config
            .csrf_keys()
            .verify(b"m", &config.csrf_keys().sign(b"m"))
    );
}

#[test]
fn serve_webhook_file_refusals_name_the_key_only() {
    let dir = Sandbox::new();
    let cases = [
        (format!("{URL}\r\n"), "carriage return"),
        (format!("{URL}\n{URL}\n"), "more than one line"),
        (URL.replace("https", "http"), "https"),
        ("tokenvalue0123456789".to_owned(), "not an absolute URL"),
    ];
    for (content, words) in cases {
        let file = dir.write("hook", &content);

        let message = refusal(set(base(&dir), HOOK, text(&file)));

        assert!(message.contains(HOOK), "{message}");
        assert!(message.contains(words), "{message}");
        assert_clean(&message);
    }
}

#[test]
fn serve_webhook_missing_file_stops_a_set_path_and_not_the_default() {
    let dir = Sandbox::new();

    let message = refusal(set(base(&dir), HOOK, text(&dir.path("absent_file"))));
    let config = load(base(&dir)).unwrap();

    assert!(message.contains(HOOK), "{message}");
    assert!(message.contains("cannot be read"), "{message}");
    assert_clean(&message);
    assert!(config.webhook_url().is_none());
}

#[test]
fn serve_webhook_empty_key_or_empty_file_means_no_webhook() {
    let dir = Sandbox::new();
    let empty = dir.write("empty", "");

    let from_key = load(set(base(&dir), HOOK, "")).unwrap();
    let from_file = load(set(base(&dir), HOOK, text(&empty))).unwrap();

    for config in [&from_key, &from_file] {
        assert!(config.webhook_url().is_none());
        assert_eq!(value_of(&canonical(config), "alerts.webhook_url"), "");
    }
    assert_eq!(value_of(&canonical(&from_key), HOOK), "");
    assert_eq!(value_of(&canonical(&from_file), HOOK), text(&empty));
}

#[test]
fn serve_webhook_url_is_recorded_as_a_fingerprint_and_never_printed() {
    let dir = Sandbox::new();
    let file = dir.write("hook", &format!("{URL}\n"));

    let config = load(set(base(&dir), HOOK, text(&file))).unwrap();

    let pairs = canonical(&config);
    let fingerprint = SettingValue::fingerprint(URL);
    assert_eq!(value_of(&pairs, "alerts.webhook_url"), fingerprint.as_str());
    assert_eq!(
        config.webhook_url().unwrap().uri().host(),
        Some("hooks.plant.example")
    );
    let shown = format!("{config:?} {:?}", config.configured());
    for part in ["hooks", "tokenvalue", "8443", KEY_A, KEY_B] {
        assert!(!shown.contains(part), "{part} in {shown}");
    }
}

#[test]
fn serve_ca_file_must_exist_when_set() {
    let dir = Sandbox::new();
    let ca = dir.write("ca", "certificate text");

    let message = refusal(set(base(&dir), CA, text(&dir.path("absent_file"))));
    let set_file = load(set(base(&dir), CA, text(&ca))).unwrap();
    let empty = load(set(base(&dir), CA, "")).unwrap();

    assert!(message.contains(CA), "{message}");
    assert!(message.contains("cannot be read"), "{message}");
    assert_clean(&message);
    assert_eq!(value_of(&canonical(&set_file), CA), text(&ca));
    assert_eq!(value_of(&canonical(&empty), CA), "");
}

#[test]
fn serve_app_connection_reads_the_password_file_and_names_the_key_on_a_fault() {
    let dir = Sandbox::new();
    let good = dir.write("app_secret", "app-secret-text\n");
    let bad = dir.write("app_bad", "topsecretvalue\nsecondline\n");

    let config = load(set(base(&dir), "secrets.app_password_file", text(&good))).unwrap();
    let faulty = load(set(base(&dir), "secrets.app_password_file", text(&bad))).unwrap();

    let connection = config.app_connection().unwrap();
    assert_eq!(connection.user, "recipe_app");
    assert_eq!(connection.host, "db");
    assert!(!format!("{connection:?}").contains("app-secret-text"));
    let error = format!("{:#}", faulty.app_connection().unwrap_err());
    assert!(error.contains("secrets.app_password_file"), "{error}");
    assert!(!error.contains("topsecretvalue"), "{error}");
}
