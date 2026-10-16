//! Tests of the swap of the in-page parameter forms (RQ-2, ADR-0037 (f)).
//!
//! A boosted form scrolls to its target after a save, because HTMX sets `scrollIntoViewOnBoost`.
//! A save from deep in the leaf grid then moves the focused input far out of view. The edit form
//! and the in-page form of a new parameter therefore hold the literal modifiers `show:none` and
//! `focus-scroll:true`, so the view stays and the focused input stays in view.

use super::super::test_kit::{NEW_FORM, get_form_fragment, parameter_path};
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::recipes::SPEED_KEY;

/// The swap of the edit form, which swaps `#parameter-summary`.
const EDIT_SWAP: &str =
    "hx-target=\"#parameter-summary\" hx-swap=\"innerHTML show:none focus-scroll:true\"";
/// The swap of the in-page form of a new parameter, which appends to `#parameter-rows`.
const ADD_SWAP: &str =
    "hx-target=\"#parameter-rows\" hx-swap=\"beforeend show:none focus-scroll:true\"";

#[tokio::test]
async fn parameter_forms_the_edit_form_keeps_the_view_after_a_save() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    let (_before, form) = reply
        .body
        .split_once("<form id=\"parameter-edit-form\"")
        .unwrap();
    let first_tag = form.split('>').next().unwrap();
    assert!(first_tag.contains(EDIT_SWAP), "{first_tag}");
}

#[tokio::test]
async fn parameter_forms_the_in_page_form_of_a_new_parameter_keeps_the_view_after_a_save() {
    let world = recipe_world();
    let reply = get_form_fragment(&world, NEW_FORM).await;
    let (_before, form) = reply
        .body
        .split_once("<form id=\"parameter-new-form\"")
        .unwrap();
    let first_tag = form.split('>').next().unwrap();
    assert!(first_tag.contains(ADD_SWAP), "{first_tag}");
}
