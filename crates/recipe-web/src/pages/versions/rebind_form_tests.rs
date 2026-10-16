//! Tests of the rebind form on the version page (task 12.2).
//!
//! Spec "Rebind form of an outdated draft". ADR-0043 (l), ADR-0039 (f), design D4, D6, and D7.

use axum::http::StatusCode;
use recipe_app::memory::JournalEntry;
use recipe_app::ports::Isolation;
use recipe_domain::plant::Primitive;

use super::outdated_tests::outdated_of;
use super::rebind_world::{
    DRAFT_PAGE, ZONE_ONE_KEY, bounded_world, form_id, seed_big, seed_gate, seed_zone_one,
    seed_zone_v4, zone_v3,
};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, assert_no_user_text_in_hx_attributes};
use crate::test_support::icon_tag;
use crate::test_support::plant::{POLISH, World};
use crate::test_support::recipes::{LINE_KEY, ZONES_KEY};

/// Return the start tag of the rebind form `id` of the draft of `R-100`.
fn form_start(id: &str) -> String {
    format!(
        "<form id=\"{id}\" method=\"post\" action=\"/recipes/R-100/versions/3/rebind\" hx-boost=\"true\" hx-target=\"#parameter-table\" hx-swap=\"innerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\">"
    )
}

#[tokio::test]
async fn rebind_form_holds_the_type_ids_and_one_input_per_new_member_leaf() {
    let world = recipe_world();
    let zone_v3 = zone_v3(&world);
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let list = outdated_of(&reply.body);
    let id = form_id(world.ids.zone_v2);
    assert!(list.contains(&form_start(&id)), "{list}");
    assert_eq!(list.matches("<form").count(), 1, "{list}");
    assert!(list.contains("Rebind ST_Zone@2 to ST_Zone@3"), "{list}");
    let hidden = [
        ("row_version", "1".to_owned()),
        ("from_type_id", world.ids.zone_v2.to_string()),
        ("to_type_id", zone_v3.to_string()),
    ];
    for (name, value) in hidden {
        let field = format!("<input type=\"hidden\" name=\"{name}\" value=\"{value}\">");
        assert!(list.contains(&field), "{name}: {list}");
    }
    for n in 0..4_u32 {
        let index = n.checked_add(1).unwrap();
        let name = format!("name=\"new_member_values[{ZONES_KEY}][{index}].temp_min\"");
        let input = format!("<input id=\"new-0-{n}\" {name}");
        assert!(list.contains(&input), "{input}: {list}");
        let label = format!("<label for=\"new-0-{n}\">New member [{index}].temp_min of ZONES");
        assert!(list.contains(&label), "{label}: {list}");
    }
    assert!(!list.contains("new-0-4"), "{list}");
    assert!(
        !list.contains(&format!("new_member_values[{LINE_KEY}]")),
        "LINE holds ST_Zone@2 as a member, so the rebind does not move it: {list}"
    );
    let button = format!("<button type=\"submit\" id=\"{id}-submit\"");
    assert!(list.contains(&button), "{list}");
    assert!(list.contains("Rebind parameters</button>"), "{list}");
    assert_clean_html(&reply.body);
    assert_no_user_text_in_hx_attributes(&reply.body, "ZONES");
}

#[tokio::test]
async fn rebind_form_numbers_the_parameters_of_the_form_in_the_order_of_the_list() {
    let world = recipe_world();
    seed_zone_v4(
        &world,
        &[("speed", Primitive::Int), ("temp_min", Primitive::Real)],
    );
    seed_zone_one(&world);
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    let list = outdated_of(&reply.body);
    assert!(list.contains("Rebind ST_Zone@2 to ST_Zone@4"), "{list}");
    let one = format!("<input id=\"new-0-0\" name=\"new_member_values[{ZONE_ONE_KEY}].temp_min\"");
    assert!(list.contains(&one), "{list}");
    let zones =
        format!("<input id=\"new-1-0\" name=\"new_member_values[{ZONES_KEY}][1].temp_min\"");
    assert!(list.contains(&zones), "{list}");
    assert!(
        list.contains("<fieldset id=\"rebind-parameter-0\">"),
        "{list}"
    );
    assert!(list.contains("<legend>ZONE_ONE</legend>"), "{list}");
}

#[tokio::test]
async fn rebind_form_is_not_shown_to_an_operator_or_on_a_version_that_is_not_a_draft() {
    let world = recipe_world();
    let operator = world.get_en(&world.operator, DRAFT_PAGE).await;
    assert!(
        !outdated_of(&operator.body).contains("<form"),
        "{}",
        operator.body
    );
    let approved = world
        .get_en(&world.engineer, "/recipes/R-100/versions/2")
        .await;
    assert!(
        !outdated_of(&approved.body).contains("<form"),
        "{}",
        approved.body
    );
}

/// Return the read transactions of the queries that `GET` of the draft page opens, as the engineer.
///
/// A query reads in `RepeatableRead` or `ReadOnly`. The session check opens `ReadCommitted`, so the
/// function leaves it out.
async fn page_transactions(world: &World) -> Vec<JournalEntry> {
    let before = world.fixture.store.journal().len();
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    world
        .fixture
        .store
        .journal()
        .into_iter()
        .skip(before)
        .filter(|entry| {
            matches!(
                entry,
                JournalEntry::Begin(Isolation::RepeatableRead | Isolation::ReadOnly)
            )
        })
        .collect()
}

#[tokio::test]
async fn rebind_form_page_reads_two_snapshots_for_any_count_of_outdated_parameters() {
    let world = recipe_world();
    seed_zone_v4(
        &world,
        &[("speed", Primitive::Int), ("temp_min", Primitive::Real)],
    );
    seed_zone_one(&world);
    seed_gate(&world);
    let snapshot = JournalEntry::Begin(Isolation::RepeatableRead);
    let found = page_transactions(&world).await;
    assert_eq!(
        found,
        [snapshot.clone(), snapshot],
        "GetVersion and ListParameters only"
    );
}

#[tokio::test]
async fn rebind_form_is_not_shown_for_a_rebind_above_the_leaf_limit() {
    let world = recipe_world();
    let (big_one, _big_two) = seed_big(&world);
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let list = outdated_of(&reply.body);
    assert!(list.contains("\">BIG</a>"), "{list}");
    assert!(!list.contains(&form_id(big_one)), "{list}");
}

/// Assert that `list` holds a rebind form with no field of a new member and no submit button.
fn assert_no_rebind_field(list: &str) {
    assert!(!list.contains("id=\"new-"), "{list}");
    assert!(!list.contains("new_member_values"), "{list}");
    assert!(!list.contains("type=\"submit\""), "{list}");
    assert!(!list.contains("<input"), "{list}");
}

#[tokio::test]
async fn rebind_forms_over_the_bound_show_the_notice_and_no_field_in_en_and_pl() {
    // ZONES needs 4 new leaves: `.temp_min` of each of its 4 elements. The bound is 3.
    let world = bounded_world(3);
    let english = world.get_en(&world.engineer, DRAFT_PAGE).await;
    let polish = world.get(&world.engineer, DRAFT_PAGE, &[POLISH]).await;
    assert_eq!(english.status, StatusCode::OK, "{}", english.body);
    assert_eq!(polish.status, StatusCode::OK, "{}", polish.body);
    let notices = [
        (
            &english,
            "The rebinds of this draft need at least 4 new values. One page shows at most 3 new values.",
        ),
        (
            &polish,
            "Przepięcia tej wersji roboczej wymagają co najmniej 4 nowych wartości. Jedna strona pokazuje najwyżej 3 nowych wartości.",
        ),
    ];
    for (reply, notice) in notices {
        let list = outdated_of(&reply.body);
        assert!(list.contains(&form_id(world.ids.zone_v2)), "{list}");
        assert_eq!(list.matches("<form").count(), 1, "{list}");
        assert!(list.contains(&format!("<span>{notice}</span>")), "{list}");
        assert!(list.contains(&icon_tag("alert-warning")), "{list}");
        assert_no_rebind_field(list);
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn rebind_forms_at_the_bound_keep_their_fields() {
    let world = bounded_world(4);
    let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;
    let list = outdated_of(&reply.body);
    assert!(list.contains("<input id=\"new-0-3\""), "{list}");
    assert!(list.contains("type=\"submit\""), "{list}");
    assert!(!list.contains("One page shows at most"), "{list}");
}

#[tokio::test]
async fn rebind_forms_over_the_bound_are_not_shown_to_an_operator() {
    let world = bounded_world(3);
    let reply = world.get_en(&world.operator, DRAFT_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let list = outdated_of(&reply.body);
    assert!(list.contains("\">ZONES</a>"), "{list}");
    assert!(!list.contains("<form"), "{list}");
    assert!(!list.contains("One page shows at most"), "{list}");
}
