//! Tests of the unit tree in its three places (task 7.4, specs "Unit tree with the disclosure
//! pattern" and "State shown by color, icon, and word"): `GET /machines/{code}/units`, the
//! machine page, and the page of an archived machine.

use axum::http::StatusCode;
use uuid::Uuid;

use super::test_world::seed_machine_with_archived_unit;
use crate::i18n::Lang;
use crate::pages::test_helpers::{assert_clean_html, assert_no_user_text_in_hx_attributes};
use crate::test_support::plant::{
    ENGLISH, HTMX, M12_ID, World, cookie_of, main_of, text_with, world,
};
use crate::test_support::{icon_tag, text};

/// The number of live units of `M7` in `seed_plant`.
const M7_UNITS: usize = 64;
/// The live units of `M7` with depth 1 to 4. A user with `UnitManage` gets an add link on each.
const M7_ADD_LINKS: usize = 42;
/// The prefix of the address of an add link.
const ADD_PREFIX: &str = "/machines/M7/units/new?parent_path=";

/// Return the part of the machine page that holds the unit tree.
fn tree_part(body: &str) -> &str {
    let (_before, rest) = main_of(body).split_once("<div id=\"unit-tree\">").unwrap();
    let ends = [
        "<p class=\"actions\">",
        "<div id=\"unit-form\">",
        "<p><a class=\"target-link\" href=\"/machines\">",
    ];
    let end = ends
        .iter()
        .filter_map(|marker| rest.find(marker))
        .min()
        .unwrap_or(rest.len());
    rest.get(..end).unwrap()
}

/// Return the content of the row of the node `path`, from its path to the end of the row.
fn row_of<'a>(tree: &'a str, path: &str) -> &'a str {
    let marker = format!("<span class=\"tree-path\">{path}</span>");
    let (_before, rest) = tree.split_once(&marker).unwrap();
    rest.split("</div>").next().unwrap()
}

/// Return the value of every `aria-controls` attribute of a tree.
fn controls(tree: &str) -> Vec<&str> {
    tree.split("aria-controls=\"")
        .skip(1)
        .filter_map(|part| part.split('"').next())
        .collect()
}

/// Send `GET /machines/M7/units` as an HTMX request with another target than `main`.
async fn fragment_of(world: &World, lang: Lang) -> String {
    let headers = [cookie_of(lang), HTMX];
    let reply = world
        .get(&world.operator, "/machines/M7/units", &headers)
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body
}

// --- GET /machines/{code}/units: the fragment ---------------------------------------------------

#[tokio::test]
async fn machines_tree_fragment_holds_64_links_and_hides_the_groups_of_depth_3_and_4() {
    let world = world();
    let body = fragment_of(&world, Lang::En).await;
    assert!(body.starts_with("<ul class=\"tree\">"), "{body}");
    assert_eq!(
        body.matches("href=\"/machines/M7/units/U").count(),
        M7_UNITS
    );
    assert_eq!(
        body.matches(" hidden>").count(),
        6,
        "the groups of 3 depth-3 and 3 depth-4 nodes"
    );
    assert_eq!(body.matches("aria-expanded=\"true\"").count(), 5);
    assert_eq!(body.matches("aria-expanded=\"false\"").count(), 6);
    for frame in ["<html", "<h1", "<main", "class=\"topbar\""] {
        assert!(!body.contains(frame), "{frame}");
    }
    assert_clean_html(&body);
}

#[tokio::test]
async fn machines_tree_each_aria_controls_names_a_group_with_the_prefix_and_a_uuid() {
    let world = world();
    let body = fragment_of(&world, Lang::En).await;
    let names = controls(&body);
    assert_eq!(names.len(), 11, "the 11 nodes with children");
    for name in &names {
        let uuid = name.strip_prefix("tree-group-").unwrap();
        assert_eq!(uuid.len(), 36, "{name}");
        assert!(Uuid::parse_str(uuid).is_ok(), "{name}");
        assert!(body.contains(&format!("id=\"{name}\"")), "{name}");
    }
    let mut unique = names.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "each group has its own id");
}

#[tokio::test]
async fn machines_tree_fragment_names_each_unit_and_marks_a_leaf_without_a_button() {
    let world = world();
    let body = fragment_of(&world, Lang::En).await;
    assert!(
        row_of(&body, "U1")
            .contains("<a class=\"target-link\" href=\"/machines/M7/units/U1\">Infeed</a>")
    );
    assert!(body.contains("<li class=\"tree-leaf\"><div class=\"tree-row\"><span class=\"tree-path\">U1.2.4.5.6</span>"));
    assert_eq!(body.matches("<button").count(), 11);
    assert!(
        !body.contains("role="),
        "the disclosure pattern needs no tree role"
    );
}

#[tokio::test]
async fn machines_tree_branch_node_has_the_markup_of_the_unit_node_component() {
    let world = world();
    let headers = [ENGLISH, HTMX];
    let reply = world
        .get(&world.engineer, "/machines/M7/units", &headers)
        .await;
    let group = controls(&reply.body)[0];
    let collapsed = format!(
        "{}<path d=\"M9 6l6 6-6 6\"/></svg>",
        icon_tag("tree-collapsed")
    );
    let expanded = format!(
        "{}<path d=\"M6 9l6 6 6-6\"/></svg>",
        icon_tag("tree-expanded")
    );
    let label = text_with(Lang::En, "tree-add-sub-unit-to", "path", "U1");
    let expected = format!(
        "<li><div class=\"tree-row\"><button type=\"button\" class=\"tree-node\" aria-expanded=\"true\" aria-controls=\"{group}\">{collapsed}{expanded}<span class=\"tree-path\">U1</span></button><a class=\"target-link\" href=\"/machines/M7/units/U1\">Infeed</a><a class=\"target-link\" href=\"{ADD_PREFIX}U1\" hx-boost=\"true\" hx-target=\"#unit-form\" hx-swap=\"innerHTML\" hx-push-url=\"false\" aria-label=\"{label}\">Add a sub-unit</a></div><ul class=\"tree-group\" id=\"{group}\">"
    );
    assert!(reply.body.contains(&expected), "{}", reply.body);
    assert_eq!(
        reply.body.matches("</ul></li>").count(),
        11,
        "each group closes"
    );
}

#[tokio::test]
async fn machines_tree_add_links_exist_for_depth_1_to_4_of_a_role_with_unit_manage() {
    let world = world();
    for session in [&world.engineer, &world.admin] {
        let headers = [ENGLISH, HTMX];
        let reply = world.get(session, "/machines/M7/units", &headers).await;
        assert_eq!(reply.body.matches(ADD_PREFIX).count(), M7_ADD_LINKS);
        let level_4 = row_of(&reply.body, "U1.2.4.5");
        assert!(
            level_4.contains(&format!("href=\"{ADD_PREFIX}U1.2.4.5\"")),
            "{level_4}"
        );
        let level_5 = row_of(&reply.body, "U1.2.4.5.6");
        assert!(!level_5.contains("/units/new"), "{level_5}");
    }
}

#[tokio::test]
async fn machines_tree_add_link_has_the_label_with_the_path_and_literal_hx_attributes() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let headers = [cookie_of(lang), HTMX];
        let reply = world
            .get(&world.engineer, "/machines/M7/units", &headers)
            .await;
        let word = text(lang, "tree-add-sub-unit");
        let label = text_with(lang, "tree-add-sub-unit-to", "path", "U1.2");
        let link = format!(
            "<a class=\"target-link\" href=\"{ADD_PREFIX}U1.2\" hx-boost=\"true\" hx-target=\"#unit-form\" hx-swap=\"innerHTML\" hx-push-url=\"false\" aria-label=\"{label}\">{word}</a>"
        );
        assert!(
            row_of(&reply.body, "U1.2").contains(&link),
            "{lang}: {}",
            reply.body
        );
    }
}

#[tokio::test]
async fn machines_tree_of_an_operator_holds_no_add_link() {
    let world = world();
    let body = fragment_of(&world, Lang::En).await;
    assert!(!body.contains("/units/new"), "{body}");
    assert!(!body.contains("hx-"), "{body}");
    let page = world.get_en(&world.operator, "/machines/M7").await;
    assert!(!tree_part(&page.body).contains("/machines/M7/units/new"));
    assert!(!main_of(&page.body).contains("/machines/M7/units/new"));
}

// --- GET /machines/{code}/units: the page -------------------------------------------------------

#[tokio::test]
async fn machines_tree_page_without_htmx_has_the_layout_the_heading_and_the_tree() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.operator, "/machines/M7/units", &[cookie_of(lang)])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let title = text_with(lang, "unit-tree-title", "code", "M7");
        assert!(reply.body.contains(&format!("<h1>{title}</h1>")), "{lang}");
        assert!(reply.body.contains(&format!(
            "<title>{title} · {}</title>",
            text(lang, "app-name")
        )));
        assert!(reply.body.contains("class=\"topbar\""));
        assert_eq!(reply.body.matches("<h1").count(), 1);
        assert_eq!(reply.body.matches("<ul class=\"tree\">").count(), 1);
        let current = format!(
            "<a class=\"nav-link\" href=\"/machines\" aria-current=\"page\">{}</a>",
            text(lang, "nav-machines")
        );
        assert!(reply.body.contains(&current), "{lang}");
        assert_eq!(
            main_of(&reply.body)
                .matches("href=\"/machines/M7/units/U")
                .count(),
            M7_UNITS
        );
    }
}

/// Spec "Places of the unit form", scenario "Add link on the full tree page" (D20, U2).
#[tokio::test]
async fn machines_tree_full_page_holds_the_unit_tree_region_the_empty_unit_form_and_the_add_link() {
    let world = world();
    let reply = world.get_en(&world.engineer, "/machines/M7/units").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let region = main.find("<div id=\"unit-tree\"><ul class=\"tree\">");
    let form = main.find("<div id=\"unit-form\"></div>");
    assert!(region.is_some(), "{main}");
    assert!(form.is_some(), "{main}");
    assert!(region < form, "the tree stands before its form");
    assert_eq!(main.matches("id=\"unit-form\"").count(), 1);
    let link = format!("href=\"{ADD_PREFIX}U1.2\"");
    let at = main.find(&link).unwrap();
    assert!(
        at < form.unwrap(),
        "the add link stands in the tree: {main}"
    );
    assert_eq!(main.matches(ADD_PREFIX).count(), M7_ADD_LINKS);
}

/// The same scenario for an Operator: the regions stand, and no add link points to them.
#[tokio::test]
async fn machines_tree_full_page_of_an_operator_holds_the_regions_and_no_add_link() {
    let world = world();
    let reply = world.get_en(&world.operator, "/machines/M7/units").await;
    let main = main_of(&reply.body);
    assert!(
        main.contains("<div id=\"unit-tree\"><ul class=\"tree\">"),
        "{main}"
    );
    assert!(main.contains("<div id=\"unit-form\"></div>"), "{main}");
    assert!(!main.contains("/units/new"), "{main}");
}

/// The fragment of the same route does not change (D6): the tree and no region around it.
#[tokio::test]
async fn machines_tree_fragment_holds_neither_region() {
    let world = world();
    let headers = [ENGLISH, HTMX];
    let reply = world
        .get(&world.engineer, "/machines/M7/units", &headers)
        .await;
    assert!(!reply.body.contains("id=\"unit-tree\""), "{}", reply.body);
    assert!(!reply.body.contains("id=\"unit-form\""), "{}", reply.body);
}

#[tokio::test]
async fn machines_tree_page_content_answer_has_the_title_the_heading_and_the_field_next() {
    let world = world();
    let reply = world
        .get_content(&world.operator, "/machines/M7/units")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<h1 tabindex=\"-1\" autofocus>Unit tree of machine M7</h1>")
    );
    assert!(
        reply
            .body
            .contains("<title>Unit tree of machine M7 · Recipe editor</title>")
    );
    let next = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/machines/M7/units\" hx-swap-oob=\"true\">";
    assert!(reply.body.contains(next), "{}", reply.body);
    assert!(!reply.body.contains("class=\"topbar\""));
    assert_eq!(reply.body.matches("<ul class=\"tree\">").count(), 1);
}

#[tokio::test]
async fn machines_tree_route_gives_404_for_a_bad_an_unknown_or_an_archived_code() {
    let world = world();
    for code in ["X7", "M99", "M12"] {
        let path = format!("/machines/{code}/units");
        let reply = world.get_en(&world.operator, &path).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{code}");
        assert!(reply.body.contains("E-WEB-005"), "{code}: {}", reply.body);
    }
}

// --- The tree of the machine page ---------------------------------------------------------------

#[tokio::test]
async fn machines_tree_of_the_machine_page_stands_in_the_unit_tree_region() {
    let world = world();
    let page = world.get_en(&world.engineer, "/machines/M7").await;
    let tree = tree_part(&page.body);
    assert!(tree.starts_with("<ul class=\"tree\">"), "{tree}");
    assert_eq!(
        tree.matches("href=\"/machines/M7/units/U").count(),
        M7_UNITS
    );
    assert_eq!(tree.matches(ADD_PREFIX).count(), M7_ADD_LINKS);
    let node = row_of(tree, "U1.2.4.5");
    assert!(
        node.contains(&format!("href=\"{ADD_PREFIX}U1.2.4.5\"")),
        "{node}"
    );
    let leaf = row_of(tree, "U1.2.4.5.6");
    assert!(!leaf.contains("/units/new"), "{leaf}");
    assert_eq!(page.body.matches("<h1").count(), 1);
}

#[tokio::test]
async fn machines_tree_of_a_live_machine_shows_its_live_units_only() {
    let world = world();
    seed_machine_with_archived_unit(&world.fixture);
    let page = world.get_en(&world.engineer, "/machines/M20").await;
    let tree = tree_part(&page.body);
    for path in ["U1", "U1.1", "U2"] {
        assert!(
            tree.contains(&format!("<span class=\"tree-path\">{path}</span>")),
            "{path}"
        );
    }
    assert!(
        !tree.contains("<span class=\"tree-path\">U3</span>"),
        "{tree}"
    );
    assert!(!tree.contains("badge"), "{tree}");
    let fragment = world
        .get(&world.engineer, "/machines/M20/units", &[ENGLISH, HTMX])
        .await;
    assert!(!fragment.body.contains(">U3<"), "{}", fragment.body);
}

// --- Spec "State shown by color, icon, and word": the archived machine --------------------------

#[tokio::test]
async fn machines_tree_of_an_archived_machine_shows_the_archived_unit_by_icon_and_word() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let path = format!("/machines/by-id/{M12_ID}");
        let reply = world.get(&world.engineer, &path, &[cookie_of(lang)]).await;
        let tree = tree_part(&reply.body);
        let word = text(lang, "status-archived");
        let badge = format!(
            "<span class=\"badge badge-retired\">{}",
            icon_tag("status-retired")
        );
        let archived = row_of(tree, "U2");
        assert!(archived.contains(&badge), "{lang}: {archived}");
        assert!(
            archived.contains(&format!("<span>{word}</span></span>")),
            "{archived}"
        );
        assert!(archived.contains("<span>Spare</span>"), "{archived}");
        let live = row_of(tree, "U1");
        assert!(!live.contains("badge"), "{live}");
        assert!(live.contains("<span>Infeed</span>"), "{live}");
    }
}

#[tokio::test]
async fn machines_tree_of_an_archived_machine_has_no_link_and_no_button_to_add() {
    let world = world();
    let path = format!("/machines/by-id/{M12_ID}");
    for session in [&world.engineer, &world.admin] {
        let reply = world.get_en(session, &path).await;
        let tree = tree_part(&reply.body);
        for unit in ["U1", "U2"] {
            let node = format!("<span class=\"tree-path\">{unit}</span>");
            assert!(tree.contains(&node), "the tree shows {unit}: {tree}");
        }
        assert!(!tree.contains("<a"), "{tree}");
        assert!(!tree.contains("href"), "{tree}");
        assert!(!tree.contains("hx-"), "{tree}");
        assert!(!tree.contains("autofocus"), "{tree}");
    }
}

// --- Escape -------------------------------------------------------------------------------------

#[tokio::test]
async fn machines_tree_escapes_the_name_of_a_unit() {
    let world = world();
    let payload = "<img src=x onerror=alert(1)>";
    let body = "code=M95&name=Line&plc_platform=S7_300&main_unit_name=%3Cimg+src%3Dx+onerror%3Dalert(1)%3E";
    let created = world.post(&world.engineer, "/machines", body, false).await;
    assert_eq!(created.status, StatusCode::SEE_OTHER, "{}", created.body);
    for path in ["/machines/M95", "/machines/M95/units"] {
        let reply = world.get_en(&world.engineer, path).await;
        assert!(!reply.body.contains(payload), "{path}: {}", reply.body);
        assert!(
            reply.body.contains("&#60;img src=x onerror=alert(1)&#62;"),
            "{path}"
        );
        assert_no_user_text_in_hx_attributes(&reply.body, "onerror");
        assert_clean_html(&reply.body);
    }
}
