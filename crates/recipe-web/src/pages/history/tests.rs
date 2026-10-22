//! Tests of `GET /recipes/{ref}/history` (task 11.2, ADR-0051 (g), specs "Recipe history page" and
//! "Pages of a deleted recipe").

use axum::http::StatusCode;
use recipe_app::blocking::{AppSettings, PageSizes};

use crate::pages::audit::test_world::{
    AuditWorld, TIME_CELL, admin_main, assert_object_cells_mark_the_label, audit_world,
    audit_world_with, body_rows, entry_ids, older_href,
};
use crate::pages::test_helpers::{
    assert_clean_html, assert_error, assert_htmx_error, hx_pairs, part_of,
};
use crate::test_support::get_with;
use crate::test_support::plant::{HTMX, main_of};

/// Build the world with the approval data and `rows` rows on one page of the history.
async fn world_with_history_page_of(rows: u32) -> AuditWorld {
    let defaults = AppSettings::default().pages;
    audit_world_with(PageSizes {
        history: rows,
        ..defaults
    })
    .await
}

#[tokio::test]
async fn history_world_with_a_small_page_seeds_the_creator_of_the_versions() {
    let world = world_with_history_page_of(2).await;

    let reply = world
        .get_en(&world.data.admin, "/recipes/R-200/versions/1")
        .await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

#[tokio::test]
async fn history_of_an_operator_shows_the_actions_and_no_link_to_an_entry() {
    let world = audit_world().await;
    let reply = world
        .get_en(&world.operator, "/recipes/R-500/history")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("<h1>History of recipe R-500</h1>"), "{main}");
    assert!(main.contains("Version approved"), "{main}");
    assert!(body_rows(main) >= 5, "{main}");
    assert!(!main.contains("href=\"/audit"), "{main}");
    assert!(!main.contains("<pre"), "{main}");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn history_of_an_engineer_and_an_administrator_links_each_row_to_its_entry() {
    let world = audit_world().await;
    for session in [&world.data.anowak, &world.data.admin] {
        let reply = world.get_en(session, "/recipes/R-500/history").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        let rows = body_rows(main);
        assert!(rows >= 5, "{main}");
        assert_eq!(main.matches("href=\"/audit/").count(), rows, "{main}");
        assert!(main.contains("Entry "), "{main}");
        assert!(main.contains("href=\"/audit?aggregate="), "{main}");
    }
}

#[tokio::test]
async fn history_the_time_cell_of_each_row_holds_the_time_span_that_does_not_break() {
    let world = audit_world().await;
    let reply = world
        .get_en(&world.operator, "/recipes/R-500/history")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let (_head, table) = main.split_once("<tbody>").unwrap();
    let rows = table.matches("<tr>").count();
    assert!(rows >= 5, "{main}");
    assert_eq!(table.matches(TIME_CELL).count(), rows, "{main}");
}

#[tokio::test]
async fn history_rows_come_newest_first() {
    let world = audit_world().await;
    let reply = world
        .get_en(&world.data.admin, "/recipes/R-500/history")
        .await;
    let ids = entry_ids(main_of(&reply.body));
    assert!(ids.len() >= 5, "{ids:?}");
    let mut sorted = ids.clone();
    sorted.sort_unstable_by(|left, right| right.cmp(left));
    assert_eq!(ids, sorted);
}

#[tokio::test]
async fn history_of_a_deleted_recipe_is_for_the_administrator_only() {
    let world = audit_world().await;
    let admin = world
        .get_en(&world.data.admin, "/recipes/R-700/history")
        .await;
    assert_eq!(admin.status, StatusCode::OK, "{}", admin.body);
    assert!(
        main_of(&admin.body).contains("The recipe has no entries."),
        "{}",
        admin.body
    );
    for session in [&world.data.anowak, &world.operator] {
        let reply = world.get_en(session, "/recipes/R-700/history").await;
        assert_error(&reply, StatusCode::CONFLICT, "E-RCP-012", "R-700");
    }
}

#[tokio::test]
async fn history_of_a_deleted_recipe_shows_the_state_deleted_as_color_icon_and_word() {
    let world = audit_world().await;
    let deleted = world
        .get_en(&world.data.admin, "/recipes/R-700/history")
        .await;
    let main = main_of(&deleted.body);
    assert!(
        main.contains("<span class=\"badge badge-large badge-deleted\"><svg class=\"icon\""),
        "{main}"
    );
    assert!(main.contains("data-icon=\"status-retired\""), "{main}");
    assert!(main.contains("<span>Deleted</span>"), "{main}");
    let live = world
        .get_en(&world.data.admin, "/recipes/R-500/history")
        .await;
    assert!(
        !main_of(&live.body).contains("badge-deleted"),
        "{}",
        live.body
    );
}

#[tokio::test]
async fn history_path_and_query_values_give_the_codes_of_the_api() {
    let world = audit_world().await;
    for path in ["/recipes/R-999/history", "/recipes/-x/history"] {
        let reply = world.get_en(&world.data.admin, path).await;
        assert_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005", path);
    }
    for path in [
        "/recipes/R-500/history?before=abc",
        "/recipes/R-500/history?before=0",
        "/recipes/R-500/history?before=-1",
        "/recipes/R-500/history?after=1",
    ] {
        let reply = world.get_en(&world.data.admin, path).await;
        assert_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008", path);
    }
}

#[tokio::test]
async fn history_a_bad_cursor_gives_the_reason_of_the_cursor() {
    let world = audit_world().await;
    let reply = world
        .get_en(&world.data.admin, "/recipes/R-500/history?before=abc")
        .await;
    assert_error(
        &reply,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-WEB-008",
        "before",
    );
    let expected = "Field Position in the list is not valid: the link of the page is not valid. Open the first page of the list.";
    assert!(reply.body.contains(expected), "{}", reply.body);
}

#[tokio::test]
async fn history_cursor_pager_shows_newest_and_the_end_of_the_list() {
    let world = audit_world().await;
    let last = world
        .get_en(&world.data.admin, "/recipes/R-500/history")
        .await;
    let main = main_of(&last.body);
    assert!(
        main.contains("href=\"/recipes/R-500/history\">Newest entries</a>"),
        "{main}"
    );
    assert!(main.contains("No older entries."), "{main}");
    let beyond = world
        .get_en(&world.data.admin, "/recipes/R-500/history?before=1")
        .await;
    let page = main_of(&beyond.body);
    assert_eq!(body_rows(page), 0, "{page}");
    assert!(page.contains("The recipe has no entries."), "{page}");
    assert!(page.contains("No older entries."), "{page}");
}

/// The link to the older page keeps the recipe, and the older page holds the next older entries
/// (ADR-0051 (i), `design-system.md` section 10.12, finding CX-RCW-2).
#[tokio::test]
async fn history_older_link_opens_the_next_older_entries_of_the_same_recipe() {
    const PATH: &str = "/recipes/R-500/history";
    // The page of 100 rows holds the whole history. It is the order that the pages must keep.
    let all = entry_ids(&admin_main(&audit_world().await, PATH).await);
    assert!(all.len() >= 5, "{all:?}");
    let (newer, rest) = all.split_at(2);
    let (older, _rest) = rest.split_at(2);
    let world = world_with_history_page_of(2).await;
    let first = admin_main(&world, PATH).await;
    assert_eq!(entry_ids(&first), newer, "{first}");
    let href = older_href(&first);
    let cursor = href.strip_prefix(&format!("{PATH}?before=")).unwrap();
    assert_eq!(cursor.parse::<u64>().ok(), newer.last().copied(), "{href}");
    let second = admin_main(&world, &href).await;
    assert_eq!(entry_ids(&second), older, "{second}");
    assert!(
        second.contains("<h1>History of recipe R-500</h1>"),
        "{second}"
    );
    assert!(
        second.contains(&format!("href=\"{PATH}\">Newest entries</a>")),
        "{second}"
    );
    let (first_ids, second_ids) = (entry_ids(&first), entry_ids(&second));
    assert!(
        second_ids.iter().all(|id| !first_ids.contains(id)),
        "{first_ids:?} {second_ids:?}"
    );
    assert!(newer.last() > older.first(), "{newer:?} {older:?}");
}

#[tokio::test]
async fn history_page_hx_attributes_are_literals_and_hold_no_recipe_data() {
    let world = audit_world().await;
    let reply = world
        .get_content(&world.data.admin, "/recipes/R-500/history")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<title>History of recipe R-500 · Recipe editor</title>")
    );
    assert!(
        reply.body.contains("<h1 tabindex=\"-1\" autofocus>"),
        "{}",
        reply.body
    );
    let pairs = hx_pairs(&reply.body);
    assert!(!pairs.is_empty());
    for (name, value) in &pairs {
        assert!(
            !value.contains("R-500") && !value.contains("audit"),
            "{name}={value}"
        );
    }
}

#[tokio::test]
async fn history_words_come_from_the_polish_bundle() {
    let world = audit_world().await;
    let reply = world
        .get(&world.data.anowak, "pl", "/recipes/R-500/history")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let rows = part_of(main, "<tbody>", "</tbody>");
    assert!(rows.contains("Zatwierdzono wersję"), "{rows}");
    assert!(
        rows.contains("Receptura <span class=\"data\">R-500</span>"),
        "{rows}"
    );
    assert!(rows.contains("Wersja <span class=\"data\">"), "{rows}");
    assert!(!main.contains("Version approved"), "{main}");
}

#[tokio::test]
async fn history_object_cell_shows_the_label_in_mono_and_the_type_word_outside_it() {
    let world = audit_world().await;
    for (code, recipe, version) in [("en", "Recipe", "Version"), ("pl", "Receptura", "Wersja")] {
        let reply = world
            .get(&world.data.admin, code, "/recipes/R-500/history")
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_object_cells_mark_the_label(main_of(&reply.body), recipe, version);
    }
}

#[tokio::test]
async fn history_error_by_htmx_retargets_the_error_panel() {
    let world = audit_world().await;
    let cookie = format!("{}; recipe_lang=en", world.operator.cookie);
    let reply = get_with(
        &world.fixture.router(),
        "/recipes/R-700/history",
        &[("cookie", cookie.as_str()), HTMX],
    )
    .await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-RCP-012");
    assert_eq!(reply.header("hx-push-url"), "false");
}
