//! Tests of the texts of the PLC connection pages and errors (change `plc-connection-config`,
//! task 7.8).
//!
//! Every key of this change must exist in both bundles. The ten error codes must keep the
//! English text of their row in `reliability.md` section 1.2 and the same placeables in Polish.

use super::tests::{fluent_text, ftl_message, registry_message};
use super::*;

/// The ten codes of this change that have a message in the bundles.
const CONNECTION_CODES: [&str; 10] = [
    "E-PLT-022",
    "E-PLT-023",
    "E-PLT-024",
    "E-PLT-025",
    "E-PLT-026",
    "E-PLT-027",
    "E-PLT-028",
    "E-PLT-029",
    "E-WEB-013",
    "E-WEB-014",
];

/// The keys of the labels, hints, reasons, and state texts of this change.
const CONNECTION_KEYS: &[&str] = &[
    "audit-entity-plc-connection",
    "audit-action-plc-connection-set",
    "audit-action-plc-connection-exported",
    "operation-set-plc-connection",
    "operation-get-plc-connection-form",
    "operation-export-plc-connection",
    "machine-connection-title",
    "machine-connection-none",
    "machine-connection-password-stored",
    "machine-connection-password-none",
    "connection-title",
    "connection-machine-link",
    "connection-submit",
    "connection-saved",
    "connection-host-hint",
    "connection-rack-hint",
    "connection-slot-hint",
    "connection-db-number-hint",
    "connection-port-hint",
    "connection-address-hint",
    "connection-mode-hint",
    "connection-login-hint",
    "connection-mode-choose",
    "connection-mode-ads",
    "connection-mode-opcua",
    "connection-mode-codesysv3",
    "connection-mode-modbustcp",
    "connection-password-stored",
    "connection-password-none",
    "field-plc-host",
    "field-plc-rack",
    "field-plc-slot",
    "field-plc-db-number",
    "field-plc-port",
    "field-plc-address",
    "field-plc-comm-mode",
    "field-plc-login",
    "field-plc-password",
    "field-host",
    "field-rack",
    "field-slot",
    "field-db-number",
    "field-port",
    "field-address",
    "field-comm-mode",
    "reason-connection-host",
    "reason-connection-rack",
    "reason-connection-slot",
    "reason-connection-data-block",
    "reason-connection-port",
    "reason-connection-address",
    "reason-connection-mode",
    "reason-connection-login",
];

/// The sources of the pages and the templates of the connection. They name keys as literals.
const KEY_SOURCES: &[&str] = &[
    include_str!("../../templates/machine-connection.html"),
    include_str!("../../templates/machine-connection-saved.html"),
    include_str!("../pages/machines/connection.rs"),
    include_str!("../pages/machines/connection_save.rs"),
];

/// Return the `en` source text: the base file and the approval file.
fn en_text() -> String {
    format!("{EN_SOURCE}\n{EN_APPROVAL_SOURCE}")
}

/// Return the `pl` source text: the base file and the approval file.
fn pl_text() -> String {
    format!("{PL_SOURCE}\n{PL_APPROVAL_SOURCE}")
}

/// Return the literal keys that a source passes to `get(` or `with_arg(`.
fn keys_of_calls(source: &str) -> Vec<&str> {
    ["get(\"", "with_arg(\""]
        .iter()
        .flat_map(|call| source.split(call).skip(1))
        .filter_map(|tail| tail.split_once('"').map(|(key, _rest)| key))
        .collect()
}

/// Return the names of the placeables `{ $name }` of a text, sorted.
fn placeables_of(text: &str) -> Vec<&str> {
    let mut names: Vec<&str> = text
        .split("{ $")
        .skip(1)
        .filter_map(|tail| tail.split_once(" }").map(|(name, _rest)| name))
        .collect();
    names.sort_unstable();
    names
}

#[test]
fn connection_texts_each_key_exists_in_pl_and_en() {
    let (pl, en) = (pl_text(), en_text());
    for key in CONNECTION_KEYS {
        assert!(ftl_message(&pl, key).is_some(), "{key} in the pl files");
        assert!(ftl_message(&en, key).is_some(), "{key} in the en files");
    }
}

#[test]
fn connection_texts_each_key_of_a_page_exists_in_pl_and_en() {
    let (pl, en) = (pl_text(), en_text());
    for source in KEY_SOURCES {
        for key in keys_of_calls(source) {
            assert!(ftl_message(&pl, key).is_some(), "{key} in the pl files");
            assert!(ftl_message(&en, key).is_some(), "{key} in the en files");
        }
    }
}

#[test]
fn connection_texts_error_english_equals_its_registry_row() {
    let en = en_text();
    for code in CONNECTION_CODES {
        let expected = fluent_text(registry_message(code).unwrap_or("<no registry row>"));
        let actual = ftl_message(&en, &format!("error-{code}"));
        assert_eq!(actual, Some(expected.as_str()), "{code}");
    }
}

#[test]
fn connection_texts_error_polish_has_the_placeables_of_the_english_text() {
    let (pl, en) = (pl_text(), en_text());
    for code in CONNECTION_CODES {
        let key = format!("error-{code}");
        let polish = ftl_message(&pl, &key);
        let english = ftl_message(&en, &key);
        assert!(polish.is_some(), "{key} in the pl files");
        assert_ne!(polish, english, "{key} has an English text in pl");
        assert_eq!(
            polish.map(placeables_of),
            english.map(placeables_of),
            "{key}"
        );
    }
}
