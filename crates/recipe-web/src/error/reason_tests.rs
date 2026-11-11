//! Tests of the 14 field reasons of the recipe forms (task 7.1, design D6 and D13).
//! Tests of the 7 field reasons of the filter inputs (step-7 item S7-3, design D6 and D8).
//! Tests of the 8 field reasons of the connection form (task 7.4 of `plc-connection-config`).

use super::reason::FieldReason;
use crate::i18n::{Bundles, Lang};

/// The reasons that design D6 adds, with their keys, in the order of `ALL`.
const RECIPE_REASON_KEYS: [(FieldReason, &str); 14] = [
    (FieldReason::ReferenceNumber, "reason-reference-number"),
    (FieldReason::ChangeNote, "reason-change-note"),
    (FieldReason::Description, "reason-description"),
    (
        FieldReason::TranslationDescription,
        "reason-translation-description",
    ),
    (FieldReason::Tag, "reason-tag"),
    (FieldReason::Ordinal, "reason-ordinal"),
    (FieldReason::S7Address, "reason-s7-address"),
    (FieldReason::BasedOn, "reason-based-on"),
    (FieldReason::Status, "reason-status"),
    (FieldReason::ValueKey, "reason-value-key"),
    (FieldReason::LimitRow, "reason-limit-row"),
    (FieldReason::MaxLength, "reason-max-length"),
    (FieldReason::LangList, "reason-lang-list"),
    (FieldReason::Reload, "reason-reload"),
];

/// The reasons of the filter inputs (design D6), with their keys and the English texts of
/// design D8, in the order of `ALL`. The texts are copied from the table, not from the bundles.
const FILTER_REASONS: [(FieldReason, &str, &str); 7] = [
    (
        FieldReason::Date,
        "reason-date",
        "a date has the form YYYY-MM-DD, for example 2026-10-17",
    ),
    (
        FieldReason::DateOrder,
        "reason-date-order",
        "the end date is the same as the start date or later",
    ),
    (
        FieldReason::Choice,
        "reason-choice",
        "choose a value from the list",
    ),
    (
        FieldReason::Uuid,
        "reason-uuid",
        "an identifier has 36 characters in the form 8-4-4-4-12 of the digits 0 to 9 and the letters a to f",
    ),
    (
        FieldReason::EntityWithoutType,
        "reason-entity-without-type",
        "choose an object type for the object",
    ),
    (
        FieldReason::Address,
        "reason-address",
        "an address is an IPv4 or an IPv6 address, for example 192.0.2.10",
    ),
    (
        FieldReason::Cursor,
        "reason-cursor",
        "the link of the page is not valid. Open the first page of the list",
    ),
];

#[test]
fn reason_all_holds_the_14_recipe_reasons_at_20_to_33_in_the_order_of_design_d6() {
    let part = FieldReason::ALL.get(20..34).unwrap();
    let expected: Vec<FieldReason> = RECIPE_REASON_KEYS
        .iter()
        .map(|(reason, _)| *reason)
        .collect();
    assert_eq!(part, expected.as_slice());
}

#[test]
fn reason_each_recipe_reason_has_its_key_in_both_bundles_and_the_english_log_text() {
    let bundles = Bundles::load().unwrap();
    for (reason, key) in RECIPE_REASON_KEYS {
        assert_eq!(reason.key(), key);
        assert!(bundles.has_message(Lang::Pl, key), "{key} in pl");
        let english = bundles.message(Lang::En, key, None).unwrap();
        assert_eq!(english, reason.to_string(), "{key}");
    }
}

#[test]
fn reason_all_holds_the_7_filter_reasons_at_34_to_40_in_the_order_of_design_d6() {
    let tail = FieldReason::ALL.get(34..41).unwrap();
    let expected: Vec<FieldReason> = FILTER_REASONS
        .iter()
        .map(|(reason, _, _)| *reason)
        .collect();
    assert_eq!(tail, expected.as_slice());
}

#[test]
fn reason_each_filter_reason_has_its_key_in_both_bundles_and_the_english_text_of_design_d8() {
    let bundles = Bundles::load().unwrap();
    for (reason, key, english) in FILTER_REASONS {
        assert_eq!(reason.key(), key);
        assert!(bundles.has_message(Lang::Pl, key), "{key} in pl");
        assert_eq!(reason.to_string(), english, "{key} log text");
        let message = bundles.message(Lang::En, key, None);
        assert_eq!(message.as_deref(), Some(english), "{key} in en");
    }
}

/// The reasons of the connection form, with their keys and English texts, in the order of `ALL`.
const CONNECTION_REASONS: [(FieldReason, &str, &str); 8] = [
    (
        FieldReason::ConnectionHost,
        "reason-connection-host",
        "a host is an IPv4 address or a host name of at most 253 characters",
    ),
    (
        FieldReason::ConnectionRack,
        "reason-connection-rack",
        "the rack is a whole number from 0 to 7",
    ),
    (
        FieldReason::ConnectionSlot,
        "reason-connection-slot",
        "the slot is a whole number from 0 to 31",
    ),
    (
        FieldReason::ConnectionDataBlock,
        "reason-connection-data-block",
        "the data block is a whole number from 1 to 65535",
    ),
    (
        FieldReason::ConnectionPort,
        "reason-connection-port",
        "the port is a whole number from 1 to 65535",
    ),
    (
        FieldReason::ConnectionAddress,
        "reason-connection-address",
        "an address has 1 to 200 characters and no control character",
    ),
    (
        FieldReason::ConnectionMode,
        "reason-connection-mode",
        "choose a communication mode from the list",
    ),
    (
        FieldReason::ConnectionLogin,
        "reason-connection-login",
        "a login has 1 to 64 characters and no control character",
    ),
];

#[test]
fn reason_all_ends_with_the_8_connection_reasons_in_the_order_of_the_table() {
    let tail = FieldReason::ALL.get(41..).unwrap();
    let expected: Vec<FieldReason> = CONNECTION_REASONS
        .iter()
        .map(|(reason, _, _)| *reason)
        .collect();
    assert_eq!(tail, expected.as_slice());
}

#[test]
fn reason_each_connection_reason_has_its_key_in_both_bundles_and_its_english_text() {
    let bundles = Bundles::load().unwrap();
    for (reason, key, english) in CONNECTION_REASONS {
        assert_eq!(reason.key(), key);
        assert!(bundles.has_message(Lang::Pl, key), "{key} in pl");
        assert_eq!(reason.to_string(), english, "{key} log text");
        let message = bundles.message(Lang::En, key, None);
        assert_eq!(message.as_deref(), Some(english), "{key} in en");
    }
}
