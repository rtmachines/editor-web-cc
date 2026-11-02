//! Tests of the spec requirement "Transfer routes in read-only mode" (A24, ADR-0109 (ac)).
//!
//! The three machine routes stay available in read-only mode. The Basic check keeps its
//! `login_attempt` row, and an export writes its `VersionExported` entry. A wrong password still
//! gives `401 E-IDN-003` and keeps its failed attempt row.

use axum::http::StatusCode;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use super::tests::{audit_count, client_get, exported_actors, seeded};
use crate::test_support::{Fixture, get_with};

/// The list route of the live machine `M7`.
const M7_LIST: &str = "/export/v1/machines/M7/recipes";
/// The v1 document route of `R-500` on `M7`.
const M7_R500: &str = "/export/v1/machines/M7/recipes/R-500/approved";
/// The v2 unit document route of `R-800` on `M7`.
const M7_R800_UNIT: &str = "/export/v2/machines/M7/recipes/R-800/approved/units/U1.2";
/// A password that the account `user1` does not have.
const WRONG_PASSWORD: &str = "not-the-password";

/// Return the number of `login_attempt` rows and the number of rows with `succeeded = true`.
fn attempts(fixture: &Fixture) -> (usize, usize) {
    let state = fixture.store.state();
    (
        state.login_attempt_count(),
        state.succeeded_login_attempt_count(),
    )
}

/// Build the seeded fixture with `R-800` and switch the read-only mode on.
async fn seeded_read_only() -> Fixture {
    let (fixture, data) = seeded().await;
    fixture.seed_unit_recipe(&data).await;
    fixture.state.read_only().set(true);
    fixture
}

#[tokio::test]
async fn export_list_in_read_only_mode_gives_200_and_confirms_the_attempt_row() {
    let fixture = seeded_read_only().await;
    let entries = audit_count(&fixture);

    let reply = client_get(&fixture, M7_LIST).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(attempts(&fixture), (1, 1));
    assert_eq!(audit_count(&fixture), entries);
}

#[tokio::test]
async fn export_approved_in_read_only_mode_gives_200_writes_the_entry_and_confirms_the_attempt() {
    let fixture = seeded_read_only().await;
    let before = exported_actors(&fixture);

    let reply = client_get(&fixture, M7_R500).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(exported_actors(&fixture).len(), before.len() + 1);
    assert_eq!(attempts(&fixture), (1, 1));
}

#[tokio::test]
async fn export_unit_in_read_only_mode_gives_the_v2_document_and_writes_the_entry() {
    let fixture = seeded_read_only().await;
    let before = exported_actors(&fixture);

    let reply = client_get(&fixture, M7_R800_UNIT).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("\"schema_version\": \"2\""),
        "{}",
        reply.body
    );
    assert_eq!(exported_actors(&fixture).len(), before.len() + 1);
    assert_eq!(attempts(&fixture), (1, 1));
}

#[tokio::test]
async fn export_wrong_password_in_read_only_mode_gives_401_and_keeps_the_failed_attempt() {
    let fixture = seeded_read_only().await;
    let header = format!(
        "Basic {}",
        STANDARD.encode(format!("user1:{WRONG_PASSWORD}"))
    );
    let entries = audit_count(&fixture);
    for path in [M7_LIST, M7_R500, M7_R800_UNIT] {
        let before = attempts(&fixture).0;

        let reply = get_with(&fixture.router(), path, &[("authorization", &header)]).await;

        assert_eq!(
            reply.status,
            StatusCode::UNAUTHORIZED,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-IDN-003"), "{path}: {}", reply.body);
        assert_eq!(attempts(&fixture).0, before + 1, "{path}");
    }
    assert_eq!(attempts(&fixture), (3, 0));
    assert_eq!(audit_count(&fixture), entries);
}
