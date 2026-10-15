//! Tests of `GET /machines` (task 7.1, specs "Machine list page" and "Content answer of a page").

use axum::http::StatusCode;

use super::test_world::{row_with, seed_machines};
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::pages::test_helpers::{assert_clean_html, assert_no_user_text_in_hx_attributes};
use crate::test_support::plant::{ENGLISH, HTMX, M12_ID, World, cookie_of, text_with, world};
use crate::test_support::{icon_tag, text};

/// The name of the machines of the pager tests. It holds each character that a link must encode.
const SEARCH_NAME: &str = "a&b #+%Ł";
/// `SEARCH_NAME` as a query value.
const SEARCH_QUERY: &str = "a%26b%20%23%2B%25%C5%81";

/// Return the markup of the search form of a page.
fn search_form(body: &str) -> &str {
    let (_before, rest) = body
        .split_once("<form method=\"get\" action=\"/machines\"")
        .unwrap();
    rest.split("</form>").next().unwrap()
}

/// Seed 101 live machines with `SEARCH_NAME`, so the list has a second page.
fn world_with_two_pages() -> World {
    let world = world();
    seed_machines(&world.fixture, 101, SEARCH_NAME);
    world
}

// --- Spec "Machine list page" -------------------------------------------------------------------

#[tokio::test]
async fn machines_list_of_an_operator_has_the_row_and_no_new_link() {
    let world = world();
    let reply = world.get_en(&world.operator, "/machines").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<title>Machines · Recipe editor</title>"),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("<a href=\"/machines/M7\">M7</a>"));
    assert!(!reply.body.contains("href=\"/machines/new\""));
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn machines_list_of_a_role_with_machine_manage_has_the_new_machine_link() {
    let world = world();
    for session in [&world.engineer, &world.admin] {
        let reply = world.get_en(session, "/machines").await;
        let link = "<a class=\"target-link\" href=\"/machines/new\">New machine</a>";
        assert!(reply.body.contains(link), "{}", reply.body);
    }
}

#[tokio::test]
async fn machines_list_is_a_table_with_four_columns_in_both_languages() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.engineer, "/machines", &[cookie_of(lang)])
            .await;
        let caption = text(lang, "machines-caption");
        let frame = format!(
            "<div class=\"table-wrap\" role=\"region\" tabindex=\"0\" aria-labelledby=\"machines-caption\"><table><caption id=\"machines-caption\">{caption}</caption>"
        );
        assert!(reply.body.contains(&frame), "{lang}: {}", reply.body);
        for key in [
            "column-code",
            "column-name",
            "column-platform",
            "column-state",
        ] {
            let head = format!("<th scope=\"col\">{}</th>", text(lang, key));
            assert!(reply.body.contains(&head), "{lang} {key}");
        }
        assert_eq!(reply.body.matches("<th scope=\"col\">").count(), 4);
    }
}

#[tokio::test]
async fn machines_list_row_of_a_live_machine_shows_code_name_and_platform_without_a_badge() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.operator, "/machines", &[cookie_of(lang)])
            .await;
        let row = row_with(&reply.body, "/machines/M7");
        let platform = text(lang, "platform-s7-300");
        assert!(row.contains("<td>Press line</td>"), "{row}");
        assert!(row.contains(&format!("<td>{platform}</td>")), "{row}");
        assert!(!row.contains("badge"), "{row}");
    }
}

#[tokio::test]
async fn machines_list_hides_the_archived_machines_without_the_filter() {
    let world = world();
    let reply = world.get_en(&world.engineer, "/machines").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("/machines/by-id/"), "{}", reply.body);
    assert!(!reply.body.contains("M12"), "{}", reply.body);
}

#[tokio::test]
async fn machines_list_archived_machine_shows_icon_word_and_the_link_by_id() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let path = "/machines?show_archived=1";
        let reply = world.get(&world.engineer, path, &[cookie_of(lang)]).await;
        let row = row_with(&reply.body, "M12");
        let badge = format!(
            "<span class=\"badge badge-retired\">{}",
            icon_tag("status-retired")
        );
        assert!(row.contains(&badge), "{row}");
        let word = format!("<span>{}</span></span>", text(lang, "status-archived"));
        assert!(row.contains(&word), "{row}");
        assert!(row.contains(&format!("href=\"/machines/by-id/{M12_ID}\"")));
        assert!(!row.contains("href=\"/machines/M12\""), "{row}");
        let live = row_with(&reply.body, "/machines/M7");
        assert!(live.contains("href=\"/machines/M7\""), "{live}");
    }
}

#[tokio::test]
async fn machines_list_search_matches_a_part_of_the_code_or_the_name_in_any_case() {
    let world = world();
    for query in ["q=press", "q=PRESS", "q=m7", "q=ss%20li"] {
        let reply = world
            .get_en(&world.operator, &format!("/machines?{query}"))
            .await;
        assert!(reply.body.contains("href=\"/machines/M7\""), "{query}");
    }
    let archived = world
        .get_en(&world.operator, "/machines?q=m1&show_archived=1")
        .await;
    assert!(
        archived.body.contains("/machines/by-id/"),
        "{}",
        archived.body
    );
    assert!(!archived.body.contains("href=\"/machines/M7\""));
}

#[tokio::test]
async fn machines_list_search_form_echoes_the_text_and_the_checkbox() {
    let world = world();
    let reply = world
        .get_en(&world.operator, "/machines?q=Press&show_archived=1")
        .await;
    let form = search_form(&reply.body);
    assert!(form.contains("<label for=\"machine_q\">Code or name contains</label>"));
    assert!(form.contains("id=\"machine_q\""), "{form}");
    assert!(form.contains("name=\"q\""), "{form}");
    assert!(form.contains("value=\"Press\""), "{form}");
    assert!(form.contains("autocomplete=\"off\""), "{form}");
    assert!(form.contains("id=\"show_archived\" name=\"show_archived\" value=\"1\" checked"));
    assert!(form.contains(">Search</button>"), "{form}");
    assert!(!form.contains("csrf_token"), "a GET form holds no token");
    let plain = world.get_en(&world.operator, "/machines").await;
    assert!(!search_form(&plain.body).contains("checked"));
}

#[tokio::test]
async fn machines_list_without_a_match_shows_the_empty_state() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.operator, "/machines?q=zzz", &[cookie_of(lang)])
            .await;
        let empty = format!("<p class=\"empty\">{}</p>", text(lang, "machines-empty"));
        assert!(reply.body.contains(&empty), "{lang}: {}", reply.body);
        assert!(!reply.body.contains("href=\"/machines/M7\""));
    }
}

#[tokio::test]
async fn machines_list_escapes_the_search_text_and_keeps_it_out_of_hx_attributes() {
    let world = world();
    let payload = "\"><script>alert(1)</script>";
    let encoded = "%22%3E%3Cscript%3Ealert(1)%3C%2Fscript%3E";
    let reply = world
        .get_en(&world.operator, &format!("/machines?q={encoded}"))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("<script>alert"), "{}", reply.body);
    assert!(reply.body.contains("&#60;script&#62;"), "{}", reply.body);
    assert_no_user_text_in_hx_attributes(&reply.body, payload);
    assert_clean_html(&reply.body);
}

// --- Pages of the list --------------------------------------------------------------------------

#[tokio::test]
async fn machines_list_first_page_holds_100_rows_and_a_next_link_with_the_encoded_text() {
    let world = world_with_two_pages();
    let path = format!("/machines?q={SEARCH_QUERY}");
    let reply = world.get_en(&world.operator, &path).await;
    assert_eq!(reply.body.matches("<td class=\"data\">").count(), 100);
    let next = format!("href=\"/machines?q={SEARCH_QUERY}&#38;page=2\">Next page</a>");
    assert!(reply.body.contains(&next), "{}", reply.body);
    assert!(!reply.body.contains(">Previous page</a>"));
    assert!(
        reply
            .body
            .contains("<span class=\"pager-current\" aria-current=\"page\">Page 1</span>")
    );
}

#[tokio::test]
async fn machines_list_second_page_holds_the_last_row_and_a_previous_link() {
    let world = world_with_two_pages();
    let path = format!("/machines?q={SEARCH_QUERY}&page=2");
    let reply = world.get_en(&world.operator, &path).await;
    assert_eq!(reply.body.matches("<td class=\"data\">").count(), 1);
    let previous = format!("href=\"/machines?q={SEARCH_QUERY}&#38;page=1\">Previous page</a>");
    assert!(reply.body.contains(&previous), "{}", reply.body);
    assert!(!reply.body.contains(">Next page</a>"));
}

#[tokio::test]
async fn machines_list_pager_keeps_the_archived_filter_and_omits_an_empty_text() {
    let world = world_with_two_pages();
    let filtered = format!("/machines?q={SEARCH_QUERY}&show_archived=1");
    let reply = world.get_en(&world.operator, &filtered).await;
    let next = format!("q={SEARCH_QUERY}&#38;show_archived=1&#38;page=2");
    assert!(reply.body.contains(&next), "{}", reply.body);
    let plain = world.get_en(&world.operator, "/machines").await;
    assert!(
        plain.body.contains("href=\"/machines?page=2\""),
        "{}",
        plain.body
    );
    let empty_q = world.get_en(&world.operator, "/machines?q=").await;
    assert!(empty_q.body.contains("href=\"/machines?page=2\""));
}

#[tokio::test]
async fn machines_list_without_a_second_page_shows_no_pager() {
    let world = world();
    let reply = world.get_en(&world.operator, "/machines").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("class=\"pager\""), "{}", reply.body);
}

/// D17 (U16): the page after the end shows `list-page-after-end`, not the empty state.
#[tokio::test]
async fn machines_list_after_the_last_page_shows_the_after_end_text_and_no_next_link() {
    let world = world_with_two_pages();
    let reply = world.get_en(&world.operator, "/machines?page=3").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("Page 3 is after the last page."));
    assert!(!reply.body.contains(&text(Lang::En, "machines-empty")));
    assert!(!reply.body.contains("class=\"empty\""));
    assert!(!reply.body.contains(">Next page</a>"));
    // RQ-5: page 2 is the last page, so a link to it from page 3 would not help.
    assert!(!reply.body.contains("Previous page"), "{}", reply.body);
    assert!(!reply.body.contains("class=\"pager\""), "{}", reply.body);
}

/// Spec "Search text and pages of a list", scenario "Page after the end".
#[tokio::test]
async fn machines_list_after_the_end_links_to_the_first_page_in_both_languages() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.operator, "/machines?page=2", &[cookie_of(lang)])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let sentence = text_with(lang, "list-page-after-end", "page", "2");
        assert!(reply.body.contains(&sentence), "{lang}: {}", reply.body);
        let link = format!(
            "<a class=\"target-link\" href=\"/machines?page=1\">{}</a>",
            text(lang, "list-first-page")
        );
        assert!(reply.body.contains(&link), "{lang}: {}", reply.body);
        assert!(!reply.body.contains(&text(lang, "machines-empty")));
        assert!(!reply.body.contains("href=\"/machines/M7\""));
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn machines_list_after_the_end_keeps_the_search_text_in_the_first_page_link() {
    let world = world_with_two_pages();
    let path = format!("/machines?q={SEARCH_QUERY}&page=3");
    let reply = world.get_en(&world.operator, &path).await;
    let link = format!(
        "<a class=\"target-link\" href=\"/machines?q={SEARCH_QUERY}&#38;page=1\">Go to the first page</a>"
    );
    assert!(reply.body.contains(&link), "{}", reply.body);
}

#[tokio::test]
async fn machines_list_first_page_without_a_match_still_shows_the_empty_state() {
    let world = world();
    let reply = world.get_en(&world.operator, "/machines?q=zzz").await;
    assert!(reply.body.contains(&text(Lang::En, "machines-empty")));
    assert!(!reply.body.contains("after the last page"));
    assert!(!reply.body.contains("Go to the first page"));
}

/// Spec "Search text and pages of a list", scenario "Search text of spaces only" (U17).
#[tokio::test]
async fn machines_list_search_text_of_spaces_only_is_no_search_text() {
    let single = world();
    let reply = single
        .get_en(&single.operator, "/machines?q=%20%20%20")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("<a href=\"/machines/M7\">M7</a>"));
    let world = world_with_two_pages();
    for query in ["q=%20%20%20", "q=+", "q=%20"] {
        let path = format!("/machines?{query}");
        let reply = world.get_en(&world.operator, &path).await;
        assert_eq!(reply.status, StatusCode::OK, "{query}: {}", reply.body);
        assert_eq!(reply.body.matches("<td class=\"data\">").count(), 100);
        assert!(!reply.body.contains("class=\"empty\""), "{query}");
        assert!(
            reply
                .body
                .contains("href=\"/machines?page=2\">Next page</a>"),
            "{query}: the links keep no search text"
        );
        let form = search_form(&reply.body);
        assert!(
            form.contains("maxlength=\"200\" value=\"\""),
            "{query}: {form}"
        );
    }
}

/// Another search text keeps its rule: a control character and a text over 200 characters stay refused.
#[tokio::test]
async fn machines_list_search_text_with_spaces_and_a_letter_keeps_its_rule() {
    let world = world();
    let reply = world.get_en(&world.operator, "/machines?q=%20M7%20").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let bad = world.get_en(&world.operator, "/machines?q=%20%01").await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", bad.body);
}

// --- Malformed query ----------------------------------------------------------------------------

#[tokio::test]
async fn machines_list_rejects_a_bad_query_with_422_in_the_error_panel() {
    let world = world();
    let long_q = "a".repeat(201);
    let bad = [
        "page=0".to_owned(),
        "page=abc".to_owned(),
        "page=-1".to_owned(),
        "page=01".to_owned(),
        "page=99999999999".to_owned(),
        "show_archived=2".to_owned(),
        "show_archived=true".to_owned(),
        "show_archived=".to_owned(),
        "archived=X7".to_owned(),
        "archived=m7".to_owned(),
        "q=%01".to_owned(),
        format!("q={long_q}"),
        "other=1".to_owned(),
        "page=1&page=2".to_owned(),
    ];
    for query in bad {
        let path = format!("/machines?{query}");
        let headers = [ENGLISH, HTMX];
        let reply = world.get(&world.engineer, &path, &headers).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{query}");
        assert!(reply.body.contains("E-WEB-008"), "{query}: {}", reply.body);
        assert_eq!(reply.header("hx-retarget"), "#error-panel", "{query}");
    }
}

#[tokio::test]
async fn machines_list_names_the_field_and_the_reason_of_a_bad_value() {
    let world = world();
    let cases = [
        ("q=%01", "machine_q", "reason-search-text"),
        ("show_archived=2", "show_archived", "reason-flag"),
        ("archived=X7", "archived", "reason-machine-code"),
        ("page=0", "page", "reason-page"),
        ("other=1", "query", "reason-query"),
    ];
    for (query, field, reason) in cases {
        let path = format!("/machines?{query}");
        let reply = world.get(&world.engineer, &path, &[ENGLISH, HTMX]).await;
        let label = text(Lang::En, &format!("field-{}", field.replace('_', "-")));
        assert!(reply.body.contains(&label), "{query}: {}", reply.body);
        assert!(reply.body.contains(&text(Lang::En, reason)), "{query}");
    }
    let linked = world
        .get(&world.engineer, "/machines?q=%01", &[ENGLISH, HTMX])
        .await;
    assert!(
        linked.body.contains("href=\"#machine_q\""),
        "{}",
        linked.body
    );
}

// --- Archive message ----------------------------------------------------------------------------

#[tokio::test]
async fn machines_list_shows_the_archive_message_of_an_archived_machine_in_the_toast() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let path = "/machines?archived=M12";
        let reply = world.get(&world.engineer, path, &[cookie_of(lang)]).await;
        let bundles = Bundles::load().unwrap();
        let sentence = Texts::new(&bundles, lang).with_arg("machine-archived-toast", "code", "M12");
        let toast = format!(
            "<div id=\"toast\" role=\"status\" aria-live=\"polite\"><div class=\"toast\">{}",
            icon_tag("alert-success")
        );
        assert!(reply.body.contains(&toast), "{lang}: {}", reply.body);
        let expected = format!("<span>{sentence}</span>");
        assert!(reply.body.contains(&expected), "{lang}: {}", reply.body);
    }
}

#[tokio::test]
async fn machines_list_shows_no_archive_message_for_a_live_or_an_unknown_machine() {
    let world = world();
    for code in ["M7", "M99"] {
        let reply = world
            .get_en(&world.engineer, &format!("/machines?archived={code}"))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{code}");
        assert!(
            reply
                .body
                .contains("<div id=\"toast\" role=\"status\" aria-live=\"polite\"></div>"),
            "{code}: {}",
            reply.body
        );
    }
}

// --- Spec "Content answer of a page" ------------------------------------------------------------

#[tokio::test]
async fn machines_list_content_answer_has_the_title_the_heading_and_the_field_next() {
    let world = world();
    let reply = world
        .get_content(&world.engineer, "/machines?q=press")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<title>Machines · Recipe editor</title>")
    );
    assert!(
        reply
            .body
            .contains("<h1 tabindex=\"-1\" autofocus>Machines</h1>")
    );
    assert!(reply.body.contains("<div id=\"error-panel\"></div>"));
    assert!(reply.body.contains("id=\"toast\""));
    let next = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/machines?q=press\" hx-swap-oob=\"true\">";
    assert!(reply.body.contains(next), "{}", reply.body);
    for frame in ["class=\"topbar\"", "class=\"skip-link\"", "<main", "<html"] {
        assert!(!reply.body.contains(frame), "{frame}");
    }
    let target = [ENGLISH, HTMX, ("hx-target", "unit-tree")];
    let other = world.get(&world.engineer, "/machines", &target).await;
    assert!(other.body.contains("class=\"topbar\""));
}
