//! Tests of the fragments `parameter-row.html` and `parameter-table.html` alone (task 9.4).
//!
//! Wave 5 renders them in the answers of `AddParameter` and `GET .../parameters` (design D7).

use askama::Template;

use super::parameter_table::{ParameterRowFragment, ParameterRowView};
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, hx_pairs};
use crate::test_support::plant::main_of;

/// Build the row of a parameter whose name holds markup.
fn row() -> ParameterRowView {
    ParameterRowView {
        href: "/recipes/R-100/versions/3/parameters/73570007-0000-0000-0000-000000000011"
            .to_owned(),
        tag: "SPEED".to_owned(),
        name: "<b>Speed</b>".to_owned(),
        type_text: "INT".to_owned(),
        unit: "U1".to_owned(),
        address: "DB2.DBW0".to_owned(),
        outdated: false,
    }
}

#[test]
fn parameter_row_fragment_starts_with_the_row_and_clears_the_form_and_the_error_panel() {
    let bundles = Bundles::load().unwrap();
    let fragment = ParameterRowFragment {
        t: Texts::new(&bundles, Lang::En),
        row: row(),
        has_address: true,
        message: "Parameter SPEED is added.".to_owned(),
    };
    let html = fragment.render().unwrap();
    assert!(
        html.starts_with("<tr><td class=\"data\"><a href=\"/recipes/R-100/versions/3/parameters/"),
        "{html}"
    );
    assert!(
        html.contains("<td>&#60;b&#62;Speed&#60;/b&#62;</td>"),
        "{html}"
    );
    assert!(html.contains("<td class=\"data\">DB2.DBW0</td>"), "{html}");
    assert!(
        html.contains("<div id=\"parameter-form\" hx-swap-oob=\"innerHTML\"></div>"),
        "{html}"
    );
    assert!(
        html.contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>"),
        "{html}"
    );
    assert!(html.contains("Parameter SPEED is added."), "{html}");
    let values: Vec<String> = hx_pairs(&html)
        .into_iter()
        .map(|(_name, value)| value)
        .collect();
    assert_eq!(
        values,
        ["true", "innerHTML", "innerHTML", "innerHTML"],
        "{html}"
    );
    assert_clean_html(&html);
}

/// Render the row fragment of `row()` with an address column.
fn fragment_html() -> String {
    let bundles = Bundles::load().unwrap();
    let fragment = ParameterRowFragment {
        t: Texts::new(&bundles, Lang::En),
        row: row(),
        has_address: true,
        message: "Parameter SPEED is added.".to_owned(),
    };
    fragment.render().unwrap()
}

#[test]
fn parameter_row_fragment_focuses_the_link_of_the_new_row() {
    let html = fragment_html();
    let link = "<a href=\"/recipes/R-100/versions/3/parameters/73570007-0000-0000-0000-000000000011\" autofocus>SPEED</a>";
    assert!(html.contains(link), "{html}");
    assert_eq!(html.matches("autofocus").count(), 1, "{html}");
}

#[test]
fn parameter_row_fragment_clears_the_empty_state_of_the_table() {
    let html = fragment_html();
    let empty = "<tbody id=\"parameter-empty\" hx-swap-oob=\"true\"></tbody>";
    assert!(html.contains(empty), "{html}");
}

#[tokio::test]
async fn parameter_table_of_the_version_page_focuses_no_row() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, "/recipes/R-100/versions/3")
        .await;
    let main = main_of(&reply.body);
    assert!(main.contains("\">SPEED</a></td>"), "{main}");
    assert!(!main.contains("autofocus"), "{main}");
}

#[test]
fn parameter_row_fragment_without_addresses_has_no_address_cell() {
    let bundles = Bundles::load().unwrap();
    let fragment = ParameterRowFragment {
        t: Texts::new(&bundles, Lang::En),
        row: row(),
        has_address: false,
        message: String::new(),
    };
    let html = fragment.render().unwrap();
    assert!(!html.contains("DB2.DBW0"), "{html}");
}
