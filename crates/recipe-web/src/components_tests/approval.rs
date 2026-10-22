//! Tests of the components of the approval and audit pages (design D7, D8, `design-system.md` 8,
//! 10.5, 10.12, task 9.1).
//!
//! The components are the danger disclosure with a `row_version`, the pager with a label of the
//! caller, the cursor pager, and the four icons of the diff states. A test template below
//! `templates/test/` calls one macro and takes every text from a struct field. The expected markup
//! is written here, in the form of the design.

use askama::Template;

use super::{Bundles, DIFF_ICONS, Lang, Texts, assert_fixed_svg_tags, icon, text};

/// The design document, for the check of the icon table.
const DESIGN: &str = include_str!("../../../../docs/architecture/design-system.md");

#[derive(Template)]
#[template(path = "test/danger-versioned.html")]
struct VersionedDangerPage<'a> {
    t: Texts<'a>,
    summary: String,
    warning: String,
    button: String,
    action: String,
    csrf: String,
    row_version: i64,
}

#[derive(Template)]
#[template(path = "test/cursor-pager.html")]
struct CursorPagerPage<'a> {
    t: Texts<'a>,
    label: String,
    newest: String,
    older: String,
}

#[derive(Template)]
#[template(path = "test/pager-named.html")]
struct NamedPagerPage<'a> {
    t: Texts<'a>,
    label: String,
    previous: String,
    next: String,
    current: u32,
}

#[derive(Template)]
#[template(path = "test/diff-icons.html")]
struct DiffIconsPage;

/// Render the retire disclosure of version 2 in `lang` with the given values.
fn versioned_with(lang: Lang, action: &str, csrf: &str, row_version: i64) -> String {
    let bundles = Bundles::load().unwrap();
    VersionedDangerPage {
        t: Texts::new(&bundles, lang),
        summary: text(lang, "retire-summary"),
        warning: text(lang, "retire-notice"),
        button: "Retire version 2".to_owned(),
        action: action.to_owned(),
        csrf: csrf.to_owned(),
        row_version,
    }
    .render()
    .unwrap()
}

/// Render the retire disclosure of `R-100` with the token `TOKEN` and the row version 7.
fn versioned(lang: Lang) -> String {
    versioned_with(lang, "/recipes/R-100/versions/2/retire", "TOKEN", 7)
}

#[test]
fn components_danger_versioned_is_the_form_of_design_d7_in_both_languages() {
    for lang in [Lang::En, Lang::Pl] {
        let expected = format!(
            "<details class=\"danger\"><summary>{}{}<span>{}</span></summary><form method=\"post\" action=\"/recipes/R-100/versions/2/retire\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\"><input type=\"hidden\" name=\"csrf_token\" value=\"TOKEN\"><input type=\"hidden\" name=\"row_version\" value=\"7\"><div id=\"danger-notice\"><div class=\"alert alert-warning\">{}<span>{}</span></div></div><button type=\"submit\" class=\"btn-danger\" aria-describedby=\"danger-notice\">Retire version 2</button><span class=\"htmx-indicator\">{}</span></form></details>",
            icon("tree-collapsed"),
            icon("tree-expanded"),
            text(lang, "retire-summary"),
            icon("alert-warning"),
            text(lang, "retire-notice"),
            text(lang, "form-in-progress"),
        );
        assert_eq!(versioned(lang).trim(), expected, "{lang}");
    }
}

#[test]
fn components_danger_versioned_notice_has_the_fixed_id_once_and_no_role() {
    let html = versioned(Lang::En);
    assert_eq!(html.matches("id=\"danger-notice\"").count(), 1, "{html}");
    assert!(!html.contains("role="), "{html}");
    assert!(!html.contains("aria-live"), "{html}");
}

#[test]
fn components_danger_versioned_button_names_the_notice_in_aria_describedby() {
    let html = versioned(Lang::En);
    let button = html.split("<button").nth(1).unwrap();
    let tag = button.split('>').next().unwrap();
    assert!(tag.contains("aria-describedby=\"danger-notice\""), "{tag}");
    assert_eq!(html.matches("aria-describedby").count(), 1, "{html}");
}

#[test]
fn components_danger_versioned_row_version_is_a_hidden_field_with_the_number() {
    let html = versioned_with(Lang::En, "/x", "T", 12);
    assert!(
        html.contains("<input type=\"hidden\" name=\"row_version\" value=\"12\">"),
        "{html}"
    );
    assert_eq!(html.matches("name=\"row_version\"").count(), 1);
    assert_eq!(html.matches("name=\"csrf_token\"").count(), 1);
}

#[test]
fn components_danger_versioned_form_holds_four_literal_hx_attributes_and_the_indicator() {
    let html = versioned_with(Lang::En, "/a\"><script>x</script>", "\"><b>", 1);
    for literal in [
        "hx-boost=\"true\"",
        "hx-target=\"#content\"",
        "hx-push-url=\"true\"",
        "hx-sync=\"closest form:drop\"",
    ] {
        assert_eq!(html.matches(literal).count(), 1, "{literal}: {html}");
    }
    assert!(!html.contains("hx-disabled-elt"), "{html}");
    assert_eq!(html.matches(" hx-").count(), 4, "{html}");
    assert_eq!(html.matches("class=\"htmx-indicator\"").count(), 1);
    for raw in ["<script", "<b>", " style=", " on"] {
        assert!(!html.contains(raw), "found {raw}: {html}");
    }
    assert!(
        html.contains("action=\"/a&#34;&#62;&#60;script&#62;"),
        "{html}"
    );
}

/// Render the cursor pager with the label of the caller and the two addresses.
fn cursor(lang: Lang, label: &str, newest: &str, older: &str) -> String {
    let bundles = Bundles::load().unwrap();
    CursorPagerPage {
        t: Texts::new(&bundles, lang),
        label: label.to_owned(),
        newest: newest.to_owned(),
        older: older.to_owned(),
    }
    .render()
    .unwrap()
}

#[test]
fn components_cursor_pager_with_an_older_row_has_two_target_links_in_order() {
    for lang in [Lang::En, Lang::Pl] {
        let label = text(lang, "pager-label");
        let expected = format!(
            "<nav class=\"pager\" aria-label=\"{label}\"><a class=\"target-link\" href=\"/audit\">{}</a><a class=\"target-link\" href=\"/audit?before=40\">{}</a></nav>",
            text(lang, "audit-newest"),
            text(lang, "audit-older"),
        );
        let html = cursor(lang, &label, "/audit", "/audit?before=40");
        assert_eq!(html.trim(), expected, "{lang}");
    }
}

#[test]
fn components_cursor_pager_on_the_last_page_shows_the_text_in_place_of_the_second_link() {
    for lang in [Lang::En, Lang::Pl] {
        let html = cursor(lang, "Pages", "/audit", "");
        let expected = format!(
            "<nav class=\"pager\" aria-label=\"Pages\"><a class=\"target-link\" href=\"/audit\">{}</a><span class=\"pager-end\">{}</span></nav>",
            text(lang, "audit-newest"),
            text(lang, "audit-no-older"),
        );
        assert_eq!(html.trim(), expected, "{lang}");
        assert_eq!(html.matches("<a ").count(), 1, "{html}");
        assert!(!html.contains(&text(lang, "audit-older")), "{html}");
    }
}

#[test]
fn components_cursor_pager_name_is_the_label_of_the_caller_with_the_section_name() {
    let bundles = Bundles::load().unwrap();
    let texts = Texts::new(&bundles, Lang::En);
    let label = texts.with_arg("diff-pager-label", "section", "Values");
    let html = cursor(Lang::En, &label, "/a", "/b");
    assert!(html.contains(&format!("aria-label=\"{label}\"")), "{html}");
    assert!(label.contains("Values"), "{label}");
}

#[test]
fn components_cursor_pager_escapes_the_label_and_the_addresses() {
    let html = cursor(
        Lang::En,
        "\"><script>x</script>",
        "/x\"><script>y</script>",
        "/y?a=1&b=2",
    );
    assert!(!html.contains("<script"), "{html}");
    assert!(html.contains("href=\"/y?a=1&#38;b=2\""), "{html}");
    assert_eq!(
        html.matches("<a class=\"target-link\"").count(),
        html.matches("<a ").count()
    );
}

/// Render the page pager with the label of the caller.
fn named(label: &str, previous: &str, next: &str) -> String {
    let bundles = Bundles::load().unwrap();
    NamedPagerPage {
        t: Texts::new(&bundles, Lang::En),
        label: label.to_owned(),
        previous: previous.to_owned(),
        next: next.to_owned(),
        current: 2,
    }
    .render()
    .unwrap()
}

#[test]
fn components_named_pager_is_the_page_pager_with_the_label_of_the_caller() {
    let html = named("Pages of the section Values", "/p1", "/p3");
    let expected = "<nav class=\"pager\" aria-label=\"Pages of the section Values\"><a class=\"target-link\" href=\"/p1\">Previous page</a><span class=\"pager-current\" aria-current=\"page\">Page 2</span><a class=\"target-link\" href=\"/p3\">Next page</a></nav>";
    assert_eq!(html.trim(), expected);
}

#[test]
fn components_named_pager_omits_a_link_without_an_address() {
    let html = named("L", "", "/p3");
    assert_eq!(html.matches("<a ").count(), 1, "{html}");
    assert!(!html.contains("Previous page"), "{html}");
}

#[test]
fn components_diff_icons_have_the_fixed_tag_and_the_shapes_of_the_design() {
    let html = DiffIconsPage.render().unwrap();
    for (name, _) in DIFF_ICONS {
        assert!(html.contains(&icon(name)), "icon {name}");
    }
    assert_eq!(html.matches("<svg").count(), 4);
    assert_eq!(html.matches("</svg>").count(), 4);
    assert_fixed_svg_tags(&html);
    for forbidden in [" style=", " on", "<title", "<text", "<script"] {
        assert!(!html.contains(forbidden), "found {forbidden}");
    }
}

#[test]
fn components_diff_icons_are_the_four_rows_of_the_icon_table() {
    for (name, _) in DIFF_ICONS {
        assert!(
            DESIGN.contains(&format!("| `{name}` |")),
            "no row for {name}"
        );
    }
}

#[derive(Template)]
#[template(path = "test/danger-action.html")]
struct ActionDangerPage<'a> {
    t: Texts<'a>,
    summary: String,
    warning: String,
    button: String,
    action: String,
    csrf: String,
}

/// Render the delete disclosure of a recipe in `lang` with the given action and token.
fn action_with(lang: Lang, action: &str, csrf: &str) -> String {
    let bundles = Bundles::load().unwrap();
    ActionDangerPage {
        t: Texts::new(&bundles, lang),
        summary: text(lang, "delete-recipe-summary"),
        warning: text(lang, "delete-recipe-notice"),
        button: "Delete recipe R-100".to_owned(),
        action: action.to_owned(),
        csrf: csrf.to_owned(),
    }
    .render()
    .unwrap()
}

#[test]
fn components_danger_action_is_the_form_without_a_row_version_in_both_languages() {
    for lang in [Lang::En, Lang::Pl] {
        let expected = format!(
            "<details class=\"danger\"><summary>{}{}<span>{}</span></summary><form method=\"post\" action=\"/recipes/R-100/delete\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\"><input type=\"hidden\" name=\"csrf_token\" value=\"TOKEN\"><div id=\"danger-notice\"><div class=\"alert alert-warning\">{}<span>{}</span></div></div><button type=\"submit\" class=\"btn-danger\" aria-describedby=\"danger-notice\">Delete recipe R-100</button><span class=\"htmx-indicator\">{}</span></form></details>",
            icon("tree-collapsed"),
            icon("tree-expanded"),
            text(lang, "delete-recipe-summary"),
            icon("alert-warning"),
            text(lang, "delete-recipe-notice"),
            text(lang, "form-in-progress"),
        );
        let html = action_with(lang, "/recipes/R-100/delete", "TOKEN");
        assert_eq!(html.trim(), expected, "{lang}");
        assert!(!html.contains("row_version"), "{html}");
    }
}

#[test]
fn components_danger_action_holds_the_notice_once_without_role_and_four_literal_hx_attributes() {
    let html = action_with(Lang::En, "/a\"><script>x</script>", "\"><b>");
    assert_eq!(html.matches("id=\"danger-notice\"").count(), 1, "{html}");
    assert_eq!(
        html.matches("aria-describedby=\"danger-notice\"").count(),
        1
    );
    assert!(!html.contains("role="), "{html}");
    assert!(!html.contains("hx-disabled-elt"), "{html}");
    assert_eq!(html.matches(" hx-").count(), 4, "{html}");
    for raw in ["<script", "<b>", " style=", " onclick=", " onload="] {
        assert!(!html.contains(raw), "found {raw}: {html}");
    }
}
