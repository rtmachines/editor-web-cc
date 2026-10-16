//! HTMX tests of each error code of `AddParameter` (task 11.1, design D16).
//!
//! Each answer goes into `#error-panel` and keeps the URL (`api.md` section 5.3). An error of a
//! field also swaps the form out of band with the line under its input (ADR-0041 (e)).

use axum::http::StatusCode;
use recipe_domain::plant::limits::MAX_DEPTH;

use super::test_kit::{ADD, INT_FIELDS, assert_panel_error, changed, post_htmx};
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::plant::World;

/// One error case: the changed fields, the status, the code, and the input of the line.
type Case = (
    Vec<(&'static str, Option<&'static str>)>,
    StatusCode,
    &'static str,
    Option<&'static str>,
);

/// Send the fields of `INT_FIELDS` with `changes` and assert the answer of `want`.
async fn assert_case(world: &World, path: &str, case: Case) {
    let (changes, status, code, input) = case;
    let before = world.fixture.snapshot();
    let pairs = changed(&INT_FIELDS, &changes);
    let reply = post_htmx(world, path, &pairs).await;
    assert_panel_error(&reply, status, code);
    if let Some(input) = input {
        let line = format!("<div class=\"field-error\" id=\"{input}-error\">");
        assert!(reply.body.contains(&line), "{code}: {}", reply.body);
        let link = format!("href=\"#{input}\"");
        assert!(reply.body.contains(&link), "{code}: {}", reply.body);
        assert!(
            reply.body.contains("<form id=\"parameter-new-form\""),
            "{code}"
        );
        assert!(reply.body.contains("hx-swap-oob=\"true\""), "{code}");
    }
    assert_grid_lines_only_under(&reply.body, code, input);
    assert_eq!(world.fixture.snapshot(), before, "{code}");
}

/// Assert that no input of the grid of a primitive except `input` has an error line or a link.
///
/// A fault of the value stands under the value input. A fault of a limit stands under the input of
/// its limit row (ADR-0039 (n), ADR-0041 (e)).
fn assert_grid_lines_only_under(body: &str, code: &str, input: Option<&str>) {
    for other in ["value-0", "limits-0-min", "limits-0-max"] {
        if input == Some(other) {
            continue;
        }
        let line = format!("id=\"{other}-error\"");
        assert!(!body.contains(&line), "{code} {other}: {body}");
        let link = format!("href=\"#{other}\"");
        assert!(!body.contains(&link), "{code} {other}: {body}");
    }
}

/// The cases of the fields of the definition.
fn definition_cases() -> Vec<Case> {
    let unprocessable = StatusCode::UNPROCESSABLE_ENTITY;
    vec![
        (
            vec![("tag", Some("SPEED"))],
            StatusCode::CONFLICT,
            "E-RCP-014",
            Some("tag"),
        ),
        (
            vec![("tag", Some("bad tag"))],
            unprocessable,
            "E-WEB-008",
            Some("tag"),
        ),
        (
            vec![("unit_path", Some("U9"))],
            StatusCode::NOT_FOUND,
            "E-PLT-020",
            Some("unit_path"),
        ),
        (
            vec![("unit_path", Some("x"))],
            unprocessable,
            "E-PLT-017",
            Some("unit_path"),
        ),
        (
            vec![("type", Some("ARRAY[")), ("value_type", None)],
            unprocessable,
            "E-PLT-013",
            Some("type"),
        ),
    ]
}

/// The cases of the S7 address on the S7-300 of `M7`.
fn address_cases() -> Vec<Case> {
    let unprocessable = StatusCode::UNPROCESSABLE_ENTITY;
    [
        ("", "E-RCP-003"),
        ("DB2.DBW0", "E-RCP-022"),
        ("DB9.DBW1", "E-RCP-023"),
        ("DB9.DBB0", "E-RCP-054"),
    ]
    .into_iter()
    .map(|(address, code)| {
        (
            vec![("s7_address", Some(address))],
            unprocessable,
            code,
            Some("s7_address"),
        )
    })
    .collect()
}

/// The cases of the value and of the limits. A fault of the value stands under the value input. A
/// fault of a limit stands under the first input of its limit row.
fn value_cases() -> Vec<Case> {
    let unprocessable = StatusCode::UNPROCESSABLE_ENTITY;
    let limit = |min: &'static str, max: &'static str| {
        vec![
            ("limits[0].leaf_path", Some("")),
            ("limits[0].min", Some(min)),
            ("limits[0].max", Some(max)),
        ]
    };
    let path = vec![
        ("limits[0].leaf_path", Some(".x")),
        ("limits[0].max", Some("5")),
    ];
    vec![
        (
            vec![("value", Some("abc"))],
            unprocessable,
            "E-RCP-005",
            Some("value-0"),
        ),
        (
            limit("0", "10"),
            unprocessable,
            "E-RCP-004",
            Some("value-0"),
        ),
        (
            limit("", "99999"),
            unprocessable,
            "E-RCP-006",
            Some("limits-0-min"),
        ),
        (
            limit("10", "5"),
            unprocessable,
            "E-RCP-018",
            Some("limits-0-min"),
        ),
        (path, unprocessable, "E-RCP-017", None),
    ]
}

#[tokio::test]
async fn parameters_add_errors_of_the_definition_stand_under_their_input() {
    let world = recipe_world();
    for case in definition_cases() {
        assert_case(&world, ADD, case).await;
    }
}

#[tokio::test]
async fn parameters_add_errors_of_the_s7_address_stand_under_its_input() {
    let world = recipe_world();
    for case in address_cases() {
        assert_case(&world, ADD, case).await;
    }
}

#[tokio::test]
async fn parameters_add_errors_of_the_value_and_the_limits_stand_under_the_leaf() {
    let world = recipe_world();
    for case in value_cases() {
        assert_case(&world, ADD, case).await;
    }
}

#[tokio::test]
async fn parameters_add_a_limit_on_a_bool_leaf_gives_e_rcp_019() {
    let world = recipe_world();
    let changes = vec![
        ("type", Some("BOOL")),
        ("value_type", Some("BOOL")),
        ("value", Some("true")),
        ("s7_address", Some("DB9.DBX0.0")),
        ("limits[0].leaf_path", Some("")),
        ("limits[0].min", Some("0")),
    ];
    let case = (changes, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-019", None);
    assert_case(&world, ADD, case).await;
}

#[tokio::test]
async fn parameters_add_a_type_above_the_depth_limit_gives_e_plt_018() {
    let world = recipe_world();
    let depth = usize::try_from(MAX_DEPTH.checked_add(1).unwrap()).unwrap();
    let text: &'static str = format!("{}INT", "ARRAY[1..2] OF ".repeat(depth)).leak();
    let changes = vec![("type", Some(text)), ("value_type", None)];
    let case = (
        changes,
        StatusCode::UNPROCESSABLE_ENTITY,
        "E-PLT-018",
        Some("type"),
    );
    assert_case(&world, ADD, case).await;
}

/// The form of a TwinCAT machine has no S7 address input, so the line stands in the summary only.
#[tokio::test]
async fn parameters_add_an_s7_address_on_twincat_gives_e_rcp_053() {
    let world = recipe_world();
    let changes = vec![("unit_path", Some("U1"))];
    let case = (changes, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-053", None);
    assert_case(&world, "/recipes/R-200/versions/1/parameters", case).await;
}

#[tokio::test]
async fn parameters_add_on_an_archived_unit_gives_e_plt_009() {
    let world = recipe_world();
    let archived = world
        .post(
            &world.engineer,
            "/machines/M7/units/U1.4.1/archive",
            "",
            false,
        )
        .await;
    assert_eq!(archived.status, StatusCode::SEE_OTHER, "{}", archived.body);
    let changes = vec![("unit_path", Some("U1.4.1"))];
    let case = (
        changes,
        StatusCode::CONFLICT,
        "E-PLT-009",
        Some("unit_path"),
    );
    assert_case(&world, ADD, case).await;
}

#[tokio::test]
async fn parameters_add_above_the_parameter_budget_gives_e_rcp_021() {
    let world = recipe_world();
    let large = world.fixture.seed_large_draft();
    let path = format!(
        "/recipes/{}/versions/{}/parameters",
        large.reference, large.number
    );
    let changes = vec![("s7_address", None)];
    let case = (changes, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-021", None);
    assert_case(&world, &path, case).await;
}

/// RR1-2, ADR-0041 (e): an error without an input still swaps the form out of band.
///
/// The form keeps the sent texts and has no error line and no `aria-invalid`, so no line of an
/// earlier answer stays under a valid input. The line stands only in the summary.
#[tokio::test]
async fn parameters_add_above_the_budget_swaps_the_form_without_a_stale_line() {
    let world = recipe_world();
    let large = world.fixture.seed_large_draft();
    let path = format!(
        "/recipes/{}/versions/{}/parameters",
        large.reference, large.number
    );
    let pairs = changed(&INT_FIELDS, &[("s7_address", None)]);
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-021");
    let (_panel, form) = reply
        .body
        .split_once("<form id=\"parameter-new-form\"")
        .unwrap();
    let first_tag = form.split('>').next().unwrap();
    assert!(first_tag.contains("hx-swap-oob=\"true\""), "{first_tag}");
    assert!(form.contains("value=\"NEW_P\""), "{form}");
    assert!(!form.contains("aria-invalid"), "{form}");
    assert!(!form.contains("class=\"field-error\""), "{form}");
    assert!(reply.body.contains("E-RCP-021"), "{}", reply.body);
}

#[tokio::test]
async fn parameters_add_errors_of_the_path_keep_the_url() {
    let world = recipe_world();
    let cases = [
        (
            "/recipes/R-100/versions/2/parameters",
            StatusCode::CONFLICT,
            "E-RCP-002",
        ),
        (
            "/recipes/R-999/versions/3/parameters",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/recipes/R-100/versions/x/parameters",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
    ];
    for (path, status, code) in cases {
        let case = (Vec::new(), status, code, None);
        assert_case(&world, path, case).await;
    }
}

#[tokio::test]
async fn parameters_add_lists_two_bad_fields_at_once() {
    let world = recipe_world();
    let pairs = changed(
        &INT_FIELDS,
        &[("tag", Some("bad tag")), ("ordinal", Some("x"))],
    );
    let reply = post_htmx(&world, ADD, &pairs).await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert!(reply.body.contains("Errors: 2."), "{}", reply.body);
    for input in ["tag", "ordinal"] {
        let line = format!("id=\"{input}-error\"");
        assert!(reply.body.contains(&line), "{input}: {}", reply.body);
    }
}
