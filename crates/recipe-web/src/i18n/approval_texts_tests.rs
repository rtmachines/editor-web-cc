//! Tests of the texts of the approval and audit pages (change `approval-audit-history`, design D8).

use super::*;

/// Every message key of the table D8, in the order of the table.
const D8_KEYS: &[&str] = &[
    "nav-audit",
    "version-actions-heading",
    "version-approve-link",
    "version-compare-approved",
    "version-compare-base",
    "recipe-history-link",
    "retire-summary",
    "retire-notice",
    "retire-button",
    "discard-summary",
    "discard-notice",
    "discard-button",
    "delete-recipe-summary",
    "delete-recipe-notice",
    "delete-recipe-button",
    "restore-recipe-button",
    "form-in-progress",
    "toast-version-approved",
    "toast-version-approved-retired",
    "version-already-approved",
    "toast-version-retired",
    "approve-title",
    "approve-retires-notice",
    "approve-first-notice",
    "approve-go-to-form",
    "approve-form-heading",
    "approve-outdated-legend",
    "approve-allow-outdated",
    "approve-allow-outdated-hint",
    "approve-button",
    "approve-back",
    "approve-blockers-heading",
    "approve-blocker-editor-hint",
    "approve-changed-draft",
    "approve-first-page",
    "field-allow-outdated",
    "field-rv",
    "field-kind",
    "field-aggregate",
    "field-entity-type",
    "field-entity-id",
    "field-actor",
    "field-action",
    "field-from",
    "field-to",
    "field-before",
    "field-client-ip",
    "task-stage-diff",
    "diff-title",
    "diff-summary-heading",
    "diff-pager-label",
    "diff-section-limits",
    "diff-section-values",
    "diff-section-definitions",
    "diff-section-translations",
    "diff-section-added",
    "diff-section-removed",
    "diff-count-parameters",
    "diff-count-rows",
    "diff-unchanged",
    "diff-caption",
    "diff-col-parameter",
    "diff-col-unit",
    "diff-col-path",
    "diff-col-version",
    "diff-col-limit",
    "diff-col-field",
    "diff-col-lang",
    "diff-col-name",
    "diff-col-type",
    "diff-col-value",
    "diff-col-state",
    "diff-field-tag",
    "diff-field-unit",
    "diff-field-type",
    "diff-field-s7",
    "diff-field-ordinal",
    "diff-field-name-en",
    "diff-field-description-en",
    "diff-field-tr-name",
    "diff-field-tr-description",
    "diff-none",
    "diff-limit-range",
    "diff-limit-min-only",
    "diff-limit-max-only",
    "diff-limit-max-length",
    "diff-leaf-count",
    "diff-more-rows",
    "diff-open-version",
    "diff-empty",
    "diff-back",
    "diff-kind-empty",
    "reason-date",
    "reason-date-order",
    "reason-choice",
    "reason-uuid",
    "reason-entity-without-type",
    "reason-address",
    "reason-cursor",
    "diff-state-limit",
    "diff-state-value",
    "diff-state-definition",
    "diff-state-translation",
    "diff-state-added",
    "diff-state-removed",
    "history-title",
    "history-caption",
    "history-empty",
    "audit-col-time",
    "audit-col-actor",
    "audit-col-action",
    "audit-col-entity",
    "audit-actor-system",
    "audit-entry-link",
    "audit-whole-recipe",
    "audit-title",
    "audit-filter-heading",
    "audit-filter-entity-type",
    "audit-filter-any-type",
    "audit-filter-action",
    "audit-filter-any-action",
    "audit-filter-actor",
    "audit-filter-from",
    "audit-filter-to",
    "audit-filter-submit",
    "audit-filter-clear",
    "audit-filter-aggregate",
    "audit-filter-aggregate-id",
    "audit-filter-aggregate-clear",
    "audit-filter-object",
    "audit-filter-object-clear",
    "audit-group-plant",
    "audit-group-recipe",
    "audit-group-identity",
    "audit-group-export",
    "audit-group-settings",
    "audit-caption",
    "audit-empty",
    "audit-newest",
    "audit-older",
    "audit-no-older",
    "audit-security-link",
    "audit-entry-title",
    "audit-entry-time",
    "audit-entry-actor",
    "audit-entry-action",
    "audit-entry-entity-type",
    "audit-entry-entity",
    "audit-entry-aggregate",
    "audit-entry-request",
    "audit-entry-client",
    "audit-entry-session",
    "audit-entry-chain",
    "audit-entry-before",
    "audit-entry-after",
    "audit-entry-no-state",
    "audit-entry-object-link",
    "audit-entry-aggregate-link",
    "audit-entity-machine",
    "audit-entity-unit",
    "audit-entity-plc-type",
    "audit-entity-recipe",
    "audit-entity-recipe-version",
    "audit-entity-parameter",
    "audit-entity-app-user",
    "audit-entity-app-setting",
    "audit-action-machine-created",
    "audit-action-machine-renamed",
    "audit-action-machine-archived",
    "audit-action-unit-added",
    "audit-action-unit-renamed",
    "audit-action-unit-archived",
    "audit-action-udt-defined",
    "audit-action-derived-type-created",
    "audit-action-recipe-created",
    "audit-action-recipe-renamed",
    "audit-action-recipe-deleted",
    "audit-action-recipe-restored",
    "audit-action-version-created",
    "audit-action-draft-discarded",
    "audit-action-version-approved",
    "audit-action-version-retired",
    "audit-action-parameter-added",
    "audit-action-parameter-definition-updated",
    "audit-action-parameter-value-set",
    "audit-action-limits-replaced",
    "audit-action-translation-upserted",
    "audit-action-translation-removed",
    "audit-action-parameter-removed",
    "audit-action-type-revision-rebound",
    "audit-action-user-logged-in",
    "audit-action-user-logged-out",
    "audit-action-user-created",
    "audit-action-user-role-set",
    "audit-action-user-deactivated",
    "audit-action-user-activated",
    "audit-action-password-reset",
    "audit-action-own-password-changed",
    "audit-action-login-unlocked",
    "audit-action-version-exported",
    "audit-action-setting-changed",
    "security-title",
    "security-caption",
    "security-empty",
    "security-filter-kind",
    "security-filter-any-kind",
    "security-filter-login",
    "security-filter-login-hint",
    "security-filter-client-ip",
    "security-col-count",
    "security-col-agent",
    "security-col-request",
    "security-col-detail",
    "security-login-unknown",
    "security-kind-login-failed",
    "security-kind-login-rejected-inactive",
    "security-kind-login-throttled",
    "security-kind-csrf-rejected",
    "security-kind-forbidden",
    "security-kind-unlock",
    "security-kind-audit-chain-broken",
    "error-E-EXP-004",
    "missing-audit-entry",
    "missing-malformed-audit-entry",
];

/// One sample of the table D8: the key, its arguments, the English text, and the Polish text.
type Sample = (
    &'static str,
    &'static [(&'static str, &'static str)],
    &'static str,
    &'static str,
);

/// A sample of the table D8. The texts are copied from the table, not from the bundles.
const D8_SAMPLES: &[Sample] = &[
    ("nav-audit", &[], "Audit", "Audyt"),
    (
        "audit-filter-aggregate-id",
        &[("label", "12345678")],
        "Aggregate: 12345678",
        "Agregat: 12345678",
    ),
    (
        "version-approve-link",
        &[("number", "3")],
        "Approve version 3",
        "Zatwierdź wersję 3",
    ),
    (
        "retire-notice",
        &[],
        "The recipe then has no approved version. The export of the approved version stops until the next approval.",
        "Receptura nie będzie wtedy mieć wersji zatwierdzonej. Eksport wersji zatwierdzonej przestanie działać do następnego zatwierdzenia.",
    ),
    (
        "discard-button",
        &[("number", "2")],
        "Discard draft 2",
        "Odrzuć szkic 2",
    ),
    (
        "toast-version-approved-retired",
        &[("number", "3"), ("retired", "2")],
        "Version 3 is approved. Version 2 is retired.",
        "Wersja 3 jest zatwierdzona. Wersja 2 jest wycofana.",
    ),
    (
        "approve-title",
        &[("number", "2"), ("reference", "R-500")],
        "Approval of version 2 of recipe R-500",
        "Zatwierdzenie wersji 2 receptury R-500",
    ),
    (
        "approve-first-notice",
        &[("reference", "R-500")],
        "Recipe R-500 has no approved version. Every parameter is new.",
        "Receptura R-500 nie ma wersji zatwierdzonej. Każdy parametr jest nowy.",
    ),
    (
        "diff-title",
        &[("new", "2"), ("reference", "R-500"), ("old", "1")],
        "Changes of version 2 of recipe R-500 against version 1",
        "Zmiany wersji 2 receptury R-500 względem wersji 1",
    ),
    (
        "diff-back",
        &[("number", "3")],
        "Back to version 3",
        "Wróć do wersji 3",
    ),
    (
        "diff-kind-empty",
        &[("section", "Values")],
        "The section “Values” has no change.",
        "Sekcja „Values” nie ma zmian.",
    ),
    (
        "reason-date",
        &[],
        "a date has the form YYYY-MM-DD, for example 2026-10-17",
        "data ma postać RRRR-MM-DD, na przykład 2026-10-17",
    ),
    (
        "reason-date-order",
        &[],
        "the end date is the same as the start date or later",
        "data końcowa jest taka sama jak data początkowa albo późniejsza",
    ),
    (
        "reason-choice",
        &[],
        "choose a value from the list",
        "wybierz wartość z listy",
    ),
    (
        "reason-uuid",
        &[],
        "an identifier has 36 characters in the form 8-4-4-4-12 of the digits 0 to 9 and the letters a to f",
        "identyfikator ma 36 znaków w postaci 8-4-4-4-12 z cyfr od 0 do 9 i liter od a do f",
    ),
    (
        "reason-entity-without-type",
        &[],
        "choose an object type for the object",
        "wybierz typ obiektu dla obiektu",
    ),
    (
        "reason-address",
        &[],
        "an address is an IPv4 or an IPv6 address, for example 192.0.2.10",
        "adres to adres IPv4 albo IPv6, na przykład 192.0.2.10",
    ),
    (
        "reason-cursor",
        &[],
        "the link of the page is not valid. Open the first page of the list",
        "link strony jest nieprawidłowy. Otwórz pierwszą stronę listy",
    ),
    (
        "diff-limit-range",
        &[("min", "0.5"), ("max", "9")],
        "0.5 to 9",
        "0.5 do 9",
    ),
    (
        "diff-limit-min-only",
        &[("min", "0.5")],
        "at least 0.5",
        "co najmniej 0.5",
    ),
    (
        "diff-limit-max-only",
        &[("max", "9")],
        "at most 9",
        "najwyżej 9",
    ),
    (
        "diff-limit-max-length",
        &[("max_length", "16")],
        "Maximum length: 16",
        "Maksymalna długość: 16",
    ),
    (
        "audit-action-udt-defined",
        &[],
        "Struct defined",
        "Zdefiniowano strukturę",
    ),
    (
        "audit-filter-from",
        &[],
        "From (UTC, YYYY-MM-DD)",
        "Od (UTC, RRRR-MM-DD)",
    ),
    (
        "security-kind-csrf-rejected",
        &[],
        "Request without a valid token",
        "Żądanie bez ważnego tokenu",
    ),
    (
        "security-filter-login-hint",
        &[],
        "A row with a count above 1 shows only the login of its first event. The filter \"Client address\" finds every attempt.",
        "Wiersz z liczbą większą niż 1 pokazuje tylko login swojego pierwszego zdarzenia. Filtr „Adres klienta” znajduje każdą próbę.",
    ),
    (
        "error-E-EXP-004",
        &[],
        "The server cannot create the export document. Contact the administrator.",
        "Serwer nie może utworzyć dokumentu eksportu. Skontaktuj się z administratorem.",
    ),
];

/// Format the message `key` in `lang` with the arguments `args`.
fn format_sample(
    bundles: &Bundles,
    lang: Lang,
    key: &str,
    args: &[(&str, &str)],
) -> Option<String> {
    let mut fluent_args = FluentArgs::new();
    for (name, value) in args {
        fluent_args.set(*name, *value);
    }
    bundles.message(lang, key, Some(&fluent_args))
}

#[test]
fn i18n_approval_texts_equal_the_sample_of_design_d8_in_both_languages() {
    let bundles = Bundles::load().unwrap();

    for (key, args, english, polish) in D8_SAMPLES {
        for (lang, expected) in [(Lang::En, english), (Lang::Pl, polish)] {
            let found = format_sample(&bundles, lang, key, args);
            assert_eq!(found.as_deref(), Some(*expected), "{key} in {lang}");
        }
    }
}

/// Return the message keys that `source` defines. Each key starts a line at column 0.
///
/// The function skips comment lines, attribute lines, indented continuation lines, and term lines.
fn approval_message_keys(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter(|line| line.starts_with(|c: char| c.is_ascii_alphabetic()))
        .filter_map(|line| line.split_once('=').map(|(id, _text)| id.trim_end()))
        .collect()
}

#[test]
fn approval_message_keys_reads_ids_that_start_uppercase_or_have_no_blank() {
    let source = "Key-One = a\nkey-two=b\n";

    assert_eq!(approval_message_keys(source), vec!["Key-One", "key-two"]);
}

#[test]
fn approval_message_keys_skips_comments_terms_attributes_and_indented_lines() {
    let source = "# comment = x\n-term = y\n    .attr = z\n    other = w\nkey =\n    value\n";

    assert_eq!(approval_message_keys(source), vec!["key"]);
}

#[test]
fn i18n_approval_texts_have_no_key_outside_design_d8_in_both_languages() {
    for (lang, source) in [
        (Lang::Pl, PL_APPROVAL_SOURCE),
        (Lang::En, EN_APPROVAL_SOURCE),
    ] {
        let keys = approval_message_keys(source);
        assert_eq!(
            keys.len(),
            D8_KEYS.len(),
            "the {lang} approval file does not define the keys of design D8 once each"
        );
        for key in keys {
            assert!(
                D8_KEYS.contains(&key),
                "the key {key} of the {lang} approval file is not in design D8"
            );
        }
    }
}

#[test]
fn i18n_approval_texts_have_every_key_of_design_d8_in_both_languages() {
    let bundles = Bundles::load().unwrap();

    for lang in [Lang::Pl, Lang::En] {
        for key in D8_KEYS {
            assert!(
                bundles.has_message(lang, key),
                "the key {key} has no message in {lang}"
            );
        }
    }
}
