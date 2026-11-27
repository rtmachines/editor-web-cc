//! The matrix rows and the sweep pages of the parameter pages (design D12).
//!
//! `routes::tests` and `pages::sweep_tests` collect both lists. The agents of wave 5 add the rows
//! of the parameter routes here. The placeholder `{key}` names the parameter `LINE` of `R-100`.

use axum::http::Method;

use crate::routes::matrix::{Endpoint, Want, endpoint};

/// The body of a valid `AddParameter` on the draft 3 of `R-100`. The draft has the `row_version` 1.
const ADD_BODY: &str = "row_version=1&unit_path=U1&tag=MATRIX_P&name_en=Matrix&description_en=Matrix+row&type=INT&value_type=INT&s7_address=DB9.DBW0&ordinal=0&value=1";
/// The body of `SetParameterValue` for `LINE` (`ST_Line@1`): four zones with the value 0 in each leaf.
const VALUE_BODY: &str = "row_version=1&value.zones%5B1%5D.speed=0&value.zones%5B1%5D.temp_max=0&value.zones%5B2%5D.speed=0&value.zones%5B2%5D.temp_max=0&value.zones%5B3%5D.speed=0&value.zones%5B3%5D.temp_max=0&value.zones%5B4%5D.speed=0&value.zones%5B4%5D.temp_max=0";
/// The body of a valid save of the edit form of `LINE` without a change.
///
/// It sends the value fields and the limit rows as a pair (ADR-0213 (i)). The limit rows are blank.
/// The rows have the indexes 1 and 2, so the XSS test can add the row 0 without a duplicate key.
const DEFINITION_BODY: &str = "row_version=1&unit_path=U1&tag=LINE&name_en=line&description_en=The+zones+of+the+press+line&type=ST_Line%401&value_type=ST_Line%401&s7_address=DB1.DBB0&ordinal=0&value.zones%5B1%5D.speed=0&value.zones%5B1%5D.temp_max=0&value.zones%5B2%5D.speed=0&value.zones%5B2%5D.temp_max=0&value.zones%5B3%5D.speed=0&value.zones%5B3%5D.temp_max=0&value.zones%5B4%5D.speed=0&value.zones%5B4%5D.temp_max=0&limits%5B1%5D.leaf_path=.zones%5B*%5D.speed&limits%5B2%5D.leaf_path=.zones%5B*%5D.temp_max";
/// The path of the page of `LINE`. A save of the edit form without HTMX goes to it.
const LINE_PAGE: &str = "/recipes/R-100/versions/3/parameters/73570007-0000-0000-0000-000000000001";

/// Return the matrix rows of the parameter routes.
///
/// A row holds an endpoint and the answer for each of the six principals: no session, the
/// Administrator, the ProcessEngineer, the Operator, the user who must change the password, and the
/// deactivated user (`api.md` section 3.3).
pub(crate) fn route_rows() -> Vec<(Endpoint, [Want; 6])> {
    let mut rows = new_rows();
    rows.extend(parameter_rows());
    rows.extend(table_and_translation_rows());
    rows
}

/// The answers of a route that every role reads.
const VIEW: [Want; 6] = [
    Want::Login,
    Want::Page,
    Want::Page,
    Want::Page,
    Want::Password,
    Want::Login,
];
/// The answers of a route that needs `ParameterEdit`.
const EDIT: [Want; 6] = [
    Want::Login,
    Want::Page,
    Want::Page,
    Want::Forbidden,
    Want::Password,
    Want::Login,
];

/// Return the rows of the form of a new parameter and of `AddParameter`.
fn new_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Forbidden, Login, Password, Redirect};
    let added = Redirect("/recipes/R-100/versions/3");
    vec![
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/new",
                "",
            ),
            EDIT,
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters",
                ADD_BODY,
            ),
            [Login, added, added, Forbidden, Password, Login],
        ),
    ]
}

/// Return the rows of the page, the edit form, the value, and the limits of `LINE`.
fn parameter_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Forbidden, Login, Password, Redirect};
    let (view, edit) = (VIEW, EDIT);
    let saved = Redirect(LINE_PAGE);
    vec![
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/{key}",
                "",
            ),
            view,
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/definition",
                DEFINITION_BODY,
            ),
            [Login, saved, saved, Forbidden, Password, Login],
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/value",
                VALUE_BODY,
            ),
            edit,
        ),
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
                "",
            ),
            view,
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
                "row_version=1",
            ),
            edit,
        ),
    ]
}

/// Return the rows of the table, the translations, and the removal of `LINE`.
///
/// The translation `pl` of `LINE` exists, so its removal succeeds. A command without HTMX goes to
/// the parameter page or, after the removal, to the version page.
fn table_and_translation_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Forbidden, Login, Password, Redirect};
    let saved = Redirect(LINE_PAGE);
    let removed = Redirect("/recipes/R-100/versions/3");
    let translation = "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}";
    vec![
        (
            endpoint(Method::GET, "/recipes/{ref}/versions/{n}/parameters", ""),
            VIEW,
        ),
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/{key}/translations",
                "",
            ),
            VIEW,
        ),
        (
            endpoint(
                Method::POST,
                translation,
                "row_version=1&name=Linia&description=",
            ),
            [Login, saved, saved, Forbidden, Password, Login],
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}/delete",
                "row_version=1",
            ),
            [Login, saved, saved, Forbidden, Password, Login],
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/delete",
                "row_version=1",
            ),
            [Login, removed, removed, Forbidden, Password, Login],
        ),
    ]
}

/// Return the pattern and the request path of each full `GET` page of the parameter pages.
///
/// The sweep requests each page in both languages. Without `HX-Request`, the table answers the
/// version page, and the limit rows and the translation rows answer the parameter page.
pub(crate) fn sweep_pages() -> Vec<(&'static str, String)> {
    vec![
        (
            "/recipes/{ref}/versions/{n}/parameters",
            "/recipes/R-100/versions/3/parameters".to_owned(),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/{key}/translations",
            format!("{LINE_PAGE}/translations"),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/new",
            "/recipes/R-100/versions/3/parameters/new".to_owned(),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/{key}",
            LINE_PAGE.to_owned(),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
            format!("{LINE_PAGE}/limits"),
        ),
    ]
}

/// Tests of the table of a draft with a parameter whose type does not resolve (ADR-0195 (ad)).
#[cfg(test)]
mod unresolved_type_tests {
    use axum::http::StatusCode;
    use recipe_domain::plant::PlcTypeId;
    use serde_json::json;
    use uuid::Uuid;

    use crate::i18n::Lang;
    use crate::pages::recipes::test_world::recipe_world;
    use crate::test_support::plant::{ENGLISH, HTMX, World, main_of, seed_cycle, struct_id};
    use crate::test_support::recipes::earlier_name_world::{ZONE_KEY, seed_zone_of_type};
    use crate::test_support::text;

    /// The page of the draft 3 of `R-100`. It holds the parameter table.
    const DRAFT_PAGE: &str = "/recipes/R-100/versions/3";

    /// Return the table row that links to the parameter `ZONE`.
    fn zone_row(body: &str) -> String {
        let anchor = format!("/parameters/{ZONE_KEY}");
        let (head, tail) = body.split_once(&anchor).unwrap();
        let start = head.rfind("<tr>").unwrap();
        let end = tail.find("</tr>").unwrap();
        format!(
            "{}{anchor}{}",
            head.get(start..).unwrap(),
            tail.get(..end).unwrap()
        )
    }

    /// The named route of the parameter table. A request with `HX-Request` gets the fragment.
    const TABLE_ROUTE: &str = "/recipes/R-100/versions/3/parameters";

    /// Seed a parameter `ZONE` whose root type is in a struct cycle.
    fn seed_cycle_zone(world: &World) {
        seed_cycle(world);
        seed_zone_of_type(
            &world.fixture,
            world.ids.draft,
            struct_id(1),
            json!({"beta": {}}).into(),
        );
    }

    /// Seed a parameter `ZONE` whose root type has no row.
    fn seed_missing_root_zone(world: &World) {
        let missing = PlcTypeId::from(Uuid::from_u128(0x7357_0e00_0000_0000_0000_0000_0000_0f99));
        seed_zone_of_type(&world.fixture, world.ids.draft, missing, json!(5).into());
    }

    /// Assert the row of a struct cycle: the type name and the composite word.
    fn assert_cycle_row(body: &str) {
        let row = zone_row(body);
        let row = row.as_str();
        let word = text(Lang::En, "parameters-value-composite");
        assert!(row.contains("ST_Alpha@1"), "{row}");
        assert!(
            row.contains(&format!("<td class=\"value-note\">{word}</td>")),
            "{row}"
        );
    }

    /// Assert the row of a missing root row: empty type cell and empty value cell.
    fn assert_missing_root_row(body: &str) {
        let row = zone_row(body);
        let row = row.as_str();
        let word = text(Lang::En, "parameters-value-composite");
        assert!(!row.contains(&word), "{row}");
        assert!(
            row.contains("<td class=\"data\"></td><td class=\"value-note\"></td>"),
            "{row}"
        );
    }

    #[tokio::test]
    async fn the_table_shows_the_composite_word_for_a_parameter_of_a_struct_cycle() {
        let world = recipe_world();
        seed_cycle_zone(&world);

        let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;

        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_cycle_row(main_of(&reply.body));
    }

    #[tokio::test]
    async fn the_table_shows_an_empty_type_and_an_empty_value_for_a_missing_root_row() {
        let world = recipe_world();
        seed_missing_root_zone(&world);

        let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;

        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_missing_root_row(main_of(&reply.body));
    }

    #[tokio::test]
    async fn the_table_fragment_shows_the_composite_word_for_a_parameter_of_a_struct_cycle() {
        let world = recipe_world();
        seed_cycle_zone(&world);

        let reply = world
            .get(&world.engineer, TABLE_ROUTE, &[ENGLISH, HTMX])
            .await;

        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(!reply.body.contains("<main"), "{}", reply.body);
        assert_cycle_row(&reply.body);
    }

    #[tokio::test]
    async fn the_table_fragment_shows_an_empty_type_and_an_empty_value_for_a_missing_root_row() {
        let world = recipe_world();
        seed_missing_root_zone(&world);

        let reply = world
            .get(&world.engineer, TABLE_ROUTE, &[ENGLISH, HTMX])
            .await;

        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(!reply.body.contains("<main"), "{}", reply.body);
        assert_missing_root_row(&reply.body);
    }
}

/// Tests of the largest edit form that the budgets of A26 allow (ADR-0211 (d) to (j)).
#[cfg(test)]
mod largest_form_tests;

/// The helpers of the value leg tests: the form body, its digest, and the field lines.
#[cfg(test)]
mod value_leg_kit;

/// Tests of the two bodies of the value leg of the load job (design `value-body-memory-load` D6).
#[cfg(test)]
mod value_leg_tests;

/// Tests of the memory of the largest edit page after ADR-0223 (change `parameter-page-memory`).
#[cfg(test)]
mod largest_measure_tests;

/// The shared kit of the measurement tests: the pin, the check of an answer, and the loaded request.
#[cfg(test)]
mod measure_kit;

/// Tests of the memory of the refused form of round 1 (task 5.3, ADR-0223).
#[cfg(test)]
mod largest_refused_tests;

/// Tests of the memory of the switch answers of the fixture (task 5.5, ADR-0223).
#[cfg(test)]
mod largest_switch_tests;

/// Tests of the memory of the notice of a new type and of a refused command (task 5.6, ADR-0223).
#[cfg(test)]
mod largest_again_tests;

/// Tests of the memory of a valid save of the edit form (task 5.7, ADR-0223).
#[cfg(test)]
mod largest_save_tests;

/// Tests of the memory of the wide and the deep shape (tasks 5.8 and 5.9, ADR-0223).
#[cfg(test)]
mod shape_tests;

/// Tests of the cap on the bound texts of a limit row at the routes (task 2.2, ADR-0225).
#[cfg(test)]
mod limit_text_tests;

/// Tests of the memory of the largest answer at the cap of a bound text (task 2.4, ADR-0225).
#[cfg(test)]
mod largest_cap_tests;
