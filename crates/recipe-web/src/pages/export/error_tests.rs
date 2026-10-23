//! Tests of the error answers of the two export routes (design D11 T11, ADR-0053).
//!
//! Each error is the JSON body of the error layer, and no error writes a `VersionExported` entry.
//! `seed_approval_recipes` gives `R-500` (approved version 1, draft version 2), `R-600` (only a
//! retired version 1), and the deleted `R-700`. `recipe_world` gives `R-100`, whose approved
//! version 2 has the zero hash of the memory seed.

use axum::http::StatusCode;

use super::tests::{export, exports};
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::{Fixture, Reply};

/// The cases of `seed_approval_recipes`: the path, the status, and the code.
const CASES: [(&str, StatusCode, &str); 9] = [
    (
        "/export/v1/recipes/R-600/approved",
        StatusCode::NOT_FOUND,
        "E-EXP-001",
    ),
    (
        "/export/v1/recipes/R-700/approved",
        StatusCode::NOT_FOUND,
        "E-EXP-001",
    ),
    (
        "/export/v1/recipes/R-500/versions/2",
        StatusCode::CONFLICT,
        "E-EXP-003",
    ),
    (
        "/export/v1/recipes/R-700/versions/1",
        StatusCode::CONFLICT,
        "E-RCP-012",
    ),
    (
        "/export/v1/recipes/R-999/approved",
        StatusCode::NOT_FOUND,
        "E-WEB-005",
    ),
    (
        "/export/v1/recipes/R-500/versions/9",
        StatusCode::NOT_FOUND,
        "E-WEB-005",
    ),
    (
        "/export/v1/recipes/%FF/approved",
        StatusCode::NOT_FOUND,
        "E-WEB-005",
    ),
    (
        "/export/v1/recipes/%FF/versions/1",
        StatusCode::NOT_FOUND,
        "E-WEB-005",
    ),
    (
        "/export/v1/recipes/R-500/versions/%FF",
        StatusCode::NOT_FOUND,
        "E-WEB-005",
    ),
];

/// Assert the JSON error body of ADR-0053 (a) with `code`, and no `HX-*` header.
fn assert_json_error(reply: &Reply, status: StatusCode, code: &str, what: &str) {
    assert_eq!(reply.status, status, "{what}: {}", reply.body);
    assert_eq!(reply.header("content-type"), "application/json", "{what}");
    assert_eq!(reply.header("content-disposition"), "", "{what}");
    let start = format!("{{\"code\":\"{code}\",\"message\":\"");
    assert!(reply.body.starts_with(&start), "{what}: {}", reply.body);
    let request_id = format!(",\"request_id\":\"{}\"}}", reply.header("x-request-id"));
    assert!(reply.body.ends_with(&request_id), "{what}: {}", reply.body);
    let names: Vec<&str> = reply.headers.keys().map(|name| name.as_str()).collect();
    assert!(names.iter().all(|name| !name.starts_with("hx-")), "{what}");
}

#[tokio::test]
async fn export_errors_of_both_routes_are_json_bodies_and_write_no_entry() {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    for (path, status, code) in CASES {
        let before = fixture.snapshot();
        let reply = export(&fixture, &data.admin, path).await;
        assert_json_error(&reply, status, code, path);
        assert_eq!(fixture.snapshot(), before, "{path}");
    }
    assert!(exports(&fixture).is_empty());
}

#[tokio::test]
async fn export_a_stored_hash_that_the_body_does_not_match_gives_exp_002_as_json() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let path = "/export/v1/recipes/R-100/approved";
    let reply = export(&world.fixture, &world.admin, path).await;
    assert_json_error(&reply, StatusCode::INTERNAL_SERVER_ERROR, "E-EXP-002", path);
    assert_eq!(world.fixture.snapshot(), before);
    assert!(exports(&world.fixture).is_empty());
}
