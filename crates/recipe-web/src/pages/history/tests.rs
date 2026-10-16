//! Tests of `GET /recipes/{ref}/history` (task 11.2, ADR-0051 (g), specs "Recipe history page" and
//! "Pages of a deleted recipe").

use axum::http::StatusCode;

use crate::pages::audit::test_world::audit_world;
use crate::pages::test_helpers::{assert_clean_html, assert_error, assert_htmx_error, hx_pairs};
use crate::test_support::get_with;
use crate::test_support::plant::{HTMX, main_of};

/// The number of table rows in the body of the table.
fn body_rows(main: &str) -> usize {
    main.matches("<tr><td><span class=\"data\">").count()
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
async fn history_rows_come_newest_first() {
    let world = audit_world().await;
    let reply = world
        .get_en(&world.data.admin, "/recipes/R-500/history")
        .await;
    let ids: Vec<u64> = main_of(&reply.body)
        .split("href=\"/audit/")
        .skip(1)
        .filter_map(|rest| rest.split('"').next()?.parse().ok())
        .collect();
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
    let main = main_of(&reply.body);
    assert!(main.contains("Zatwierdzono wersję"), "{main}");
    assert!(main.contains("Receptura"), "{main}");
    assert!(!main.contains("Version approved"), "{main}");
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
