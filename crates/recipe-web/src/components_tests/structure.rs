//! Tests of the field, the table frame, and the tree (design D6, `design-system.md` 10.4, 10.5, 10.8).

use askama::Template;

use super::{Bundles, Lang, Texts, icon, text};

/// The group id that a tree test passes. The real id is a fixed prefix and the UUID of a unit.
const UUID: &str = "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10";

#[derive(Template)]
#[template(path = "test/field.html")]
struct FieldPage<'a> {
    t: Texts<'a>,
    name: &'static str,
    label: String,
    hint: String,
    kind: &'static str,
    required: bool,
    data: bool,
    maxlength: u16,
    autocomplete: &'static str,
}

struct Row {
    a: String,
    b: String,
}

#[derive(Template)]
#[template(path = "test/table.html")]
struct TablePage {
    caption_id: &'static str,
    caption: String,
    head_a: String,
    head_b: String,
    empty_text: String,
    rows: Vec<Row>,
}

#[derive(Template)]
#[template(path = "test/tree.html")]
struct TreePage {
    group_id: &'static str,
    path: String,
    name: String,
    expanded: bool,
    leaf_path: String,
    leaf_name: String,
}

/// Render the field `current_password` in `lang` with the given switches.
fn field(lang: Lang, required: bool, data: bool, hint: &str) -> String {
    let bundles = Bundles::load().unwrap();
    FieldPage {
        t: Texts::new(&bundles, lang),
        name: "current_password",
        label: text(lang, "field-current-password"),
        hint: hint.to_owned(),
        kind: "password",
        required,
        data,
        maxlength: 128,
        autocomplete: "current-password",
    }
    .render()
    .unwrap()
}

/// Return the tokens of the opening tag `<input ...>` of `html`.
fn input_tokens(html: &str) -> Vec<String> {
    let start = html.find("<input").unwrap();
    let tag = html.get(start..).unwrap().split('>').next().unwrap();
    tag.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn components_field_label_for_input_hint_and_required_word() {
    for lang in [Lang::Pl, Lang::En] {
        let word = text(lang, "form-required");
        let label = text(lang, "field-current-password");
        let html = field(lang, true, false, "Hint text");
        let expected = format!(
            "<div class=\"field\"><label for=\"current_password\">{label} <span class=\"field-required\">{word}</span></label><span class=\"field-hint\" id=\"current_password-hint\">Hint text</span><input id=\"current_password\" name=\"current_password\" type=\"password\" maxlength=\"128\" autocomplete=\"current-password\" aria-required=\"true\" aria-describedby=\"current_password-hint\"></div>"
        );
        assert_eq!(html.trim(), expected, "{lang}");
    }
}

#[test]
fn components_field_required_word_follows_the_language() {
    assert!(field(Lang::Pl, true, false, "h").contains(">wymagane<"));
    assert!(field(Lang::En, true, false, "h").contains(">required<"));
}

#[test]
fn components_field_optional_has_no_required_word_and_no_aria_required() {
    let html = field(Lang::En, false, false, "Hint text");
    assert!(!html.contains("field-required"));
    assert!(!html.contains("aria-required"));
    assert!(html.contains("<label for=\"current_password\">Current password</label>"));
}

#[test]
fn components_field_data_input_has_the_data_class() {
    let tokens = input_tokens(&field(Lang::En, false, true, "h"));
    assert!(tokens.iter().any(|token| token == "class=\"data\""));
    let plain = input_tokens(&field(Lang::En, false, false, "h"));
    assert!(!plain.iter().any(|token| token.starts_with("class=")));
}

#[test]
fn components_field_data_input_turns_off_autocapitalize_and_spellcheck() {
    let tokens = input_tokens(&field(Lang::En, false, true, "h"));
    assert!(
        tokens
            .iter()
            .any(|token| token == "autocapitalize=\"none\"")
    );
    assert!(tokens.iter().any(|token| token == "spellcheck=\"false\""));
    let plain = input_tokens(&field(Lang::En, false, false, "h"));
    assert!(
        !plain
            .iter()
            .any(|token| token.starts_with("autocapitalize"))
    );
    assert!(!plain.iter().any(|token| token.starts_with("spellcheck")));
}

#[test]
fn components_field_input_has_none_of_the_forbidden_attributes() {
    for required in [true, false] {
        let tokens = input_tokens(&field(Lang::En, required, true, "h"));
        for token in &tokens {
            let name = token.split('=').next().unwrap();
            for forbidden in ["required", "pattern", "minlength", "placeholder"] {
                assert_ne!(name, forbidden, "forbidden attribute {token}");
            }
        }
        assert!(tokens.iter().any(|token| token == "maxlength=\"128\""));
    }
}

#[test]
fn components_field_without_a_hint_has_no_hint_and_no_describedby() {
    let html = field(Lang::En, true, false, "");
    assert!(!html.contains("field-hint"));
    assert!(!html.contains("aria-describedby"));
}

#[test]
fn components_field_hint_is_escaped() {
    let html = field(Lang::En, false, false, "<script>x</script>");
    assert!(!html.contains("<script"));
    assert!(html.contains("&#60;script&#62;"));
}

/// Render the table with the given rows.
fn table(rows: Vec<Row>) -> String {
    TablePage {
        caption_id: "users-caption",
        caption: text(Lang::En, "recipes-title"),
        head_a: text(Lang::En, "field-login"),
        head_b: text(Lang::En, "field-role"),
        empty_text: text(Lang::En, "recipes-empty"),
        rows,
    }
    .render()
    .unwrap()
}

#[test]
fn components_table_frame_is_a_labelled_scrollable_region() {
    let html = table(vec![Row {
        a: "mhoffman".to_owned(),
        b: "Operator".to_owned(),
    }]);
    let start = "<div class=\"table-wrap\" role=\"region\" tabindex=\"0\" aria-labelledby=\"users-caption\"><table><caption id=\"users-caption\">Recipes</caption>";
    assert!(html.trim().starts_with(start), "{html}");
    assert!(html.trim().ends_with("</table></div>"));
    assert!(html.contains("<th scope=\"col\">Login</th><th scope=\"col\">Role</th>"));
    assert!(html.contains("<tbody><tr><td>mhoffman</td><td>Operator</td></tr></tbody>"));
    assert!(!html.contains("class=\"empty\""));
}

#[test]
fn components_table_every_th_has_scope_col() {
    let html = table(Vec::new());
    assert_eq!(html.matches("<th ").count(), 2);
    assert_eq!(html.matches("<th scope=\"col\">").count(), 2);
}

#[test]
fn components_table_without_rows_shows_the_empty_state() {
    let html = table(Vec::new());
    assert!(html.contains(
        "<tbody><tr><td colspan=\"2\"><p class=\"empty\">No recipe to show.</p></td></tr></tbody>"
    ));
}

#[test]
fn components_table_cell_text_is_escaped() {
    let html = table(vec![Row {
        a: "<script>x</script>".to_owned(),
        b: "b".to_owned(),
    }]);
    assert!(!html.contains("<script"));
}

/// Render the tree with one node and one leaf.
fn tree(expanded: bool, path: &str, name: &str) -> String {
    TreePage {
        group_id: UUID,
        path: path.to_owned(),
        name: name.to_owned(),
        expanded,
        leaf_path: "L1.M2.U2".to_owned(),
        leaf_name: "Leaf".to_owned(),
    }
    .render()
    .unwrap()
}

/// The tag of the group that the node controls.
fn group_tag(expanded: bool) -> String {
    let hidden = if expanded { "" } else { " hidden" };
    format!("<ul class=\"tree-group\" id=\"tree-group-{UUID}\"{hidden}>")
}

#[test]
fn components_tree_node_with_children_is_a_disclosure_button() {
    for (expanded, value) in [(false, "false"), (true, "true")] {
        let html = tree(expanded, "L1.M2.U1", "Unit");
        let button = format!(
            "<button type=\"button\" class=\"tree-node\" aria-expanded=\"{value}\" aria-controls=\"tree-group-{UUID}\">"
        );
        let expected = format!(
            "<li>{button}{}{}<span class=\"tree-path\">L1.M2.U1</span><span>Unit</span></button>{}",
            icon("tree-collapsed"),
            icon("tree-expanded"),
            group_tag(expanded)
        );
        assert!(html.contains(&expected), "expanded {expanded}: {html}");
        assert!(html.contains("</ul></li>"));
    }
}

#[test]
fn components_tree_group_id_equals_aria_controls_and_hidden_follows_state() {
    let collapsed = tree(false, "p", "n");
    assert!(collapsed.contains(&group_tag(false)));
    let expanded = tree(true, "p", "n");
    assert!(expanded.contains(&group_tag(true)));
    assert!(!expanded.contains(" hidden"));
    assert_eq!(collapsed.matches(" hidden").count(), 1);
}

#[test]
fn components_tree_leaf_has_no_button_and_no_aria_expanded() {
    let html = tree(true, "p", "n");
    let leaf =
        "<li class=\"tree-leaf\"><span class=\"tree-path\">L1.M2.U2</span><span>Leaf</span></li>";
    assert!(html.contains(leaf), "{html}");
    assert_eq!(html.matches("<button").count(), 1);
    assert_eq!(html.matches("aria-expanded").count(), 1);
    assert_eq!(html.matches("aria-controls").count(), 1);
}

#[test]
fn components_tree_uses_the_disclosure_pattern_not_the_tree_roles() {
    let html = tree(false, "p", "n");
    for forbidden in [
        "role=\"tree\"",
        "role=\"treeitem\"",
        "role=",
        " style=",
        " on",
    ] {
        assert!(!html.contains(forbidden), "found {forbidden}");
    }
    assert!(html.contains("<ul class=\"tree\">"));
}

#[test]
fn components_tree_escapes_the_unit_path_and_the_name() {
    let payload = "<script>alert(1)</script>";
    let html = tree(false, payload, payload);
    assert!(!html.contains("<script"));
    assert_eq!(html.matches("&#60;script&#62;").count(), 2);
    assert!(!html.contains(&format!("{payload}</span>")));
}

#[test]
fn components_tree_svg_tags_are_fixed() {
    let html = tree(true, "p", "n");
    assert_eq!(html.matches("<svg").count(), 2);
    super::assert_fixed_svg_tags(&html);
}
