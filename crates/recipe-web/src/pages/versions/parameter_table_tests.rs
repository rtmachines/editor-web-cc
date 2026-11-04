//! Tests of the fragments `parameter-row.html` and `parameter-table.html` alone (task 9.4).
//!
//! Wave 5 renders them in the answers of `AddParameter` and `GET .../parameters` (design D7).

use askama::Template;

use recipe_domain::text::Text;

use super::parameter_table::{
    ParameterRowFragment, ParameterRowView, ShownName, ValueView, shown_name,
};
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, hx_pairs};
use crate::test_support::plant::{POLISH, main_of};

/// Parse a text of a name.
fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

/// Build the row of a parameter whose name holds markup. It has no translation, so the English
/// base text shows without a marker.
fn row() -> ParameterRowView {
    row_named(shown_name(
        Texts::new(&Bundles::load().unwrap(), Lang::En),
        None,
        &text("<b>Speed</b>"),
    ))
}

/// Build the row of a parameter with the shown name `name`.
fn row_named(name: ShownName) -> ParameterRowView {
    ParameterRowView {
        href: "/recipes/R-100/versions/3/parameters/73570007-0000-0000-0000-000000000011"
            .to_owned(),
        tag: "SPEED".to_owned(),
        name,
        type_text: "INT".to_owned(),
        value: ValueView {
            text: "1200".to_owned(),
            is_word: false,
            id: String::new(),
            lines: Vec::new(),
        },
        unit: "U1".to_owned(),
        address: "DB2.DBW0".to_owned(),
        address_id: String::new(),
        address_lines: Vec::new(),
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

#[tokio::test]
async fn parameter_table_header_names_the_machine_unit_in_english() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, "/recipes/R-100/versions/3")
        .await;
    let main = main_of(&reply.body);
    assert!(
        main.contains("<th scope=\"col\">Machine unit</th>"),
        "{main}"
    );
}

#[tokio::test]
async fn parameter_table_header_names_the_machine_unit_in_polish() {
    let world = recipe_world();
    let reply = world
        .get(&world.engineer, "/recipes/R-100/versions/3", &[POLISH])
        .await;
    let main = main_of(&reply.body);
    assert!(
        main.contains("<th scope=\"col\">Jednostka maszyny</th>"),
        "{main}"
    );
}

#[test]
fn shown_name_gives_the_translation_without_a_marker() {
    let bundles = Bundles::load().unwrap();
    let shown = shown_name(
        Texts::new(&bundles, Lang::Pl),
        Some(&text("Prędkość linii")),
        &text("Line speed"),
    );
    assert_eq!(shown.text, "Prędkość linii");
    assert_eq!(shown.marker, None);
}

#[test]
fn shown_name_marks_the_english_base_text_in_the_polish_ui() {
    let bundles = Bundles::load().unwrap();
    let shown = shown_name(Texts::new(&bundles, Lang::Pl), None, &text("Line speed"));
    assert_eq!(shown.text, "Line speed");
    assert_eq!(shown.marker.as_deref(), Some("(en)"));
}

#[test]
fn shown_name_gives_the_english_base_text_without_a_marker_in_the_english_ui() {
    let bundles = Bundles::load().unwrap();
    let shown = shown_name(Texts::new(&bundles, Lang::En), None, &text("Line speed"));
    assert_eq!(shown.text, "Line speed");
    assert_eq!(shown.marker, None);
}

#[test]
fn shown_name_escapes_a_translation_and_the_english_base_text() {
    let bundles = Bundles::load().unwrap();
    let t = Texts::new(&bundles, Lang::Pl);
    let translated = shown_name(t, Some(&text("<b>Speed</b>")), &text("Line speed"));
    let base = shown_name(t, None, &text("<b>Speed</b>"));
    let render = |name| {
        ParameterRowFragment {
            t,
            row: row_named(name),
            has_address: true,
            message: String::new(),
        }
        .render()
        .unwrap()
    };
    let html = render(translated);
    assert!(
        html.contains("<td>&#60;b&#62;Speed&#60;/b&#62;</td>"),
        "{html}"
    );
    let html = render(base);
    assert!(
        html.contains("<td><span lang=\"en\">&#60;b&#62;Speed&#60;/b&#62;</span> (en)</td>"),
        "{html}"
    );
}

#[test]
fn parameter_row_fragment_marks_the_english_name_in_the_polish_ui() {
    let bundles = Bundles::load().unwrap();
    let t = Texts::new(&bundles, Lang::Pl);
    let fragment = ParameterRowFragment {
        t,
        row: row_named(shown_name(t, None, &text("Line speed"))),
        has_address: true,
        message: String::new(),
    };
    let html = fragment.render().unwrap();
    assert!(
        html.contains("<td><span lang=\"en\">Line speed</span> (en)</td>"),
        "{html}"
    );
}
