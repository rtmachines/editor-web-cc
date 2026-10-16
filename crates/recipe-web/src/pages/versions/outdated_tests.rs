//! Tests of the list of the outdated parameters on the version page (task 12.1).
//!
//! Spec "Rebind form of an outdated draft". ADR-0043 (j) and (l), design D7, `design-system.md`
//! section 9.

use axum::http::StatusCode;

use crate::i18n::Lang;
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{POLISH, cookie_of, main_of};
use crate::test_support::recipes::ZONES_KEY;
use crate::test_support::{icon_tag, text};

/// The version page of the draft of `R-100`. `ZONES` and `LINE` use `ST_Zone@2`.
const DRAFT_PAGE: &str = "/recipes/R-100/versions/3";

/// Return the markup of `#outdated-list` of a page: from its start tag to the parameter table.
pub(super) fn outdated_of(body: &str) -> &str {
    let main = main_of(body);
    let (_before, rest) = main.split_once("<div id=\"outdated-list\"").unwrap();
    rest.split("<div id=\"parameter-table\">").next().unwrap()
}

/// Return the markup of the row of the outdated parameter `tag`, from its link to the row end.
fn row_of<'a>(list: &'a str, tag: &str) -> &'a str {
    let (_before, rest) = list.split_once(&format!("\">{tag}</a>")).unwrap();
    rest.split("</tr>").next().unwrap()
}

#[tokio::test]
async fn outdated_list_names_each_outdated_parameter_with_the_state_outdated() {
    let world = recipe_world();
    let reply = world.get_en(&world.operator, DRAFT_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let list = outdated_of(&reply.body);
    assert!(list.contains(&text(Lang::En, "outdated-caption")), "{list}");
    assert!(list.contains(&text(Lang::En, "outdated-hint")), "{list}");
    let header = format!(
        "<th scope=\"col\">{}</th>",
        text(Lang::En, "outdated-col-struct")
    );
    assert!(list.contains(&header), "{list}");
    assert!(header.contains("Outdated struct"), "{header}");
    let link = format!("<a href=\"{DRAFT_PAGE}/parameters/{ZONES_KEY}\">ZONES</a>");
    assert!(list.contains(&link), "{list}");
    for tag in ["LINE", "ZONES"] {
        let row = row_of(list, tag);
        assert!(row.contains("<td class=\"data\">ST_Zone@2</td>"), "{row}");
        assert!(row.contains(&icon_tag("alert-warning")), "{row}");
        assert!(row.contains("<span>Outdated</span>"), "{row}");
    }
    assert!(
        row_of(list, "ZONES").contains("<td class=\"data\">U1.3</td>"),
        "{list}"
    );
    for tag in ["SPEED", "LINE_A"] {
        assert!(!list.contains(&format!("\">{tag}</a>")), "{tag}: {list}");
    }
    assert!(!list.contains("<form"), "an Operator gets no form: {list}");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn outdated_list_shows_the_word_of_the_ui_language() {
    let world = recipe_world();
    let reply = world.get(&world.operator, DRAFT_PAGE, &[POLISH]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let list = outdated_of(&reply.body);
    let word = format!("<span>{}</span>", text(Lang::Pl, "status-outdated"));
    assert!(row_of(list, "ZONES").contains(&word), "{list}");
    assert!(list.contains(&text(Lang::Pl, "outdated-caption")), "{list}");
}

/// The hint of `LINE` (`ST_Line@1` holds `ST_Zone@2`, and `ST_Zone@3` exists) in each language.
const LINE_HINTS: [(Lang, &str); 2] = [
    (
        Lang::En,
        "LINE uses ST_Zone inside the struct ST_Line@1. Create a new revision of ST_Line@1 that uses ST_Zone@3. Then rebind LINE.",
    ),
    (
        Lang::Pl,
        "LINE używa ST_Zone wewnątrz struktury ST_Line@1. Utwórz nową rewizję ST_Line@1, która używa ST_Zone@3. Potem przepnij LINE.",
    ),
];

#[tokio::test]
async fn outdated_list_tells_how_to_update_a_struct_inside_a_struct() {
    let world = recipe_world();
    for (lang, hint) in LINE_HINTS {
        let reply = world
            .get(&world.engineer, DRAFT_PAGE, &[cookie_of(lang)])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let list = outdated_of(&reply.body);
        let notice = format!("<div class=\"alert alert-info\">{}", icon_tag("alert-info"));
        assert!(list.contains(&notice), "{lang:?}: {list}");
        assert!(
            list.contains(&format!("<span>{hint}</span>")),
            "{lang:?}: {list}"
        );
        assert_eq!(list.matches("ST_Line@1").count(), 2, "only LINE: {list}");
        assert!(
            !list.contains("<legend>LINE</legend>"),
            "no rebind form moves LINE: {list}"
        );
        assert!(list.contains("<legend>ZONES</legend>"), "{lang:?}: {list}");
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn outdated_list_is_empty_on_a_version_that_is_not_a_draft() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, "/recipes/R-100/versions/2")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        main_of(&reply.body).contains("<div id=\"outdated-list\"></div>"),
        "{}",
        reply.body
    );
}
