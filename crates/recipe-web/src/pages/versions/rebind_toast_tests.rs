//! Test of the change number in the toast of a rebind (task 4.5, design D4, ADR-0108 (f)).

use super::rebind_world::{REBIND, body_v3};
use crate::pages::draft_reply::toast_number_tests::assert_toast_number;
use crate::pages::recipes::test_world::recipe_world;

/// The four elements of `ZONES`.
const ALL: [u32; 4] = [1, 2, 3, 4];

#[tokio::test]
async fn toast_of_a_rebind_names_the_change_number() {
    let world = recipe_world();
    let body = body_v3(&world, "1", &ALL, "12.5");
    let reply = world.post(&world.engineer, REBIND, &body, true).await;
    assert_toast_number(&reply, "The parameters use ST_Zone@3.", 2);
}
