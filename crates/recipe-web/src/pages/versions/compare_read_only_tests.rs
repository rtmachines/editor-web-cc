//! Tests of the compare route in read-only mode (task 4.3, ADR-0108 (e), Assumption A24).
//!
//! The compare route is a `GET` route that writes nothing. It stays open in read-only mode. The
//! route has no operation, so the gate reads `Access::Authenticated`.

use axum::http::StatusCode;

use crate::pages::audit::test_world::audit_world;
use crate::test_support::get_with;

/// The address of the compare route of `R-100` for the versions 3 and 1.
const COMPARE: &str = "/recipes/R-100/compare?n=3&m=1";

#[tokio::test]
async fn compare_route_in_read_only_mode_still_redirects_to_the_diff() {
    let world = audit_world().await;
    world.fixture.state.read_only().set(true);

    let reply = world.get_en(&world.operator, COMPARE).await;

    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes/R-100/versions/3/diff/1");
    assert!(!reply.body.contains("E-WEB-006"), "{}", reply.body);
}

#[tokio::test]
async fn compare_route_in_read_only_mode_still_needs_a_session() {
    let world = audit_world().await;
    world.fixture.state.read_only().set(true);

    let reply = get_with(&world.fixture.router(), COMPARE, &[]).await;

    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert!(
        reply.header("location").starts_with("/login"),
        "{}",
        reply.header("location")
    );
}
