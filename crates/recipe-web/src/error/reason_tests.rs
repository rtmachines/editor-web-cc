//! Tests of the 14 field reasons of the recipe forms (task 7.1, design D6 and D13).

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

#[test]
fn reason_all_ends_with_the_14_recipe_reasons_in_the_order_of_design_d6() {
    let tail = FieldReason::ALL.get(20..).unwrap();
    let expected: Vec<FieldReason> = RECIPE_REASON_KEYS
        .iter()
        .map(|(reason, _)| *reason)
        .collect();
    assert_eq!(tail, expected.as_slice());
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
