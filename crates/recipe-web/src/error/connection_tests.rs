//! Tests of the error mapping of the PLC connection codes (task 7.6 of `plc-connection-config`,
//! ADR-0154, `api.md` section 5.1). `E-WEB-013` is a `409` and `E-WEB-014` is a `500`. Both
//! messages name the machine, so the mapping must give the placeable `code` to the `fluent`
//! message. A missing argument makes `fluent` fail and the answer then shows the English text.

use axum::body::to_bytes;
use axum::http::StatusCode;
use fluent::FluentValue;
use recipe_app::error::AppError;
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::MachineCode;
use tracing::Level;
use uuid::Uuid;

use super::WebError;
use super::answer::{ErrorContext, answer, args_of};
use super::log::level_of;
use super::status::status_of;
use crate::i18n::Lang;
use crate::test_support::state;

/// The largest body that a test reads.
const BODY_LIMIT: usize = 1024 * 1024;

fn machine() -> MachineCode {
    MachineCode::parse("M7").unwrap()
}

fn stale() -> WebError {
    WebError::App(AppError::ConnectionConcurrentEdit {
        code: machine(),
        expected: 0,
        actual: 3,
    })
}

fn unreadable() -> WebError {
    WebError::App(AppError::SecretUnreadable { code: machine() })
}

#[test]
fn the_two_connection_codes_have_the_status_of_their_row() {
    assert_eq!(status_of(ErrorCode::WEB_013), StatusCode::CONFLICT);
    assert_eq!(
        status_of(ErrorCode::WEB_014),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[test]
fn the_levels_of_the_two_codes_are_info_for_the_conflict_and_error_for_the_unreadable_secret() {
    assert_eq!(
        level_of(&stale(), ErrorCode::WEB_013, StatusCode::CONFLICT),
        Level::INFO
    );
    assert_eq!(
        level_of(
            &unreadable(),
            ErrorCode::WEB_014,
            StatusCode::INTERNAL_SERVER_ERROR
        ),
        Level::ERROR
    );
}

#[test]
fn each_connection_error_gives_the_code_of_the_machine_to_its_message() {
    let state = state();
    let ctx = ErrorContext {
        request_id: RequestId::from(Uuid::nil()),
        lang: Lang::Pl,
        htmx: true,
        retry: None,
        state: &state,
    };
    for error in [stale(), unreadable()] {
        let args = args_of(&error, &ctx);
        assert!(
            matches!(args.get("code"), Some(FluentValue::String(text)) if text == "M7"),
            "{error:?}: {:?}",
            args.get("code")
        );
    }
}

#[tokio::test]
async fn an_unreadable_secret_answers_500_with_the_code_the_machine_and_no_chain() {
    let state = state();
    let ctx = ErrorContext {
        request_id: RequestId::from(Uuid::nil()),
        lang: Lang::En,
        htmx: true,
        retry: None,
        state: &state,
    };
    let response = answer(&unreadable(), &ctx);
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let bytes = to_bytes(response.into_body(), BODY_LIMIT).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("E-WEB-014"), "{body}");
    assert!(body.contains("machine M7"), "{body}");
    assert!(!body.contains("field-lines"), "{body}");
}
