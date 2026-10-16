//! Tests of the placeholders and the data of the recipe areas (design D12, task 8.3).
//!
//! `{ref}` is `R-100`, `{n}` is `3`, `{key}` is the key of the parameter `LINE`, and `{lang}` is
//! `pl`. `world` seeds the recipe data, so a row of a recipe area finds its resources.

use recipe_domain::plant::MachineId;
use recipe_domain::recipe::ParameterKey;
use uuid::Uuid;

use super::*;
use crate::test_support::recipes::{LINE_KEY, M20_ID};

/// The text of the key of `LINE` in a URL.
fn line_key_text() -> String {
    ParameterKey::from(LINE_KEY).to_string()
}

#[test]
fn matrix_fill_replaces_the_four_placeholders_of_the_recipe_routes() {
    assert_eq!(fill("/recipes/{ref}"), "/recipes/R-100");
    assert_eq!(
        fill("/recipes/{ref}/versions/{n}"),
        "/recipes/R-100/versions/3"
    );
    assert_eq!(
        fill("/recipes/{ref}/versions/{n}/parameters/{key}"),
        format!("/recipes/R-100/versions/3/parameters/{}", line_key_text())
    );
    assert_eq!(
        fill("/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}"),
        format!(
            "/recipes/R-100/versions/3/parameters/{}/translations/pl",
            line_key_text()
        )
    );
}

#[test]
fn matrix_fill_key_is_a_hyphenated_uuid_that_the_path_value_accepts() {
    let text = fill("{key}");
    assert_eq!(text.len(), 36);
    assert!(Uuid::parse_str(&text).is_ok(), "{text}");
}

#[test]
fn matrix_fill_replaces_a_recipe_placeholder_in_a_body_and_keeps_the_plant_ones() {
    assert_eq!(
        fill("next=/recipes/{ref}&code={code}"),
        "next=/recipes/R-100&code=M7"
    );
    assert_eq!(fill("{ref}/{ref}"), "R-100/R-100");
}

#[tokio::test]
async fn matrix_world_seeds_the_recipe_data_for_each_principal() {
    for principal in PRINCIPALS {
        let (fixture, _session) = world(principal).await;
        let m20 = fixture.store.state().machine(MachineId::from(M20_ID));
        assert!(
            m20.is_some_and(|machine| machine.is_live()),
            "{principal:?}"
        );
    }
}
