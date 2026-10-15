//! Tests of the components of the plant model pages: the heading, the tree, the archived badge,
//! and the danger disclosure (design D7 and D10, `design-system.md` 10.5, 10.8, 10.9).
//!
//! A test template below `templates/test/` calls one macro and takes every text from a struct
//! field. The expected markup is written here, in the form of the design.

use askama::Template;

use super::{Bundles, Lang, Texts, assert_fixed_svg_tags, icon, text};
use crate::layout::{Layout, Shape};

/// The group id that a tree test passes. The real id is the UUID of a unit or a type.
const UUID: &str = "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10";
/// The page of the unit `U1.2`.
const NODE_HREF: &str = "/machines/M7/units/U1.2";
/// The form of a new sub-unit of `U1.2`.
const ADD_HREF: &str = "/machines/M7/units/new?parent_path=U1.2";
/// The page of the leaf `U1.2.1`.
const LEAF_HREF: &str = "/machines/M7/units/U1.2.1";

// --- The heading -------------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "test/heading.html")]
struct HeadingPage<'a> {
    layout: Layout<'a>,
    text: String,
}

/// Render the heading `text` for the content answer (`content`) or for a full page.
fn heading(content: bool, text: &str) -> String {
    let bundles = Bundles::load().unwrap();
    let texts = Texts::new(&bundles, Lang::En);
    let mut layout = Layout::bare(texts, Lang::En, "t".to_owned());
    if content {
        layout.shape = Shape::Content;
    }
    HeadingPage {
        layout,
        text: text.to_owned(),
    }
    .render()
    .unwrap()
}

#[test]
fn components_heading_of_a_full_page_is_a_plain_h1() {
    assert_eq!(heading(false, "Title").trim(), "<h1>Title</h1>");
}

#[test]
fn components_heading_of_the_content_answer_takes_the_focus() {
    assert_eq!(
        heading(true, "Title").trim(),
        "<h1 tabindex=\"-1\" autofocus>Title</h1>"
    );
}

#[test]
fn components_heading_escapes_its_text() {
    for content in [false, true] {
        let html = heading(content, "<script>x</script>");
        assert!(!html.contains("<script"), "{html}");
        assert!(html.contains("&#60;script&#62;"), "{html}");
    }
}

// --- The archived badge -----------------------------------------------------------------------

#[derive(Template)]
#[template(path = "test/archived-badge.html")]
struct ArchivedBadgePage {
    word: String,
    large: bool,
}

#[test]
fn components_archived_badge_has_the_retired_tokens_icon_and_word_in_both_languages() {
    for lang in [Lang::Pl, Lang::En] {
        let word = text(lang, "status-archived");
        for (large, class) in [
            (false, "badge badge-retired"),
            (true, "badge badge-large badge-retired"),
        ] {
            let html = ArchivedBadgePage {
                word: word.clone(),
                large,
            }
            .render()
            .unwrap();
            let expected = format!(
                "<span class=\"{class}\">{}<span>{word}</span></span>",
                icon("status-retired")
            );
            assert_eq!(html.trim(), expected, "{lang} large {large}");
        }
    }
}

#[test]
fn components_archived_badge_words_are_the_bundle_words() {
    assert_eq!(text(Lang::En, "status-archived"), "Archived");
    assert_eq!(text(Lang::Pl, "status-archived"), "Zarchiwizowana");
}

// --- The unit tree ----------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "test/unit-tree.html")]
struct UnitTreePage<'a> {
    t: Texts<'a>,
    id: &'static str,
    path: String,
    name: String,
    href: String,
    add_href: String,
    expanded: bool,
    focus: bool,
    archived: bool,
    leaf_path: String,
    leaf_name: String,
    leaf_href: String,
    leaf_add_href: String,
    leaf_focus: bool,
    leaf_archived: bool,
}

#[derive(Template)]
#[template(path = "test/unit-leaf.html")]
struct UnitLeafPage<'a> {
    t: Texts<'a>,
    path: String,
    name: String,
    href: String,
    add_href: String,
    focus: bool,
    archived: bool,
}

/// What a unit node test changes: the flags and the addresses of the node `U1.2`.
#[derive(Clone)]
struct Node {
    href: String,
    add_href: String,
    expanded: bool,
    focus: bool,
    archived: bool,
}

impl Node {
    /// A live node with a link and an add link, expanded, without focus.
    fn live() -> Self {
        Self {
            href: NODE_HREF.to_owned(),
            add_href: ADD_HREF.to_owned(),
            expanded: true,
            focus: false,
            archived: false,
        }
    }
}

/// Render the node `U1.2` named `Press` with the child leaf `U1.2.1`.
fn unit_tree(lang: Lang, node: &Node) -> String {
    let bundles = Bundles::load().unwrap();
    UnitTreePage {
        t: Texts::new(&bundles, lang),
        id: UUID,
        path: "U1.2".to_owned(),
        name: "Press".to_owned(),
        href: node.href.clone(),
        add_href: node.add_href.clone(),
        expanded: node.expanded,
        focus: node.focus,
        archived: node.archived,
        leaf_path: "U1.2.1".to_owned(),
        leaf_name: "Feeder".to_owned(),
        leaf_href: LEAF_HREF.to_owned(),
        leaf_add_href: String::new(),
        leaf_focus: false,
        leaf_archived: false,
    }
    .render()
    .unwrap()
}

/// Render a leaf `U1.2.1` named `Feeder`.
fn unit_leaf(lang: Lang, href: &str, add_href: &str, focus: bool, archived: bool) -> String {
    let bundles = Bundles::load().unwrap();
    UnitLeafPage {
        t: Texts::new(&bundles, lang),
        path: "U1.2.1".to_owned(),
        name: "Feeder".to_owned(),
        href: href.to_owned(),
        add_href: add_href.to_owned(),
        focus,
        archived,
    }
    .render()
    .unwrap()
}

/// The markup of the link "Add a sub-unit" with its literal `hx-*` attributes.
fn add_link(href: &str, label: &str, word: &str) -> String {
    format!(
        "<a class=\"target-link\" href=\"{href}\" hx-boost=\"true\" hx-target=\"#unit-form\" hx-swap=\"innerHTML\" hx-push-url=\"false\" aria-label=\"{label}\">{word}</a>"
    )
}

/// The markup of a node row up to the start of its group, without the optional parts.
fn node_row(name_markup: &str, tail: &str) -> String {
    format!(
        "<li><div class=\"tree-row\"><button type=\"button\" class=\"tree-node\" aria-expanded=\"true\" aria-controls=\"tree-group-{UUID}\">{}{}<span class=\"tree-path\">U1.2</span></button>{name_markup}{tail}</div><ul class=\"tree-group\" id=\"tree-group-{UUID}\">",
        icon("tree-collapsed"),
        icon("tree-expanded"),
    )
}

#[test]
fn components_unit_node_is_one_row_with_button_name_link_and_add_link() {
    for (lang, label, word) in [
        (Lang::En, "Add a sub-unit to U1.2", "Add a sub-unit"),
        (Lang::Pl, "Dodaj podjednostkę do U1.2", "Dodaj podjednostkę"),
    ] {
        let html = unit_tree(lang, &Node::live());
        let link = format!("<a class=\"target-link\" href=\"{NODE_HREF}\">Press</a>");
        let expected = node_row(&link, &add_link(ADD_HREF, label, word));
        assert!(html.contains(&expected), "{lang}: {html}");
        assert!(html.contains("</ul></li>"));
    }
}

#[test]
fn components_unit_node_without_the_link_shows_the_name_as_plain_text() {
    let node = Node {
        href: String::new(),
        ..Node::live()
    };
    let html = unit_tree(Lang::En, &node);
    assert!(html.contains("</button><span>Press</span>"), "{html}");
    assert!(!html.contains(&format!("href=\"{NODE_HREF}\"")), "{html}");
}

#[test]
fn components_unit_node_without_the_add_link_omits_it() {
    let node = Node {
        add_href: String::new(),
        ..Node::live()
    };
    let html = unit_tree(Lang::En, &node);
    assert!(!html.contains("hx-boost"), "{html}");
    assert!(!html.contains("Add a sub-unit"), "{html}");
    assert!(
        html.contains(&format!("href=\"{NODE_HREF}\">Press</a></div>")),
        "{html}"
    );
}

#[test]
fn components_unit_node_with_the_focus_flag_puts_autofocus_on_its_link() {
    let node = Node {
        focus: true,
        ..Node::live()
    };
    let html = unit_tree(Lang::En, &node);
    assert!(
        html.contains(&format!("href=\"{NODE_HREF}\" autofocus>Press</a>")),
        "{html}"
    );
    assert_eq!(html.matches("autofocus").count(), 1);
    assert_eq!(
        unit_tree(Lang::En, &Node::live())
            .matches("autofocus")
            .count(),
        0
    );
}

#[test]
fn components_unit_node_of_an_archived_unit_shows_the_archived_badge_after_the_name() {
    for (lang, word) in [(Lang::En, "Archived"), (Lang::Pl, "Zarchiwizowana")] {
        let node = Node {
            archived: true,
            add_href: String::new(),
            ..Node::live()
        };
        let html = unit_tree(lang, &node);
        let badge = format!(
            "<span class=\"badge badge-retired\">{}<span>{word}</span></span>",
            icon("status-retired")
        );
        let expected = format!("href=\"{NODE_HREF}\">Press</a>{badge}</div>");
        assert!(html.contains(&expected), "{lang}: {html}");
    }
    assert!(!unit_tree(Lang::En, &Node::live()).contains("badge"));
}

#[test]
fn components_unit_node_collapsed_hides_its_group_and_keeps_the_ids_equal() {
    let node = Node {
        expanded: false,
        ..Node::live()
    };
    let html = unit_tree(Lang::En, &node);
    assert!(html.contains("aria-expanded=\"false\""), "{html}");
    assert!(
        html.contains(&format!(
            "<ul class=\"tree-group\" id=\"tree-group-{UUID}\" hidden>"
        )),
        "{html}"
    );
    assert!(html.contains(&format!("aria-controls=\"tree-group-{UUID}\"")));
}

#[test]
fn components_unit_leaf_is_a_row_with_the_path_and_no_button() {
    let html = unit_leaf(Lang::En, LEAF_HREF, "", false, false);
    let expected = format!(
        "<li class=\"tree-leaf\"><div class=\"tree-row\"><span class=\"tree-path\">U1.2.1</span><a class=\"target-link\" href=\"{LEAF_HREF}\">Feeder</a></div></li>"
    );
    assert_eq!(html.trim(), format!("<ul class=\"tree\">{expected}</ul>"));
    assert!(!html.contains("<button"));
    assert!(!html.contains("aria-expanded"));
}

#[test]
fn components_unit_leaf_can_hold_the_add_link_the_focus_and_the_badge() {
    let html = unit_leaf(Lang::En, LEAF_HREF, ADD_HREF, true, true);
    assert!(
        html.contains(&format!("href=\"{LEAF_HREF}\" autofocus>Feeder</a>")),
        "{html}"
    );
    assert!(html.contains("<span>Archived</span>"), "{html}");
    let label = "Add a sub-unit to U1.2.1";
    assert!(
        html.contains(&add_link(ADD_HREF, label, "Add a sub-unit")),
        "{html}"
    );
    let plain = unit_leaf(Lang::En, "", "", false, false);
    assert!(plain.contains("<span>Feeder</span></div>"), "{plain}");
}

/// Return the value of every attribute whose name starts with `hx-`.
fn hx_values(html: &str) -> Vec<(String, String)> {
    html.split(" hx-")
        .skip(1)
        .filter_map(|part| {
            let (name, rest) = part.split_once("=\"")?;
            let (value, _tail) = rest.split_once('"')?;
            Some((format!("hx-{name}"), value.to_owned()))
        })
        .collect()
}

#[test]
fn components_unit_node_hx_attributes_are_literals_and_user_text_is_escaped() {
    let payload = "<script>alert(1)</script>";
    let bundles = Bundles::load().unwrap();
    let html = UnitTreePage {
        t: Texts::new(&bundles, Lang::En),
        id: UUID,
        path: payload.to_owned(),
        name: payload.to_owned(),
        href: format!("/x?q={payload}"),
        add_href: format!("/y?q={payload}"),
        expanded: true,
        focus: false,
        archived: false,
        leaf_path: payload.to_owned(),
        leaf_name: payload.to_owned(),
        leaf_href: payload.to_owned(),
        leaf_add_href: payload.to_owned(),
        leaf_focus: false,
        leaf_archived: false,
    }
    .render()
    .unwrap();
    assert!(!html.contains("<script"), "{html}");
    let literals = [
        ("hx-boost", "true"),
        ("hx-target", "#unit-form"),
        ("hx-swap", "innerHTML"),
        ("hx-push-url", "false"),
    ];
    let found = hx_values(&html);
    assert_eq!(found.len(), 8, "{found:?}");
    for (name, value) in found {
        assert!(
            literals.contains(&(name.as_str(), value.as_str())),
            "{name}={value}"
        );
    }
}

#[test]
fn components_unit_node_svg_tags_are_fixed() {
    let node = Node {
        archived: true,
        ..Node::live()
    };
    assert_fixed_svg_tags(&unit_tree(Lang::En, &node));
}

// --- The type tree ----------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "test/type-tree.html")]
struct TypeTreePage {
    id: &'static str,
    index: u16,
    name: String,
    type_text: String,
    expanded: bool,
    leaf_name: String,
    leaf_type: String,
}

/// Render a type node `ST_Zone@2` with the child member `temp_max` of the type `REAL`.
fn type_tree(index: u16, type_text: &str, expanded: bool) -> String {
    TypeTreePage {
        id: UUID,
        index,
        name: "zone".to_owned(),
        type_text: type_text.to_owned(),
        expanded,
        leaf_name: "temp_max".to_owned(),
        leaf_type: "REAL".to_owned(),
    }
    .render()
    .unwrap()
}

#[test]
fn components_type_node_shows_name_and_type_in_mono_and_numbers_its_group() {
    let html = type_tree(3, "ST_Zone@2", false);
    let expected = format!(
        "<li><div class=\"tree-row\"><button type=\"button\" class=\"tree-node\" aria-expanded=\"false\" aria-controls=\"tree-group-{UUID}-3\">{}{}<span class=\"data\">zone</span><span class=\"data tree-type\">ST_Zone@2</span></button></div><ul class=\"tree-group\" id=\"tree-group-{UUID}-3\" hidden>",
        icon("tree-collapsed"),
        icon("tree-expanded"),
    );
    assert!(html.contains(&expected), "{html}");
    assert!(html.contains("</ul></li>"));
}

#[test]
fn components_type_node_group_ids_differ_for_one_type_at_two_places() {
    let first = type_tree(1, "ST_Zone@2", true);
    let second = type_tree(7, "ST_Zone@2", true);
    assert!(first.contains(&format!("id=\"tree-group-{UUID}-1\"")));
    assert!(second.contains(&format!("id=\"tree-group-{UUID}-7\"")));
    assert!(!first.contains(&format!("tree-group-{UUID}-7")));
    assert!(!first.contains(" hidden"));
}

#[test]
fn components_type_node_without_a_type_text_shows_the_name_only() {
    let html = type_tree(0, "", true);
    let button = html.split("</button>").next().unwrap();
    assert_eq!(button.matches("class=\"data\"").count(), 1, "{html}");
    assert!(
        html.contains("<span class=\"data\">zone</span></button>"),
        "{html}"
    );
}

#[test]
fn components_type_leaf_shows_name_and_type_in_mono_without_a_button() {
    let html = type_tree(0, "ST_Zone@2", true);
    let leaf = "<li class=\"tree-leaf\"><span class=\"data\">temp_max</span><span class=\"data tree-type\">REAL</span></li>";
    assert!(html.contains(leaf), "{html}");
    assert_eq!(html.matches("<button").count(), 1);
    assert_eq!(html.matches("aria-expanded").count(), 1);
}

#[test]
fn components_type_tree_uses_the_disclosure_pattern_and_fixed_svg_tags() {
    let html = type_tree(0, "ST_Zone@2", true);
    for forbidden in ["role=", " style=", " on", "hx-"] {
        assert!(!html.contains(forbidden), "found {forbidden}");
    }
    assert!(html.contains("<ul class=\"tree\">"));
    assert_fixed_svg_tags(&html);
}

#[test]
fn components_type_tree_escapes_name_and_type() {
    let payload = "<script>alert(1)</script>";
    let html = TypeTreePage {
        id: UUID,
        index: 0,
        name: payload.to_owned(),
        type_text: payload.to_owned(),
        expanded: true,
        leaf_name: payload.to_owned(),
        leaf_type: payload.to_owned(),
    }
    .render()
    .unwrap();
    assert!(!html.contains("<script"), "{html}");
    assert_eq!(html.matches("&#60;script&#62;").count(), 4);
}

// --- The danger disclosure --------------------------------------------------------------------

#[derive(Template)]
#[template(path = "test/danger.html")]
struct DangerPage<'a> {
    t: Texts<'a>,
    summary: String,
    warning: String,
    button: String,
    action: String,
    csrf: String,
}

/// Render the archive form of the unit `U1.2` in `lang`.
fn danger(lang: Lang, action: &str) -> String {
    let bundles = Bundles::load().unwrap();
    let texts = Texts::new(&bundles, lang);
    let page = DangerPage {
        t: texts,
        summary: texts.get("unit-archive-summary"),
        warning: texts.get("unit-archive-warning"),
        button: texts.with_arg("unit-archive-submit", "path", "U1.2"),
        action: action.to_owned(),
        csrf: "6666".to_owned(),
    };
    // The macro takes the texts but writes none of them itself, so the test reads the field here.
    assert_eq!(page.t.get("unit-archive-summary"), page.summary);
    page.render().unwrap()
}

#[test]
fn components_danger_is_a_details_with_a_summary_a_warning_and_a_post_form() {
    for lang in [Lang::En, Lang::Pl] {
        let t = |key: &str| text(lang, key);
        let button = Texts::new(&Bundles::load().unwrap(), lang).with_arg(
            "unit-archive-submit",
            "path",
            "U1.2",
        );
        let expected = format!(
            "<details class=\"danger\"><summary>{}{}<span>{}</span></summary><form method=\"post\" action=\"/machines/M7/units/U1.2/archive\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\"><input type=\"hidden\" name=\"csrf_token\" value=\"6666\"><div class=\"alert alert-warning\">{}<span>{}</span></div><button type=\"submit\" class=\"btn-danger\">{button}</button></form></details>",
            icon("tree-collapsed"),
            icon("tree-expanded"),
            t("unit-archive-summary"),
            icon("alert-warning"),
            t("unit-archive-warning"),
        );
        let html = danger(lang, "/machines/M7/units/U1.2/archive");
        assert_eq!(html.trim(), expected, "{lang}");
    }
}

#[test]
fn components_danger_button_names_the_object_and_the_form_never_sends_get() {
    let html = danger(Lang::En, "/machines/M7/archive");
    assert!(html.contains(">Archive unit U1.2</button>"), "{html}");
    assert!(html.contains("method=\"post\""));
    assert!(!html.contains("method=\"get\""));
    assert!(!html.contains(" open"), "the disclosure starts closed");
}

#[test]
fn components_danger_hx_attributes_are_literals_and_the_action_is_escaped() {
    let html = danger(Lang::En, "/x\"><script>alert(1)</script>");
    assert!(!html.contains("<script"), "{html}");
    let found = hx_values(&html);
    let expected = [
        ("hx-boost", "true"),
        ("hx-target", "#content"),
        ("hx-push-url", "true"),
        ("hx-sync", "closest form:drop"),
    ];
    assert_eq!(found.len(), expected.len(), "{found:?}");
    for ((name, value), (want_name, want_value)) in found.iter().zip(expected) {
        assert_eq!((name.as_str(), value.as_str()), (want_name, want_value));
    }
}

#[test]
fn components_danger_svg_tags_are_fixed_and_it_has_no_alert_role() {
    let html = danger(Lang::En, "/machines/M7/archive");
    assert_fixed_svg_tags(&html);
    assert_eq!(html.matches("<svg").count(), 3);
    assert!(!html.contains("role="));
}
