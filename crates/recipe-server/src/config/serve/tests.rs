//! Tests of the 18 operator keys of `serve` (design D7, D14 row T17).
//!
//! The environment of the process is global, so no test sets an environment value. A test
//! passes a figment provider in its place, as the tests of `migrate` do.

use std::path::Path;

use anyhow::Result;
use figment::Figment;
use figment::providers::Serialized;
use recipe_domain::setting::SettingValue;
use serde::Serialize;

use super::{DEV_BUILD, RawServe, SERVE_ENV_KEYS, ServeConfig, check_secure, load_with};
use crate::config::test_support::Sandbox;
use crate::serve::Plan;

pub(super) const KEY_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub(super) const KEY_B: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
pub(super) const URL: &str = "https://hooks.plant.example:8443/alert/tokenvalue0123456789";
const RELIABILITY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/architecture/reliability.md"
);

/// The keys of design D7 in table order. `alerts.webhook_url` is the derived key.
const KEYS: [&str; 18] = [
    "db.host",
    "db.port",
    "db.name",
    "db.app_user",
    "secrets.app_password_file",
    "security.login_hmac_key_file",
    "security.csrf_signing_key_file",
    "http.trusted_proxies",
    "http.public_port",
    "http.internal_port",
    "approval.separation_of_duties",
    "alerts.webhook_url_file",
    "alerts.webhook_ca_file",
    "alerts.webhook_url",
    "session.secure",
    "session.idle_timeout",
    "session.absolute_timeout",
    "i18n.translation_langs",
];

/// The environment names of design D7. The derived key has none.
const ENV_NAMES: [&str; 17] = [
    "RECIPE_DB__HOST",
    "RECIPE_DB__PORT",
    "RECIPE_DB__NAME",
    "RECIPE_DB__APP_USER",
    "RECIPE_SECRETS__APP_PASSWORD_FILE",
    "RECIPE_SECURITY__LOGIN_HMAC_KEY_FILE",
    "RECIPE_SECURITY__CSRF_SIGNING_KEY_FILE",
    "RECIPE_HTTP__TRUSTED_PROXIES",
    "RECIPE_HTTP__PUBLIC_PORT",
    "RECIPE_HTTP__INTERNAL_PORT",
    "RECIPE_APPROVAL__SEPARATION_OF_DUTIES",
    "RECIPE_ALERTS__WEBHOOK_URL_FILE",
    "RECIPE_ALERTS__WEBHOOK_CA_FILE",
    "RECIPE_SESSION__SECURE",
    "RECIPE_SESSION__IDLE_TIMEOUT",
    "RECIPE_SESSION__ABSOLUTE_TIMEOUT",
    "RECIPE_I18N__TRANSLATION_LANGS",
];

pub(super) fn set(figment: Figment, key: &str, value: impl Serialize) -> Figment {
    figment.merge(Serialized::default(key, value))
}

pub(super) fn text(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

/// A provider with the two key files, which the defaults cannot supply on a test host.
pub(super) fn base(dir: &Sandbox) -> Figment {
    let login = dir.write("login_key", &format!("{KEY_A}\n"));
    let csrf = dir.write("csrf_key", &format!("{KEY_B}\n"));
    let figment = set(Figment::new(), "security.login_hmac_key_file", text(&login));
    set(figment, "security.csrf_signing_key_file", text(&csrf))
}

pub(super) fn load(figment: Figment) -> Result<ServeConfig> {
    load_with(figment, false)
}

pub(super) fn refusal(figment: Figment) -> String {
    format!("{:#}", load(figment).unwrap_err())
}

/// The 18 pairs of key and canonical text of a configuration.
pub(super) fn canonical(config: &ServeConfig) -> Vec<(String, String)> {
    let pairs = config.configured().canonical().unwrap();
    pairs
        .into_iter()
        .map(|(key, value)| (key.as_str().to_owned(), value.as_str().to_owned()))
        .collect()
}

pub(super) fn value_of(pairs: &[(String, String)], key: &str) -> String {
    let (_key, value) = pairs.iter().find(|(name, _)| name == key).unwrap();
    value.clone()
}

/// Assert that the row of `reliability.md` 7.11 that starts with `prefix` holds `needle`.
fn assert_row(prefix: &str, needle: &str) {
    let document = std::fs::read_to_string(RELIABILITY).unwrap();
    let start = format!("| `{prefix}` |");
    let row = document
        .lines()
        .find(|line| line.starts_with(&start))
        .unwrap();
    assert!(row.contains(needle), "{needle} not in {row}");
}

#[test]
fn serve_keys_are_the_18_keys_of_design_d7_in_table_order() {
    let dir = Sandbox::new();
    let config = load(base(&dir)).unwrap();

    let pairs = canonical(&config);

    let names: Vec<&str> = pairs.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(names, KEYS);
}

#[test]
fn serve_environment_names_are_those_of_design_d7() {
    let expected: Vec<String> = ENV_NAMES
        .iter()
        .map(|name| name.strip_prefix("RECIPE_").unwrap().to_lowercase())
        .collect();

    assert_eq!(SERVE_ENV_KEYS.to_vec(), expected);
    for (name, key) in ENV_NAMES
        .iter()
        .zip(KEYS.iter().filter(|k| **k != "alerts.webhook_url"))
    {
        assert_eq!(
            name.strip_prefix("RECIPE_").unwrap().to_lowercase(),
            key.replace('.', "__")
        );
    }
}

#[test]
fn serve_every_environment_key_is_a_key_of_the_settings() {
    for key in SERVE_ENV_KEYS {
        let figment = Figment::new().merge(Serialized::default(&key.replace("__", "."), "x"));

        let message = format!("{:#}", load(figment).unwrap_err());

        assert!(!message.contains("unknown field"), "{key}: {message}");
    }
}

#[test]
fn serve_refuses_an_unknown_key_and_the_derived_key() {
    let dir = Sandbox::new();
    for key in [
        "http.nope",
        "alerts.webhook_url",
        "db.migrator_user",
        "other.key",
    ] {
        let message = refusal(set(base(&dir), key, "x"));

        assert!(message.contains("unknown field"), "{key}: {message}");
    }
}

#[test]
fn serve_defaults_equal_the_table_of_reliability_7_11() {
    let dir = Sandbox::new();
    let config = load(base(&dir)).unwrap();

    let pairs = canonical(&config);

    let expected = [
        ("db.host", "db"),
        ("db.port", "5432"),
        ("db.name", "recipe"),
        ("db.app_user", "recipe_app"),
        ("secrets.app_password_file", "/run/secrets/app_password"),
        ("http.trusted_proxies", "172.28.0.2/32"),
        ("http.public_port", "8080"),
        ("http.internal_port", "9000"),
        ("approval.separation_of_duties", "true"),
        ("alerts.webhook_url_file", "/run/secrets/alerts_webhook_url"),
        ("alerts.webhook_ca_file", ""),
        ("alerts.webhook_url", ""),
        ("session.secure", "true"),
        ("session.idle_timeout", "1800s"),
        ("session.absolute_timeout", "43200s"),
        ("i18n.translation_langs", "pl"),
    ];
    for (key, text) in expected {
        assert_eq!(value_of(&pairs, key), text, "{key}");
    }
}

#[test]
fn serve_default_texts_are_in_the_rows_of_reliability_7_11() {
    assert_row("db.host` / `db.port` / `db.name", "`db` / 5432 / `recipe`");
    assert_row("db.app_user", "`recipe_app`");
    assert_row("secrets.app_password_file", "`/run/secrets/app_password`");
    assert_row(
        "security.login_hmac_key_file",
        "`/run/secrets/login_hmac_key`",
    );
    assert_row(
        "security.csrf_signing_key_file",
        "`/run/secrets/csrf_signing_key`",
    );
    assert_row("http.trusted_proxies", "`172.28.0.2/32`");
    assert_row("http.public_port` / `http.internal_port", "8080 / 9000");
    assert_row("approval.separation_of_duties", "`true`");
    assert_row(
        "alerts.webhook_url_file",
        "`/run/secrets/alerts_webhook_url`",
    );
    assert_row("session.secure", "`true`");
    assert_row("session.idle_timeout", "30 min");
    assert_row("session.absolute_timeout", "12 h");
    assert_row("i18n.translation_langs", "`pl`");
    let raw = RawServe::default();
    assert_eq!(
        raw.security.login_hmac_key_file,
        "/run/secrets/login_hmac_key"
    );
    assert_eq!(
        raw.security.csrf_signing_key_file,
        "/run/secrets/csrf_signing_key"
    );
}

/// Set the 17 operator keys to values that differ from every default.
///
/// The two key files already come from `base`. This function keeps their sandbox paths.
fn set_every_key(figment: Figment, hook: &Path, ca: &Path) -> Figment {
    let mut figment = figment;
    for (key, value) in [
        ("db.host", "database".to_owned()),
        ("db.name", "plant".to_owned()),
        ("db.app_user", "app_x".to_owned()),
        ("secrets.app_password_file", "/s/app".to_owned()),
        (
            "http.trusted_proxies",
            "10.0.0.0/8,172.28.0.2/32".to_owned(),
        ),
        ("alerts.webhook_url_file", text(hook)),
        ("alerts.webhook_ca_file", text(ca)),
        ("i18n.translation_langs", "pl,de".to_owned()),
    ] {
        figment = set(figment, key, value);
    }
    figment = set(figment, "db.port", 6543_u16);
    figment = set(figment, "http.public_port", 8081_u16);
    figment = set(figment, "http.internal_port", 9001_u16);
    figment = set(figment, "approval.separation_of_duties", false);
    figment = set(figment, "session.secure", false);
    figment = set(figment, "session.idle_timeout", 600_u64);
    set(figment, "session.absolute_timeout", 7200_u64)
}

/// The 18 canonical pairs that `set_every_key` must give, in the table order of design D7.
fn expected_pairs(dir: &Sandbox, hook: &Path, ca: &Path) -> Vec<(String, String)> {
    let fingerprint = SettingValue::fingerprint(URL);
    [
        ("db.host", "database".to_owned()),
        ("db.port", "6543".to_owned()),
        ("db.name", "plant".to_owned()),
        ("db.app_user", "app_x".to_owned()),
        ("secrets.app_password_file", "/s/app".to_owned()),
        ("security.login_hmac_key_file", text(&dir.path("login_key"))),
        (
            "security.csrf_signing_key_file",
            text(&dir.path("csrf_key")),
        ),
        (
            "http.trusted_proxies",
            "10.0.0.0/8,172.28.0.2/32".to_owned(),
        ),
        ("http.public_port", "8081".to_owned()),
        ("http.internal_port", "9001".to_owned()),
        ("approval.separation_of_duties", "false".to_owned()),
        ("alerts.webhook_url_file", text(hook)),
        ("alerts.webhook_ca_file", text(ca)),
        ("alerts.webhook_url", fingerprint.as_str().to_owned()),
        ("session.secure", "false".to_owned()),
        ("session.idle_timeout", "600s".to_owned()),
        ("session.absolute_timeout", "7200s".to_owned()),
        ("i18n.translation_langs", "pl,de".to_owned()),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value))
    .collect()
}

#[test]
fn serve_canonical_text_of_every_set_value() {
    let dir = Sandbox::new();
    let hook = dir.write("hook", &format!("{URL}\n"));
    let ca = dir.write("ca", "certificate text");
    let defaults = canonical(&load(base(&dir)).unwrap());
    let figment = set_every_key(base(&dir), &hook, &ca);

    let pairs = canonical(&load_with(figment, true).unwrap());

    let expected = expected_pairs(&dir, &hook, &ca);
    assert_eq!(expected.len(), 18);
    assert_eq!(pairs, expected);
    // `base` already moves the two key files into the sandbox, so `defaults` cannot show them.
    for (key, value) in &expected {
        if key.starts_with("security.") {
            assert!(!value.starts_with("/run/secrets/"), "{key}: {value}");
        } else {
            assert_ne!(&value_of(&defaults, key), value, "{key} equals its default");
        }
    }
}

#[test]
fn serve_secure_false_stops_the_start_without_the_dev_build() {
    assert!(check_secure(true, false).is_ok());
    assert!(check_secure(true, true).is_ok());
    assert!(check_secure(false, true).is_ok());
    let error = check_secure(false, false).unwrap_err();
    assert!(format!("{error:#}").contains("session.secure"), "{error:#}");

    let dir = Sandbox::new();
    let message = refusal(set(base(&dir), "session.secure", false));
    assert!(message.contains("session.secure"), "{message}");
    let config = load_with(set(base(&dir), "session.secure", false), true).unwrap();
    assert_eq!(value_of(&canonical(&config), "session.secure"), "false");
}

#[cfg(feature = "dev")]
#[test]
fn serve_secure_false_loads_in_a_dev_build() {
    assert!(check_secure(false, DEV_BUILD).is_ok());
}

#[cfg(not(feature = "dev"))]
#[test]
fn serve_secure_false_is_refused_in_a_build_without_the_dev_feature() {
    let error = check_secure(false, DEV_BUILD).unwrap_err();
    assert!(format!("{error:#}").contains("session.secure"), "{error:#}");
}

#[test]
fn serve_value_over_1000_characters_names_the_key() {
    let dir = Sandbox::new();
    let long = "x".repeat(1001);
    let proxies = vec!["10.0.0.0/8"; 100].join(",");
    let langs: Vec<String> = ('a'..='z')
        .flat_map(|first| ('a'..='z').map(move |second| format!("{first}{second}")))
        .filter(|tag| tag != "en")
        .take(400)
        .collect();
    let mut cases: Vec<(&str, String)> = [
        "db.host",
        "db.name",
        "db.app_user",
        "secrets.app_password_file",
        "security.login_hmac_key_file",
        "security.csrf_signing_key_file",
        "alerts.webhook_url_file",
        "alerts.webhook_ca_file",
    ]
    .into_iter()
    .map(|key| (key, long.clone()))
    .collect();
    cases.push(("http.trusted_proxies", proxies));
    cases.push(("i18n.translation_langs", langs.join(",")));
    for (key, value) in cases {
        let message = refusal(set(base(&dir), key, value));

        assert!(message.contains(key), "{key}: {message}");
        assert!(message.contains("1000"), "{key}: {message}");
        assert!(!message.contains("xxxxxxxxxx"), "{key}: {message}");
    }
}

#[test]
fn serve_value_of_exactly_1000_characters_passes() {
    let dir = Sandbox::new();

    let config = load(set(base(&dir), "db.host", "x".repeat(1000))).unwrap();

    assert_eq!(value_of(&canonical(&config), "db.host").len(), 1000);
}

#[test]
fn serve_lists_and_timeouts_with_a_bad_item_name_the_key_only() {
    let dir = Sandbox::new();
    let cases = [
        ("http.trusted_proxies", "10.0.0.0/8,not-a-network"),
        ("http.trusted_proxies", "10.0.0.7"),
        ("http.trusted_proxies", ""),
        ("i18n.translation_langs", "en"),
        ("i18n.translation_langs", "pl,pl"),
        ("i18n.translation_langs", "PL"),
        ("i18n.translation_langs", "pl,,de"),
        ("i18n.translation_langs", ""),
    ];
    for (key, value) in cases {
        let message = refusal(set(base(&dir), key, value));

        assert!(message.contains(key), "{key}={value}: {message}");
        for part in ["not-a-network", "10.0.0.7", "PL"] {
            assert!(!message.contains(part), "{key}={value}: {message}");
        }
    }
}

#[test]
fn serve_zero_and_bad_numbers_name_the_key() {
    let dir = Sandbox::new();
    for key in ["session.idle_timeout", "session.absolute_timeout"] {
        let message = refusal(set(base(&dir), key, 0_u64));
        assert!(message.contains(key), "{key}: {message}");
    }
    for (key, value) in [
        ("http.public_port", 70_000_u32),
        ("http.internal_port", 70_000),
    ] {
        let message = refusal(set(base(&dir), key, value));
        assert!(message.contains(key), "{key}: {message}");
    }
    let message = refusal(set(base(&dir), "session.idle_timeout", "abc"));
    assert!(message.contains("session.idle_timeout"), "{message}");
}

const PORT_KEYS: [&str; 2] = ["http.public_port", "http.internal_port"];

#[test]
fn serve_ports_accept_the_bounds_as_number_and_as_text() {
    let dir = Sandbox::new();
    for key in PORT_KEYS {
        for (number, expected) in [(1_u32, "1"), (65_535, "65535")] {
            let config = load(set(base(&dir), key, number)).unwrap();
            assert_eq!(value_of(&canonical(&config), key), expected, "{key}");
            let config = load(set(base(&dir), key, expected)).unwrap();
            assert_eq!(value_of(&canonical(&config), key), expected, "{key}");
        }
    }
}

#[test]
fn serve_ports_refuse_zero_spaces_signs_empty_and_70000_and_name_the_key_only() {
    let dir = Sandbox::new();
    for key in PORT_KEYS {
        for value in ["0", " 9000", "9000 ", "+9000", "", "70000", "abc"] {
            let message = refusal(set(base(&dir), key, value));

            assert!(message.contains(key), "{key}={value:?}: {message}");
            let digits = value.trim_matches(|c: char| !c.is_ascii_digit());
            if digits.len() > 1 {
                assert!(!message.contains(digits), "{key}={value:?}: {message}");
            }
        }
        for value in [0_u32, 70_000] {
            let message = refusal(set(base(&dir), key, value));

            assert!(message.contains(key), "{key}={value}: {message}");
        }
    }
}

#[test]
fn serve_accessors_hold_the_checked_values() {
    let dir = Sandbox::new();
    let config = load(set(base(&dir), "i18n.translation_langs", "pl,de")).unwrap();

    let langs: Vec<&str> = config
        .translation_langs()
        .iter()
        .map(|l| l.as_str())
        .collect();
    assert_eq!(langs, ["pl", "de"]);
    assert!(
        config
            .trusted_proxies()
            .contains("172.28.0.2".parse().unwrap())
    );
    assert!(
        !config
            .trusted_proxies()
            .contains("172.28.0.3".parse().unwrap())
    );
    assert!(config.webhook_url().is_none());
    let tag = config.login_keys().sign(b"m");
    assert!(config.login_keys().verify(b"m", &tag));
    assert!(!config.csrf_keys().verify(b"m", &tag));
}

#[test]
fn serve_plan_from_config_keeps_each_key_ring_and_the_values_of_the_config() {
    let dir = Sandbox::new();
    let hook = dir.write("hook", &format!("{URL}\n"));
    let figment = set(base(&dir), "alerts.webhook_url_file", text(&hook));
    let figment = set(figment, "http.trusted_proxies", "10.0.0.0/8");
    let figment = set(figment, "http.public_port", 8081_u16);
    let figment = set(figment, "http.internal_port", 9001_u16);
    let figment = set(figment, "i18n.translation_langs", "pl,de");
    let figment = set(figment, "session.idle_timeout", 600_u64);
    let figment = set(figment, "approval.separation_of_duties", false);
    let config = load(figment).unwrap();

    let plan = Plan::from_config(&config);

    let login_tag = config.login_keys().sign(b"m");
    let csrf_tag = config.csrf_keys().sign(b"m");
    assert_ne!(login_tag, csrf_tag, "the two key files must differ");
    assert!(plan.login_keys.verify(b"m", &login_tag));
    assert!(!plan.login_keys.verify(b"m", &csrf_tag));
    assert!(plan.csrf_keys.verify(b"m", &csrf_tag));
    assert!(!plan.csrf_keys.verify(b"m", &login_tag));
    let planned = plan.configured.canonical().unwrap();
    assert_eq!(planned, config.configured().canonical().unwrap());
    assert_eq!(plan.configured.public_port, 8081);
    assert_eq!(plan.configured.internal_port, 9001);
    assert!(plan.webhook_url.is_some());
    let proxies = &plan.settings.web.trusted_proxies;
    assert!(proxies.contains("10.1.2.3".parse().unwrap()));
    assert!(!proxies.contains("172.28.0.2".parse().unwrap()));
    assert_eq!(
        plan.settings.web.translation_langs,
        config.translation_langs()
    );
    assert_eq!(plan.settings.identity.session_idle_timeout.as_secs(), 600);
    assert!(!plan.settings.app.approval_policy.separation_of_duties);
}
