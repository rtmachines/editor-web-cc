//! Tests of the words of the audit rows (design D8): every action and every entity type has a word
//! in both bundles.

use recipe_app::ports::{AuditEntityType, EntityLabel};
use recipe_domain::audit::AuditAction;

use super::{action_word, entity_word, kebab, label_text};
use crate::i18n::{Bundles, Lang};
use crate::pages::texts::Texts;

#[test]
fn rows_kebab_turns_a_name_of_words_into_the_key_part() {
    assert_eq!(kebab("VersionApproved"), "version-approved");
    assert_eq!(kebab("UdtDefined"), "udt-defined");
    assert_eq!(kebab("Machine"), "machine");
}

#[test]
fn rows_each_action_has_a_word_in_both_bundles_and_none_equals_its_key() {
    let bundles = Bundles::load().unwrap();
    for lang in [Lang::En, Lang::Pl] {
        let t = Texts::new(&bundles, lang);
        for action in AuditAction::ALL {
            let word = action_word(t, action);
            assert!(
                !word.starts_with("audit-action-"),
                "{lang} {action}: {word}"
            );
        }
    }
}

#[test]
fn rows_each_entity_type_has_a_word_in_both_bundles() {
    let bundles = Bundles::load().unwrap();
    for lang in [Lang::En, Lang::Pl] {
        let t = Texts::new(&bundles, lang);
        for entity_type in AuditEntityType::ALL {
            let word = entity_word(t, entity_type);
            assert!(!word.starts_with("audit-entity-"), "{lang}: {word}");
        }
    }
}

#[test]
fn rows_a_missing_label_shows_the_short_id() {
    assert_eq!(label_text(&EntityLabel::Missing, "1a2b3c4d"), "1a2b3c4d");
    assert_eq!(label_text(&EntityLabel::Version(3), "x"), "3");
}
