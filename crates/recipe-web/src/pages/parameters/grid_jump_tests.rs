//! Tests of the jump link after the leaf grid of the parameter forms (design D7).

use crate::pages::parameters::test_kit::{
    ADD, INT_FIELDS, changed, parameter_path, position, post_htmx,
};
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::plant::main_of;
use crate::test_support::recipes::SPEED_KEY;

/// The jump link to the save button after the leaf grid of an edit form (design D7).
const SAVE_JUMP: &str = "<a class=\"target-link\" href=\"#parameter-save-submit\">";
/// The jump link to the add button after the leaf grid of a new-parameter form (design D7).
const ADD_JUMP: &str = "<a class=\"target-link\" href=\"#parameter-add-submit\">";

/// Assert that `form` holds `link` once, after its last input and so after the leaf grid.
fn assert_jump_after_grid(form: &str, link: &str) {
    assert_eq!(form.matches(link).count(), 1, "{form}");
    let last_input = form.rfind("<input").unwrap();
    assert!(last_input < position(form, link), "{form}");
}

#[tokio::test]
async fn parameter_edit_form_holds_a_plain_jump_link_after_the_leaf_grid() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    let main = main_of(&reply.body);
    let form = main
        .split("<form id=\"parameter-edit-form\"")
        .nth(1)
        .unwrap()
        .split("</form>")
        .next()
        .unwrap();
    assert_jump_after_grid(form, SAVE_JUMP);
}

#[tokio::test]
async fn parameter_add_form_holds_a_plain_jump_link_after_the_leaf_grid() {
    let world = recipe_world();
    let pairs = changed(&INT_FIELDS, &[("value", Some("abc"))]);
    let reply = post_htmx(&world, ADD, &pairs).await;
    let form = reply
        .body
        .split("<form id=\"parameter-new-form\"")
        .nth(1)
        .unwrap()
        .split("</form>")
        .next()
        .unwrap();
    assert_jump_after_grid(form, ADD_JUMP);
}
