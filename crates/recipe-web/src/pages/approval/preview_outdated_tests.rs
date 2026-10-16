//! Tests of the outdated parameters on the approval preview: the list and the checkbox
//! `allow_outdated` in the form (task 10.1, ADR-0049 (b), design D3, spec "Outdated parameter").

use axum::http::StatusCode;

use super::preview_tests::part_of;
use crate::pages::audit::test_world::audit_world;
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::main_of;

/// The preview of the draft version 2 of `R-500`.
const R500_PREVIEW: &str = "/recipes/R-500/versions/2/approve";

#[tokio::test]
async fn approve_preview_lists_the_outdated_parameter_and_the_checkbox_in_the_form() {
    let world = audit_world().await;
    let anowak = world.data.anowak_user();
    let _rv = world
        .fixture
        .add_zone_to_r500(&anowak, world.data.r500_v2_row_version)
        .await;
    let reply = world.get_en(&world.data.admin, R500_PREVIEW).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    let form = part_of(main, "<form id=\"approve-fields\"", "</form>");
    let fieldset = part_of(form, "<fieldset>", "</fieldset>");
    assert!(
        fieldset.contains("<legend>Outdated parameters: 1</legend>"),
        "{fieldset}"
    );
    // The list keeps the gap of 8 px between the links (design-system.md section 7).
    let item = part_of(
        fieldset,
        "<ul class=\"link-list\"><li><a class=\"target-link\" href=\"/recipes/R-500/versions/2/parameters/",
        "</li>",
    );
    assert!(item.contains(">ZONE</a>"), "{item}");
    let checkbox = part_of(fieldset, "<input type=\"checkbox\"", ">");
    for literal in [
        "id=\"allow_outdated\"",
        "name=\"allow_outdated\"",
        "value=\"true\"",
        "aria-describedby=\"allow_outdated-hint\"",
    ] {
        assert!(checkbox.contains(literal), "{literal}: {checkbox}");
    }
    let label = part_of(fieldset, "<label class=\"check\">", "</label>");
    assert!(
        label.contains("<span>Approve with the outdated parameters</span>"),
        "{label}"
    );
    assert!(
        fieldset.contains(
            "<span class=\"field-hint\" id=\"allow_outdated-hint\">The audit log records this choice.</span>"
        ),
        "{fieldset}"
    );
}

/// `R-100` version 3 holds the outdated `ZONES` and a missing limit (design D3).
#[tokio::test]
async fn approve_preview_with_a_blocker_shows_no_outdated_list() {
    let world = audit_world().await;
    let reply = world
        .get_en(&world.data.jsmith, "/recipes/R-100/versions/3/approve")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("<h2 id=\"approve-blockers\">"), "{main}");
    assert!(!main.contains("Outdated parameters"), "{main}");
    assert!(!main.contains("allow_outdated"), "{main}");
    assert!(!main.contains("<fieldset>"), "{main}");
}
