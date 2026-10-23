//! Tests of the texts of the export error codes (change `export-v1`, design D9).

use super::*;

/// One sample of the table D9: the key, its arguments, the English text, and the Polish text.
type Sample = (
    &'static str,
    &'static [(&'static str, &'static str)],
    &'static str,
    &'static str,
);

/// A sample of the table D9. The texts are copied from the table, not from the bundles.
const D9_SAMPLES: &[Sample] = &[
    (
        "error-E-EXP-001",
        &[("reference", "R-500")],
        "Recipe R-500 has no approved version.",
        "Receptura R-500 nie ma wersji zatwierdzonej.",
    ),
    (
        "error-E-EXP-002",
        &[("number", "2"), ("reference", "R-500")],
        "Version 2 of recipe R-500 does not match its stored hash. The export is refused. Contact the administrator.",
        "Wersja 2 receptury R-500 nie zgadza się z zapisanym skrótem. Eksport jest odrzucony. Skontaktuj się z administratorem.",
    ),
    (
        "error-E-EXP-003",
        &[("number", "2"), ("reference", "R-500")],
        "Version 2 of recipe R-500 is a draft or was discarded. Only approved and retired versions can be exported.",
        "Wersja 2 receptury R-500 jest szkicem albo została odrzucona. Eksportować można tylko wersje zatwierdzone i wycofane.",
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
fn i18n_export_texts_equal_the_sample_of_design_d9_in_both_languages() {
    let bundles = Bundles::load().unwrap();

    for (key, args, english, polish) in D9_SAMPLES {
        for (lang, expected) in [(Lang::En, english), (Lang::Pl, polish)] {
            let found = format_sample(&bundles, lang, key, args);
            assert_eq!(found.as_deref(), Some(*expected), "{key} in {lang}");
        }
    }
}
