//! Tests of the type tree page `GET /machines/{code}/types/{name}` (task 8.4, spec "Type pages",
//! design D6 and D10).

use axum::http::StatusCode;
use recipe_domain::plant::Primitive;

use super::test_kit::{
    assert_content_answer, between, get_as, get_in, primitive_id, seed_array, seed_cycle,
    seed_struct, string_id, struct_id,
};
use crate::pages::test_helpers::{assert_clean_html, assert_no_user_text_in_hx_attributes};
use crate::test_support::icon_tag;
use crate::test_support::plant::{HTMX_CONTENT, POLISH, world};

const ZONE: &str = "/machines/M7/types/ST_Zone";

/// The start of a tree group node: the row, the button, and the id of its group.
fn node_open(id: &str, index: usize, expanded: bool) -> String {
    format!(
        "<li><div class=\"tree-row\"><button type=\"button\" class=\"tree-node\" aria-expanded=\"{expanded}\" aria-controls=\"tree-group-{id}-{index}\">"
    )
}

/// The end of a tree group node, after the two icons: the words and the start of the group.
fn node_close(id: &str, index: usize, words: &[&str], expanded: bool) -> String {
    // The first word is the member name. A second word is the member type, in the muted data face.
    let spans: String = words
        .iter()
        .enumerate()
        .map(|(at, word)| {
            let class = if at == 0 { "data" } else { "data tree-type" };
            format!("<span class=\"{class}\">{word}</span>")
        })
        .collect();
    let hidden = if expanded { "" } else { " hidden" };
    format!(
        "{spans}</button></div><ul class=\"tree-group\" id=\"tree-group-{id}-{index}\"{hidden}>"
    )
}

/// The leaf of a tree.
fn leaf(name: &str, type_text: &str) -> String {
    format!(
        "<li class=\"tree-leaf\"><span class=\"data\">{name}</span><span class=\"data tree-type\">{type_text}</span></li>"
    )
}

/// Return the markup of the tree: from the first item to the end of the section.
///
/// The text starts behind the opening tag of the root list and ends with its closing tag.
fn tree_of(body: &str) -> &str {
    between(body, "<ul class=\"tree\">", "</section>")
}

// --- The page -----------------------------------------------------------------------------------

#[tokio::test]
async fn types_tree_page_shows_the_tree_of_the_latest_revision_and_the_links_to_the_revisions() {
    let world = world();
    let reply = get_as(&world, &world.operator, ZONE, &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = &reply.body;
    assert!(body.contains("<h1>Type ST_Zone@2</h1>"), "{body}");
    assert!(body.contains("<title>Type ST_Zone@2 · "), "{body}");
    for (revision, current) in [(2, " aria-current=\"page\""), (1, "")] {
        let link = format!(
            "<a class=\"target-link data\" href=\"/machines/M7/types/ST_Zone?rev={revision}\"{current}>ST_Zone@{revision}</a>"
        );
        assert!(body.contains(&link), "{revision}: {body}");
    }
    assert!(body.contains("<h2>Revisions</h2>"), "{body}");
    assert!(
        body.contains("<h2 id=\"type-tree-title\">Type tree</h2>"),
        "{body}"
    );
    assert!(
        !body.contains("/machines/M7/types/new?from=ST_Zone"),
        "{body}"
    );
}

#[tokio::test]
async fn types_tree_page_writes_the_root_group_open_with_its_members_as_leaves() {
    let world = world();
    let reply = get_as(&world, &world.operator, ZONE, &[]).await;
    let id = world.ids.zone_v2.to_string();
    let tree = tree_of(&reply.body);
    let open = node_open(&id, 0, true);
    let close = node_close(&id, 0, &["ST_Zone@2"], true);
    assert!(tree.starts_with(&open), "{tree}");
    let icons = between(tree, &open, &close);
    let collapsed = icons.find(&icon_tag("tree-collapsed")).unwrap();
    let expanded = icons.find(&icon_tag("tree-expanded")).unwrap();
    assert!(collapsed < expanded, "{icons}");
    let members = format!("{}{}", leaf("speed", "INT"), leaf("temp_max", "REAL"));
    assert!(
        tree.ends_with(&format!("{close}{members}</ul></li></ul>")),
        "{tree}"
    );
}

#[tokio::test]
async fn types_tree_page_offers_the_usage_page_of_the_shown_revision() {
    let world = world();
    for (path, revision) in [(ZONE, 2), ("/machines/M7/types/ST_Zone?rev=1", 1)] {
        let reply = get_as(&world, &world.operator, path, &[]).await;
        let link = format!(
            "<a class=\"target-link\" href=\"/machines/M7/types/ST_Zone/usage?rev={revision}\">Show the use of this revision</a>"
        );
        assert!(reply.body.contains(&link), "{path}: {}", reply.body);
    }
}

#[tokio::test]
async fn types_tree_page_link_to_a_new_revision_needs_the_udt_permission() {
    let world = world();
    let link =
        "<a class=\"target-link\" href=\"/machines/M7/types/new?from=ST_Zone\">New revision</a>";
    for (session, shown) in [
        (&world.admin, true),
        (&world.engineer, true),
        (&world.operator, false),
    ] {
        let reply = get_as(&world, session, ZONE, &[]).await;
        assert_eq!(reply.body.contains(link), shown, "{}", reply.body);
    }
}

#[tokio::test]
async fn types_tree_page_names_the_revision_that_it_supersedes() {
    let world = world();
    let reply = get_as(&world, &world.operator, ZONE, &[]).await;
    let supersedes = "<dl><dt>Supersedes</dt><dd class=\"data\"><a class=\"target-link\" href=\"/machines/M7/types/ST_Zone?rev=1\">ST_Zone@1</a></dd></dl>";
    assert!(reply.body.contains(supersedes), "{}", reply.body);
    let first = get_as(
        &world,
        &world.operator,
        "/machines/M7/types/ST_Zone?rev=1",
        &[],
    )
    .await;
    assert!(
        first.body.contains("<h1>Type ST_Zone@1</h1>"),
        "{}",
        first.body
    );
    assert!(!first.body.contains("Supersedes"), "{}", first.body);
    assert!(first.body.contains(&leaf("speed", "INT")));
    assert!(!first.body.contains("temp_max"));
}

#[tokio::test]
async fn types_tree_page_links_back_to_the_type_list() {
    let world = world();
    let reply = get_as(&world, &world.operator, ZONE, &[]).await;
    let link = "<a class=\"target-link\" href=\"/machines/M7/types\">UDT types of machine M7</a>";
    assert!(reply.body.contains(link), "{}", reply.body);
}

// --- The groups of the tree (design D10, ADR-0037 e) ---------------------------------------------

#[tokio::test]
async fn types_tree_nested_struct_and_array_are_closed_groups_with_numbered_ids() {
    let world = world();
    let reply = get_as(&world, &world.operator, "/machines/M7/types/ST_Line", &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let line = world.ids.line_v1.to_string();
    let zone = world.ids.zone_v2.to_string();
    let tree = tree_of(&reply.body);
    assert!(tree.starts_with(&node_open(&line, 0, true)), "{tree}");
    assert!(tree.contains(&node_open(&zone, 1, false)), "{tree}");
    let words = ["zones", "ARRAY[1..4] OF ST_Zone@2"];
    assert!(
        tree.contains(&node_close(&zone, 1, &words, false)),
        "{tree}"
    );
    assert!(
        tree.contains(&node_close(&line, 0, &["ST_Line@1"], true)),
        "{tree}"
    );
    assert_eq!(
        tree.matches("speed").count(),
        1,
        "the element type shows once"
    );
    assert_eq!(tree.matches(" hidden").count(), 1);
    assert_eq!(tree.matches("aria-expanded=\"true\"").count(), 1);
    assert_eq!(tree.matches("aria-expanded=\"false\"").count(), 1);
}

#[tokio::test]
async fn types_tree_group_ids_differ_when_one_type_stands_at_two_places() {
    let world = world();
    let zone = world.ids.zone_v2;
    seed_struct(&world, 10, "ST_Pair", &[("first", zone), ("second", zone)]);
    let reply = get_as(&world, &world.operator, "/machines/M7/types/ST_Pair", &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let id = zone.to_string();
    let tree = tree_of(&reply.body);
    assert!(
        tree.contains(&format!("aria-controls=\"tree-group-{id}-1\"")),
        "{tree}"
    );
    assert!(
        tree.contains(&format!("aria-controls=\"tree-group-{id}-4\"")),
        "{tree}"
    );
    assert!(
        tree.contains(&format!("id=\"tree-group-{id}-1\" hidden>")),
        "{tree}"
    );
    assert!(
        tree.contains(&format!("id=\"tree-group-{id}-4\" hidden>")),
        "{tree}"
    );
    let mut ids: Vec<&str> = tree
        .split(" id=\"")
        .skip(1)
        .map(|part| part.split('"').next().unwrap())
        .collect();
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(count, 3);
    assert_eq!(ids.len(), count, "{ids:?}");
}

#[tokio::test]
async fn types_tree_primitive_members_are_leaves_with_the_name_of_their_type() {
    let world = world();
    let bool_id = primitive_id(Primitive::Bool);
    seed_struct(
        &world,
        11,
        "ST_Mixed",
        &[("flag", bool_id), ("label", string_id())],
    );
    let reply = get_as(&world, &world.operator, "/machines/M7/types/ST_Mixed", &[]).await;
    let tree = tree_of(&reply.body);
    assert!(tree.contains(&leaf("flag", "BOOL")), "{tree}");
    assert!(tree.contains(&leaf("label", "STRING")), "{tree}");
    assert_eq!(tree.matches("<li class=\"tree-leaf\">").count(), 2);
}

#[tokio::test]
async fn types_tree_array_of_a_primitive_is_one_leaf_with_the_array_in_its_type() {
    let world = world();
    let int = primitive_id(Primitive::Int);
    let cells = seed_array(&world, 20, int, &[(1, 8)]);
    let grid = seed_array(&world, 21, cells, &[(0, 1), (1, 3)]);
    seed_struct(&world, 12, "ST_Table", &[("cells", cells), ("grid", grid)]);
    let reply = get_as(&world, &world.operator, "/machines/M7/types/ST_Table", &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let tree = tree_of(&reply.body);
    assert!(
        tree.contains(&leaf("cells", "ARRAY[1..8] OF INT")),
        "{tree}"
    );
    assert!(
        tree.contains(&leaf("grid", "ARRAY[0..1,1..3] OF ARRAY[1..8] OF INT")),
        "{tree}"
    );
    assert_eq!(tree.matches("<button").count(), 1);
}

#[tokio::test]
async fn types_tree_of_a_deep_chain_is_balanced_and_only_the_root_is_open() {
    let world = world();
    let levels: u128 = 20;
    for level in 1..=levels {
        let next = if level == levels {
            primitive_id(Primitive::Int)
        } else {
            struct_id(level.checked_add(100).unwrap())
        };
        let number = level.checked_add(99).unwrap();
        seed_struct(
            &world,
            number,
            &format!("ST_Chain{level}"),
            &[("next", next)],
        );
    }
    let reply = get_as(&world, &world.operator, "/machines/M7/types/ST_Chain1", &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let tree = tree_of(&reply.body);
    assert_eq!(tree.matches("<button").count(), 20);
    assert_eq!(tree.matches(" hidden>").count(), 19);
    assert_eq!(tree.matches("aria-expanded=\"true\"").count(), 1);
    assert_eq!(tree.matches("<li class=\"tree-leaf\">").count(), 1);
    assert_eq!(tree.matches("<ul").count(), 20);
    assert_eq!(
        tree.matches("</ul>").count(),
        21,
        "the 20 groups and the root list"
    );
    assert_eq!(tree.matches("<li").count(), 21);
    assert_eq!(tree.matches("</li>").count(), 21);
    assert!(tree.ends_with("</li></ul></li></ul>"), "{tree}");
}

#[tokio::test]
async fn types_tree_page_keeps_every_list_and_item_balanced() {
    let world = world();
    let reply = get_as(&world, &world.operator, "/machines/M7/types/ST_Line", &[]).await;
    let tree = tree_of(&reply.body);
    assert_eq!(
        tree.matches("<ul").count().checked_add(1),
        Some(tree.matches("</ul>").count())
    );
    assert_eq!(
        tree.matches("<li").count(),
        tree.matches("</li>").count(),
        "{tree}"
    );
}

// --- Errors -------------------------------------------------------------------------------------

#[tokio::test]
async fn types_tree_of_an_unknown_or_malformed_name_revision_or_machine_is_not_found() {
    let world = world();
    for path in [
        "/machines/M7/types/ST_Nope",
        "/machines/M7/types/9bad",
        "/machines/M7/types/INT",
        "/machines/M7/types/ST_Zone?rev=9",
        "/machines/M7/types/ST_Zone?rev=0",
        "/machines/M7/types/ST_Zone?rev=02",
        "/machines/M7/types/ST_Zone?rev=-1",
        "/machines/M7/types/ST_Zone?rev=abc",
        "/machines/M7/types/ST_Zone?rev=",
        "/machines/M7/types/ST_Zone?rev=%3Cscript%3E",
        "/machines/M99/types/ST_Zone",
        "/machines/M12/types/ST_Zone",
        "/machines/X7/types/ST_Zone",
    ] {
        let reply = get_as(&world, &world.operator, path, &[]).await;
        assert_eq!(
            reply.status,
            StatusCode::NOT_FOUND,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-005"), "{path}");
        assert!(!reply.body.contains("<script>"), "{path}");
    }
}

#[tokio::test]
async fn types_tree_with_an_unknown_query_key_gives_422() {
    let world = world();
    for path in [
        "/machines/M7/types/ST_Zone?x=1",
        "/machines/M7/types/ST_Zone?csrf_token=abc",
        "/machines/M7/types/ST_Zone?rev=1&page=2",
    ] {
        let reply = get_as(&world, &world.operator, path, &[]).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-008"), "{path}");
    }
}

#[tokio::test]
async fn types_tree_of_a_struct_that_contains_itself_gives_the_cycle_error() {
    let world = world();
    seed_cycle(&world);
    let reply = get_as(&world, &world.operator, "/machines/M7/types/ST_Alpha", &[]).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-PLT-005"), "{}", reply.body);
    let list = get_as(&world, &world.operator, "/machines/M7/types", &[]).await;
    assert_eq!(list.status, StatusCode::OK);
}

// --- Languages, markup, and the content answer ----------------------------------------------------

#[tokio::test]
async fn types_tree_page_in_polish_uses_the_polish_words() {
    let world = world();
    let reply = get_in(&world, &world.engineer, ZONE, POLISH).await;
    let body = &reply.body;
    assert!(body.contains("<h1>Typ ST_Zone@2</h1>"), "{body}");
    assert!(body.contains("<h2>Rewizje</h2>"), "{body}");
    assert!(
        body.contains("<h2 id=\"type-tree-title\">Drzewo typu</h2>"),
        "{body}"
    );
    assert!(body.contains("<dt>Zastępuje</dt>"), "{body}");
    assert!(body.contains(">Pokaż użycie tej rewizji</a>"), "{body}");
    assert!(body.contains(">Nowa rewizja</a>"), "{body}");
}

#[tokio::test]
async fn types_tree_page_holds_no_form_no_user_text_in_an_hx_attribute_and_no_inline_code() {
    let world = world();
    let reply = get_as(&world, &world.engineer, "/machines/M7/types/ST_Line", &[]).await;
    assert_clean_html(&reply.body);
    for needle in ["M7", "ST_Line", "ST_Zone", "zones"] {
        assert_no_user_text_in_hx_attributes(&reply.body, needle);
    }
    let main = reply.body.split_once("<main id=\"content\">").unwrap().1;
    assert!(!main.contains("<form"), "{main}");
    assert!(!main.contains("hx-"), "{main}");
    assert!(
        !main.contains("role=\"tree"),
        "the tree uses the disclosure pattern"
    );
}

#[tokio::test]
async fn types_tree_page_content_answer_holds_the_title_and_the_focused_heading() {
    let world = world();
    let path = "/machines/M7/types/ST_Zone?rev=1";
    let reply = get_as(&world, &world.operator, path, &HTMX_CONTENT).await;
    assert_content_answer(&reply, "Type ST_Zone@1", path);
}
