//! The refusals of a form: a role without the right and a refused use case (ADR-0013, ADR-0041).
//!
//! The module holds the check that every area uses for a refused role. The form query of the
//! route checks the permission before the field check, so a bad field never shows the form.

use axum::http::StatusCode;
use recipe_app::auth::Operation;
use recipe_app::error::{AppError, Missing};
use recipe_domain::audit::SecurityEventKind;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{MachineCode, PlantError};
use recipe_domain::recipe::{RecipeError, ReferenceNumber};

use super::refusal_lines;
use crate::pages::test_helpers::assert_htmx_error;
use crate::test_support::plant::World;

fn machine() -> MachineCode {
    MachineCode::parse("M7").unwrap()
}

/// A domain error of the Plant Model gives one line with its code and the input of `target`.
#[test]
fn refusal_lines_turn_a_plant_error_into_one_line_under_the_named_input() {
    let error = AppError::Plant(PlantError::DuplicateMachineCode { code: machine() });
    let lines = refusal_lines(error, |seen| {
        assert!(matches!(seen, AppError::Plant(_)), "{seen:?}");
        Some("code".to_owned())
    })
    .unwrap();
    assert_eq!(lines.iter().count(), 1);
    assert_eq!(lines.first().code(), ErrorCode::PLT_001);
    assert_eq!(lines.first().target(), Some("code"));
}

/// A domain error that names no input gives a line that stands only in the summary.
#[test]
fn refusal_lines_turn_a_recipe_error_into_a_line_without_an_input() {
    let reference = ReferenceNumber::parse("R-100").unwrap();
    let error = AppError::Recipe(RecipeError::RecipeDeleted { reference });
    let lines = refusal_lines(error, |_seen| None).unwrap();
    assert_eq!(lines.first().code(), ErrorCode::RCP_012);
    assert_eq!(lines.first().target(), None);
}

/// Access errors, missing rows, and the snapshot check keep their own answer and show no form.
#[test]
fn refusal_lines_keep_every_other_error_as_it_is() {
    let cases = [
        (
            AppError::Forbidden {
                operation: Operation::RenameMachine,
            },
            ErrorCode::WEB_002,
        ),
        (
            AppError::NotFound {
                entity: Missing::Machine(machine()),
            },
            ErrorCode::WEB_005,
        ),
        (
            AppError::Recipe(RecipeError::ValidationFailed { items: Vec::new() }),
            ErrorCode::RCP_020,
        ),
    ];
    for (error, code) in cases {
        let kept = refusal_lines(error, |_seen| Some("code".to_owned())).unwrap_err();
        assert_eq!(kept.code(), code);
    }
}

/// Assert that the Operator gets `403 E-WEB-002` for `body` on `path`, with no form.
///
/// The answer writes one `forbidden` event and nothing else.
pub(crate) async fn assert_operator_refused(world: &World, path: &str, body: &str) {
    let before = world.fixture.snapshot();
    let events = world.fixture.sink.events().len();
    let reply = world.post(&world.operator, path, body, true).await;
    assert_htmx_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002");
    let after = world.fixture.sink.events();
    assert_eq!(after.len(), events.checked_add(1).unwrap(), "{path} {body}");
    assert!(
        after
            .last()
            .is_some_and(|event| event.kind() == SecurityEventKind::Forbidden),
        "{path} {body}"
    );
    assert!(!reply.body.contains("<form"), "{path}: {}", reply.body);
    assert_eq!(world.fixture.snapshot(), before, "{path} {body}");
}
