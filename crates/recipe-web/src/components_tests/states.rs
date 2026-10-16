//! Tests of the badge macros of the states "Deleted" and "Outdated" (task 8.4, `design-system.md`
//! sections 8 and 9, ADR-0043).
//!
//! A test template below `templates/test/` calls one macro and takes the word from a struct field.
//! The expected markup is written here, in the form of the design. Each state shows the icon, the
//! word, and the tokens together, so the color is never the only signal.

use askama::Template;

use super::{Lang, assert_fixed_svg_tags, icon, text};

#[derive(Template)]
#[template(path = "test/state-badge.html")]
struct StateBadgePage {
    kind: &'static str,
    word: String,
    large: bool,
}

/// Render the badge of `kind` with the word `word`.
fn render(kind: &'static str, word: &str, large: bool) -> String {
    StateBadgePage {
        kind,
        word: word.to_owned(),
        large,
    }
    .render()
    .unwrap()
}

#[test]
fn components_deleted_badge_has_its_class_the_retired_icon_and_the_word_in_both_languages() {
    for lang in [Lang::Pl, Lang::En] {
        let word = text(lang, "status-deleted");
        for (large, class) in [
            (false, "badge badge-deleted"),
            (true, "badge badge-large badge-deleted"),
        ] {
            let expected = format!(
                "<span class=\"{class}\">{}<span>{word}</span></span>",
                icon("status-retired")
            );
            assert_eq!(
                render("deleted", &word, large).trim(),
                expected,
                "{lang} large {large}"
            );
        }
    }
}

#[test]
fn components_outdated_badge_has_its_class_the_warning_icon_and_the_word_in_both_languages() {
    for lang in [Lang::Pl, Lang::En] {
        let word = text(lang, "status-outdated");
        for (large, class) in [
            (false, "badge badge-outdated"),
            (true, "badge badge-large badge-outdated"),
        ] {
            let expected = format!(
                "<span class=\"{class}\">{}<span>{word}</span></span>",
                icon("alert-warning")
            );
            assert_eq!(
                render("outdated", &word, large).trim(),
                expected,
                "{lang} large {large}"
            );
        }
    }
}

#[test]
fn components_state_badge_words_are_the_bundle_words() {
    assert_eq!(text(Lang::En, "status-deleted"), "Deleted");
    assert_eq!(text(Lang::Pl, "status-deleted"), "Usunięta");
    assert_eq!(text(Lang::En, "status-outdated"), "Outdated");
    assert_eq!(text(Lang::Pl, "status-outdated"), "Nieaktualny");
}

#[test]
fn components_state_badge_icons_are_fixed_tags_and_the_badge_carries_no_style_or_handler() {
    for (kind, large) in [
        ("deleted", false),
        ("deleted", true),
        ("outdated", false),
        ("outdated", true),
    ] {
        let html = render(kind, "w", large);
        assert_fixed_svg_tags(&html);
        assert_eq!(html.matches("<svg").count(), 1, "{kind}");
        for forbidden in [" style=", " on", "<script", "<title"] {
            assert!(!html.contains(forbidden), "{kind}: found {forbidden}");
        }
    }
}

#[test]
fn components_state_badge_word_is_escaped() {
    for kind in ["deleted", "outdated"] {
        let html = render(kind, "<script>x</script>", false);
        assert!(!html.contains("<script"), "{kind}");
        assert!(html.contains("&#60;script&#62;"), "{kind}");
    }
}

#[test]
fn components_outdated_and_deleted_badges_differ_in_class_and_icon() {
    let deleted = render("deleted", "w", false);
    let outdated = render("outdated", "w", false);
    assert_ne!(deleted, outdated);
    assert!(deleted.contains("data-icon=\"status-retired\""));
    assert!(outdated.contains("data-icon=\"alert-warning\""));
}
