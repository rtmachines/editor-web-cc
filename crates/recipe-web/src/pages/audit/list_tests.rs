//! Tests of `GET /audit` (task 11.3, ADR-0051 (c) to (e) and (i), specs "Audit list page" and
//! "Filter values of the audit list page").

use axum::http::StatusCode;
use recipe_app::blocking::{AppSettings, PageSizes};
use recipe_domain::audit::SecurityEventKind;

use super::test_world::{
    AuditWorld, TIME_CELL, admin_main, assert_object_cells_mark_the_label, audit_world,
    audit_world_with, body_rows, entry_ids, older_href, query_pairs,
};
use crate::pages::test_helpers::{assert_clean_html, assert_error, assert_htmx_error, hx_pairs};
use crate::test_support::get_with;
use crate::test_support::plant::{HTMX, main_of};

/// Return the markup of the filter form of the page.
fn filter_form(main: &str) -> &str {
    let (_before, rest) = main
        .split_once("<form method=\"get\" action=\"/audit\"")
        .unwrap();
    rest.split("</form>").next().unwrap()
}

/// Send `GET path` as an HTMX request that targets `main`, in English.
async fn htmx_get(
    world: &AuditWorld,
    session: &crate::test_support::TestSession,
    path: &str,
) -> crate::test_support::Reply {
    let cookie = format!("{}; recipe_lang=en", session.cookie);
    get_with(
        &world.fixture.router(),
        path,
        &[("cookie", cookie.as_str()), HTMX],
    )
    .await
}

#[tokio::test]
async fn audit_list_gives_403_and_one_forbidden_event_to_an_operator() {
    let world = audit_world().await;
    let before = world.fixture.sink.events().len();
    let reply = world.get_en(&world.operator, "/audit").await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", "operator");
    let events = world.fixture.sink.events();
    assert_eq!(events.len(), before.checked_add(1).unwrap());
    assert_eq!(
        events.last().map(|event| event.kind()),
        Some(SecurityEventKind::Forbidden)
    );
}

#[tokio::test]
async fn audit_list_of_an_administrator_shows_the_filter_form_the_rows_and_the_security_link() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.admin, "/audit").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("<h1>Audit log</h1>"), "{main}");
    assert!(body_rows(main) >= 10, "{main}");
    assert!(main.contains("href=\"/audit/"), "{main}");
    assert!(
        main.contains("href=\"/security-events\">Security events</a>"),
        "{main}"
    );
    assert!(main.contains("No older entries."), "{main}");
    let form = filter_form(main);
    assert!(!form.contains("csrf_token"), "{form}");
    assert!(!form.contains("name=\"before\""), "{form}");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn audit_list_the_time_cell_of_each_row_holds_the_time_span_that_does_not_break() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.admin, "/audit").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let (_head, table) = main.split_once("<tbody>").unwrap();
    let rows = table.matches("<tr>").count();
    assert!(rows >= 10, "{main}");
    assert_eq!(table.matches(TIME_CELL).count(), rows, "{main}");
}

#[tokio::test]
async fn audit_list_filter_form_is_a_boosted_get_form_with_five_literal_attributes() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.admin, "/audit").await;
    let main = main_of(&reply.body);
    let (_before, rest) = main
        .split_once("<form method=\"get\" action=\"/audit\"")
        .unwrap();
    let tag = rest.split('>').next().unwrap();
    assert!(tag.contains("hx-boost=\"true\""), "{tag}");
    assert!(tag.contains("hx-target=\"#content\""), "{tag}");
    assert!(tag.contains("hx-push-url=\"true\""), "{tag}");
    assert_eq!(tag.matches(" hx-").count(), 3, "{tag}");
}

#[tokio::test]
async fn audit_list_selects_offer_all_values_to_an_administrator_and_the_recipe_scope_to_an_engineer()
 {
    let world = audit_world().await;
    let admin = world.get_en(&world.data.admin, "/audit").await;
    let form = filter_form(main_of(&admin.body));
    // Every type, 8 types, every action, 35 actions.
    assert_eq!(form.matches("<option value=\"").count(), 45, "{form}");
    assert_eq!(form.matches("<optgroup label=\"").count(), 5, "{form}");
    assert!(
        form.contains("<option value=\"plc_type\">PLC type</option>"),
        "{form}"
    );
    let engineer = world.get_en(&world.data.anowak, "/audit").await;
    let main = main_of(&engineer.body);
    let form = filter_form(main);
    assert!(
        form.contains("<option value=\"recipe_version\">Version</option>"),
        "{form}"
    );
    assert!(!form.contains("value=\"machine\""), "{form}");
    assert!(!form.contains("value=\"app_user\""), "{form}");
    assert!(form.contains("value=\"VersionApproved\""), "{form}");
    assert!(!form.contains("value=\"MachineCreated\""), "{form}");
    assert!(!form.contains("value=\"UserCreated\""), "{form}");
    assert!(!main.contains("/security-events"), "{main}");
    assert!(!main.contains("Machine "), "{main}");
}

#[tokio::test]
async fn audit_list_filters_by_action_and_by_entity_type() {
    let world = audit_world().await;
    let all = world.get_en(&world.data.admin, "/audit").await;
    let total = body_rows(main_of(&all.body));
    let approved = world
        .get_en(&world.data.admin, "/audit?action=VersionApproved")
        .await;
    let main = main_of(&approved.body);
    let rows = body_rows(main);
    assert!((1..total).contains(&rows), "{rows} of {total}");
    assert_eq!(
        main.matches("<td>Version approved</td>").count(),
        rows,
        "{main}"
    );
    let typed = world
        .get_en(&world.data.admin, "/audit?entity_type=recipe_version")
        .await;
    let typed_rows = body_rows(main_of(&typed.body));
    assert!((1..total).contains(&typed_rows), "{typed_rows}");
}

#[tokio::test]
async fn audit_list_names_the_aggregate_of_the_filter_and_offers_the_link_that_clears_it() {
    let world = audit_world().await;
    let id = world.data.r500.as_uuid();
    let reply = world
        .get_en(&world.data.admin, &format!("/audit?aggregate={id}"))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("Aggregate: Recipe R-500"), "{main}");
    assert!(
        main.contains(&format!(
            "<input type=\"hidden\" name=\"aggregate\" value=\"{id}\">"
        )),
        "{main}"
    );
    assert!(main.contains("Show every aggregate"), "{main}");
    assert!(body_rows(main) >= 5, "{main}");
}

#[tokio::test]
async fn audit_list_object_cell_shows_the_label_in_mono_and_the_type_word_outside_it() {
    let world = audit_world().await;
    let id = world.data.r500.as_uuid();
    for (code, recipe, version) in [("en", "Recipe", "Version"), ("pl", "Receptura", "Wersja")] {
        let reply = world
            .get(&world.data.admin, code, &format!("/audit?aggregate={id}"))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_object_cells_mark_the_label(main_of(&reply.body), recipe, version);
    }
}

#[tokio::test]
async fn audit_list_names_an_aggregate_without_rows_by_its_short_id() {
    let world = audit_world().await;
    let reply = world
        .get_en(
            &world.data.admin,
            "/audit?aggregate=12345678-0000-0000-0000-000000000000",
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("<p>Aggregate: 12345678 <a "), "{main}");
    assert!(main.contains("No entry matches the filters."), "{main}");
}

#[tokio::test]
async fn audit_list_names_an_aggregate_without_rows_in_polish_with_one_space() {
    let world = audit_world().await;
    let reply = world
        .get(
            &world.data.admin,
            "pl",
            "/audit?aggregate=12345678-0000-0000-0000-000000000000",
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("<p>Agregat: 12345678 <a "), "{main}");
}

#[tokio::test]
async fn audit_list_keeps_the_entity_filter_as_hidden_fields() {
    let world = audit_world().await;
    let id = world.data.r500.as_uuid();
    let path = format!("/audit?entity_type=recipe&entity_id={id}");
    let reply = world.get_en(&world.data.admin, &path).await;
    let main = main_of(&reply.body);
    assert!(main.contains("Object: Recipe "), "{main}");
    assert!(main.contains("Show every object"), "{main}");
    let form = filter_form(main);
    assert!(form.contains("<input type=\"hidden\" name=\"entity_type\" value=\"recipe\">"));
    assert!(form.contains(&format!(
        "<input type=\"hidden\" name=\"entity_id\" value=\"{id}\">"
    )));
    assert!(!form.contains("<select id=\"entity_type\""), "{form}");
    assert!(body_rows(main) >= 1, "{main}");
}

#[tokio::test]
async fn audit_list_actor_filter_keeps_the_rows_of_one_login_and_an_unknown_actor_gives_none() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.admin, "/audit?actor=anowak").await;
    let main = main_of(&reply.body);
    let rows = body_rows(main);
    assert!(rows >= 1, "{main}");
    assert_eq!(
        main.matches("</span> <a class=\"target-link\"").count(),
        rows
    );
    assert_eq!(
        main.matches("<td class=\"data\">anowak</td>").count(),
        rows,
        "{main}"
    );
    let ghost = world.get_en(&world.data.admin, "/audit?actor=ghost").await;
    assert_eq!(ghost.status, StatusCode::OK, "{}", ghost.body);
    let main = main_of(&ghost.body);
    assert_eq!(body_rows(main), 0, "{main}");
    assert!(main.contains("No entry matches the filters."), "{main}");
}

#[tokio::test]
async fn audit_list_date_filters_use_whole_days_in_utc_and_the_last_day_sets_no_bound() {
    let world = audit_world().await;
    let all = body_rows(main_of(
        &world.get_en(&world.data.admin, "/audit").await.body,
    ));
    assert!(all > 0, "the unfiltered list has rows");
    // The clock of the tests starts on 1970-01-12.
    for (query, want) in [
        ("from=1970-01-12&to=1970-01-12", all),
        ("to=9999-12-31", all),
        ("from=0001-01-01", all),
        ("to=1970-01-11", 0),
        ("from=1970-01-13", 0),
    ] {
        let path = format!("/audit?{query}");
        let reply = world.get_en(&world.data.admin, &path).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
        assert_eq!(body_rows(main_of(&reply.body)), want, "{path}");
    }
}

#[tokio::test]
async fn audit_list_malformed_values_give_422_with_every_field_label() {
    let world = audit_world().await;
    let path = "/audit?from=2026-13-01&to=2026-02-30&action=Other&entity_type=other&aggregate=x&actor=A&before=abc";
    let reply = htmx_get(&world, &world.data.admin, path).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(reply.header("hx-push-url"), "false");
    for label in [
        "From",
        "To",
        "Action",
        "Object type",
        "Aggregate",
        "User login",
        "Position in the list",
    ] {
        assert!(reply.body.contains(label), "{label}: {}", reply.body);
    }
    assert!(!reply.body.contains("2026-13-01"), "{}", reply.body);
}

#[tokio::test]
async fn audit_list_an_htmx_422_links_each_visible_filter_field_to_its_input() {
    let world = audit_world().await;
    let path = "/audit?from=bad&to=bad&action=Other&entity_type=other&actor=A&before=abc";
    let reply = htmx_get(&world, &world.data.admin, path).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    for field in ["from", "to", "action", "entity_type", "actor"] {
        assert!(
            reply.body.contains(&format!("href=\"#{field}\"")),
            "{field}: {}",
            reply.body
        );
    }
    assert!(!reply.body.contains("href=\"#before\""), "{}", reply.body);
}

#[tokio::test]
async fn audit_list_each_visible_filter_input_has_the_id_of_its_field_name() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.admin, "/audit").await;
    let form = filter_form(main_of(&reply.body));
    for field in ["entity_type", "action", "actor", "from", "to"] {
        assert!(form.contains(&format!("id=\"{field}\"")), "{field}: {form}");
        assert!(
            form.contains(&format!("name=\"{field}\"")),
            "{field}: {form}"
        );
    }
}

#[tokio::test]
async fn audit_list_other_bad_queries_give_422() {
    let world = audit_world().await;
    let id = world.data.r500.as_uuid();
    for path in [
        "/audit?x=1".to_owned(),
        format!("/audit?entity_id={id}"),
        "/audit?from=2026-10-02&to=2026-10-01".to_owned(),
        "/audit?from=0000-01-01".to_owned(),
        "/audit?from=2026-1-1".to_owned(),
    ] {
        let reply = world.get_en(&world.data.admin, &path).await;
        assert_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008", &path);
    }
}

/// The English reason of a bad identifier (design D8).
const UUID_REASON: &str = "an identifier has 36 characters in the form 8-4-4-4-12 of the digits 0 to 9 and the letters a to f";
/// The English reason of a bad cursor `before` (design D8).
const CURSOR_REASON: &str = "the link of the page is not valid. Open the first page of the list";

/// Return the English line of a bad filter value: the field label and the reason of design D8.
fn reason_line(label: &str, reason: &str) -> String {
    format!("Field {label} is not valid: {reason}.")
}

#[tokio::test]
async fn audit_list_each_bad_filter_value_gives_the_reason_of_its_input() {
    let world = audit_world().await;
    let only_id = format!("entity_id={}", world.data.r500.as_uuid());
    let cases = [
        (
            "from=2026-13-01",
            "From",
            "a date has the form YYYY-MM-DD, for example 2026-10-17",
        ),
        (
            "from=2026-10-02&to=2026-10-01",
            "To",
            "the end date is the same as the start date or later",
        ),
        (
            "entity_type=other",
            "Object type",
            "choose a value from the list",
        ),
        ("action=Other", "Action", "choose a value from the list"),
        ("aggregate=x", "Aggregate", UUID_REASON),
        ("entity_type=recipe&entity_id=x", "Object", UUID_REASON),
        (
            only_id.as_str(),
            "Object",
            "choose an object type for the object",
        ),
        ("before=abc", "Position in the list", CURSOR_REASON),
    ];
    for (query, label, reason) in cases {
        let path = format!("/audit?{query}");
        let reply = htmx_get(&world, &world.data.admin, &path).await;
        assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
        let expected = reason_line(label, reason);
        assert!(
            reply.body.contains(&expected),
            "{path}: {expected}: {}",
            reply.body
        );
    }
}

#[tokio::test]
async fn audit_list_a_bad_date_gives_the_polish_reason_with_its_field_label() {
    let world = audit_world().await;
    let cookie = format!("{}; recipe_lang=pl", world.data.admin.cookie);
    let reply = get_with(
        &world.fixture.router(),
        "/audit?from=2026-13-01",
        &[("cookie", cookie.as_str()), HTMX],
    )
    .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    let expected = "Pole Od jest niepoprawne: data ma postać RRRR-MM-DD, na przykład 2026-10-17.";
    assert!(reply.body.contains(expected), "{}", reply.body);
}

#[tokio::test]
async fn audit_list_empty_fields_mean_no_filter() {
    let world = audit_world().await;
    let all = body_rows(main_of(
        &world.get_en(&world.data.admin, "/audit").await.body,
    ));
    assert!(all > 0, "the unfiltered list has rows");
    let path = "/audit?aggregate=&entity_type=&entity_id=&actor=&action=&from=&to=&before=";
    let reply = world.get_en(&world.data.admin, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(body_rows(main_of(&reply.body)), all);
}

#[tokio::test]
async fn audit_list_cursor_pager_ends_at_the_oldest_entry() {
    let world = audit_world().await;
    let reply = world.get_en(&world.data.admin, "/audit?before=1").await;
    let main = main_of(&reply.body);
    assert_eq!(body_rows(main), 0, "{main}");
    assert!(
        main.contains("href=\"/audit\">Newest entries</a>"),
        "{main}"
    );
    assert!(main.contains("No older entries."), "{main}");
}

/// Build the world with the approval data and `rows` rows on one page of the audit list.
async fn world_with_audit_page_of(rows: u32) -> AuditWorld {
    let defaults = AppSettings::default().pages;
    audit_world_with(PageSizes {
        audit: rows,
        ..defaults
    })
    .await
}

#[tokio::test]
async fn audit_list_world_with_a_small_page_seeds_the_creator_of_the_versions() {
    let world = world_with_audit_page_of(2).await;

    let reply = world
        .get_en(&world.data.admin, "/recipes/R-200/versions/1")
        .await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

/// Return the address of the first page of the paging test.
/// It sets six filters of the audit list and leaves the filter `entity_id` empty.
/// The two dates keep every entry, because the clock of the tests starts on 1970-01-12.
fn filtered_path(world: &AuditWorld) -> String {
    format!(
        "/audit?aggregate={}&entity_type=parameter&entity_id=&actor=anowak&action=ParameterAdded&from=1970-01-12&to=1970-01-12",
        world.data.r500.as_uuid()
    )
}

/// The link to the older page keeps the filters, and the older page holds the next older entries
/// (ADR-0051 (i), `design-system.md` section 10.12, finding CX-RCW-2).
#[tokio::test]
async fn audit_list_older_link_keeps_the_filters_and_opens_the_next_older_entries() {
    // The page of 100 rows holds every entry of the filters. It is the order that the pages keep.
    let full = audit_world().await;
    let all = entry_ids(&admin_main(&full, &filtered_path(&full)).await);
    assert!(all.len() >= 4, "{all:?}");
    let (newer, rest) = all.split_at(2);
    let (older, _rest) = rest.split_at(2);
    let world = world_with_audit_page_of(2).await;
    let first = admin_main(&world, &filtered_path(&world)).await;
    assert_eq!(entry_ids(&first), newer, "{first}");
    let href = older_href(&first);
    let mut want = vec![
        ("action".to_owned(), "ParameterAdded".to_owned()),
        ("actor".to_owned(), "anowak".to_owned()),
        (
            "aggregate".to_owned(),
            world.data.r500.as_uuid().to_string(),
        ),
        ("entity_type".to_owned(), "parameter".to_owned()),
        ("from".to_owned(), "1970-01-12".to_owned()),
        ("to".to_owned(), "1970-01-12".to_owned()),
        ("before".to_owned(), newer.last().unwrap().to_string()),
    ];
    want.sort();
    assert!(href.starts_with("/audit?"), "{href}");
    assert_eq!(query_pairs(&href), want, "{href}");
    let second = admin_main(&world, &href).await;
    assert_eq!(entry_ids(&second), older, "{second}");
    let by_anowak = second.matches("<td class=\"data\">anowak</td>").count();
    assert_eq!(by_anowak, older.len(), "{second}");
    let (first_ids, second_ids) = (entry_ids(&first), entry_ids(&second));
    assert!(
        second_ids.iter().all(|id| !first_ids.contains(id)),
        "{first_ids:?} {second_ids:?}"
    );
    assert!(newer.last() > older.first(), "{newer:?} {older:?}");
}

#[tokio::test]
async fn audit_list_content_answer_has_literal_hx_attributes_without_user_data() {
    let world = audit_world().await;
    let id = world.data.r500.as_uuid();
    let path = format!("/audit?aggregate={id}&actor=anowak");
    let cookie = format!("{}; recipe_lang=en", world.data.admin.cookie);
    let reply = get_with(
        &world.fixture.router(),
        &path,
        &[
            ("cookie", cookie.as_str()),
            HTMX,
            crate::test_support::plant::TARGET_CONTENT,
        ],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<title>Audit log · Recipe editor</title>")
    );
    for (name, value) in hx_pairs(&reply.body) {
        assert!(
            !value.contains("anowak") && !value.contains(&id.to_string()),
            "{name}={value}"
        );
    }
}

#[tokio::test]
async fn audit_list_words_come_from_the_polish_bundle() {
    let world = audit_world().await;
    let reply = world.get(&world.data.admin, "pl", "/audit").await;
    let main = main_of(&reply.body);
    assert!(main.contains("<h1>Dziennik audytu</h1>"), "{main}");
    assert!(main.contains("Zatwierdzono wersję"), "{main}");
    assert!(main.contains("<optgroup label=\"Receptury\">"), "{main}");
    assert!(!main.contains("Version approved"), "{main}");
}
