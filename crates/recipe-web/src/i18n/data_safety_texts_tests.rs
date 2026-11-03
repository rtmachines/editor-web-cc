//! Tests of the data-safety texts of the web UI (change `web-ui-data-safety`).

use super::tests::ftl_message;
use super::*;

#[test]
fn i18n_user_reset_warning_does_not_point_to_a_next_page() {
    for source in [EN_SOURCE, PL_SOURCE] {
        let text = ftl_message(source, "user-reset-warning").unwrap_or_default();
        assert!(!text.contains("next page"), "{text}");
        assert!(!text.contains("następnej stronie"), "{text}");
    }
}

#[test]
fn i18n_user_reset_warning_says_the_password_appears_only_once() {
    let en = ftl_message(EN_SOURCE, "user-reset-warning").unwrap_or_default();
    let pl = ftl_message(PL_SOURCE, "user-reset-warning").unwrap_or_default();
    assert!(en.contains("only once"), "{en}");
    assert!(pl.contains("tylko raz"), "{pl}");
}

#[test]
fn i18n_field_lines_limit_shows_the_maximum_count() {
    let bundles = Bundles::load().unwrap();
    let mut args = FluentArgs::new();
    args.set("max", "4000");
    assert_eq!(
        bundles
            .message(Lang::En, "field-lines-limit", Some(&args))
            .as_deref(),
        Some("At most 4000 characters.")
    );
    assert_eq!(
        bundles
            .message(Lang::Pl, "field-lines-limit", Some(&args))
            .as_deref(),
        Some("Najwyżej 4000 znaków.")
    );
}

#[test]
fn i18n_readonly_form_unavailable_says_the_mode_blocks_the_form() {
    let bundles = Bundles::load().unwrap();
    assert_eq!(
        bundles
            .message(Lang::En, "readonly-form-unavailable", None)
            .as_deref(),
        Some("Read-only mode blocks this form.")
    );
    assert_eq!(
        bundles
            .message(Lang::Pl, "readonly-form-unavailable", None)
            .as_deref(),
        Some("Tryb tylko do odczytu blokuje ten formularz.")
    );
}
