//! Tests of the second resource file of each language (change `approval-audit-history`, task 2.1).

use super::*;

#[test]
fn i18n_resource_key_of_the_second_english_file_resolves() {
    let bundles = Bundles::load().unwrap();

    let text = bundles.message(Lang::En, "nav-audit", None);

    assert_eq!(text.as_deref(), Some("Audit"));
}

#[test]
fn i18n_resource_key_of_the_second_polish_file_resolves() {
    let bundles = Bundles::load().unwrap();

    let text = bundles.message(Lang::Pl, "nav-audit", None);

    assert_eq!(text.as_deref(), Some("Audyt"));
}

#[test]
fn i18n_resource_key_in_both_files_of_one_language_fails_the_load() {
    let shadow = "nav-audit = Shadow\n";

    let second_file = load_bundle(Lang::En, &[shadow, EN_APPROVAL_SOURCE]);
    let first_file = load_bundle(Lang::Pl, &[PL_SOURCE, "nav-label = Cień\n"]);

    assert_eq!(second_file.err(), Some(BundleReason { lang: Lang::En }));
    assert_eq!(first_file.err(), Some(BundleReason { lang: Lang::Pl }));
}

#[test]
fn i18n_resource_keys_of_two_files_without_overlap_load() {
    let bundle = load_bundle(Lang::En, &["first = One\n", "second = Two\n"]).unwrap();

    assert!(bundle.has_message("first"));
    assert!(bundle.has_message("second"));
}

#[test]
fn i18n_resource_key_lists_include_the_second_files() {
    let bundles = Bundles::load().unwrap();

    assert!(bundles.keys(Lang::En).contains(&"nav-audit"));
    assert!(bundles.keys(Lang::Pl).contains(&"nav-audit"));
    assert!(bundles.has_message(Lang::En, "nav-audit"));
    assert!(bundles.has_message(Lang::Pl, "nav-audit"));
}
