//! Tests of the one log line of an error.

use axum::http::StatusCode;
use recipe_app::error::{AppError, TaskStage, TimeoutStage};
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use tracing::Level;
use uuid::Uuid;

use super::WebError;
use super::log::{level_of, log_error};
use super::status::status_of;
use crate::test_support::capture_logs;

const TIMEOUT_MESSAGE: &str = "The request took too long and was stopped. Try again.";

/// Write the line of `error` and return the one line that it wrote.
fn logged_line(error: &WebError, code: ErrorCode, level: Level) -> String {
    let (logs, _guard) = capture_logs();
    log_error(error, code, level, RequestId::from(Uuid::nil()));
    let text = logs.text();
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("request failed"))
        .collect();
    assert_eq!(lines.len(), 1, "{text}");
    lines.first().map(|line| (*line).to_owned()).unwrap()
}

fn timeout(stage: TimeoutStage) -> WebError {
    WebError::App(AppError::Timeout { stage })
}

#[test]
fn log_a_timeout_writes_its_stage_as_a_field() {
    let cases = [
        (TimeoutStage::Argon2Permit, "password hash permit"),
        (TimeoutStage::Validation, "validation"),
        (TimeoutStage::Export, "export"),
        (TimeoutStage::Request, "request"),
        (TimeoutStage::Diff, "diff"),
        (TimeoutStage::DetachedSlot, "security step slot"),
    ];
    for (stage, expected) in cases {
        let line = logged_line(&timeout(stage), ErrorCode::WEB_009, Level::ERROR);
        assert!(line.ends_with(&format!(" stage={expected}")), "{line}");
        assert!(
            line.contains(&format!("chain={TIMEOUT_MESSAGE} stage=")),
            "{line}"
        );
        assert!(!line.contains(": stage "), "{line}");
    }
}

#[test]
fn log_a_timeout_keeps_its_user_message_status_and_level() {
    for stage in [
        TimeoutStage::Argon2Permit,
        TimeoutStage::Validation,
        TimeoutStage::Export,
        TimeoutStage::Request,
        TimeoutStage::Diff,
        TimeoutStage::DetachedSlot,
    ] {
        let error = timeout(stage);
        let code = error.code();
        let status = status_of(code);
        assert_eq!(code, ErrorCode::WEB_009);
        assert_eq!(error.to_string(), TIMEOUT_MESSAGE);
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(level_of(&error, code, status), Level::ERROR);
    }
}

#[test]
fn log_an_error_that_is_not_a_timeout_has_no_stage_field() {
    let error = WebError::App(AppError::TaskFailed {
        stage: TaskStage::PasswordHash,
    });
    let line = logged_line(&error, ErrorCode::WEB_011, Level::ERROR);
    assert!(!line.contains("stage="), "{line}");
    assert!(!line.contains(": stage "), "{line}");
    assert!(line.ends_with(&error.to_string()), "{line}");
}

#[test]
fn log_a_timeout_writes_its_stage_at_the_warn_and_info_levels() {
    for level in [Level::WARN, Level::INFO] {
        let line = logged_line(
            &timeout(TimeoutStage::Argon2Permit),
            ErrorCode::WEB_009,
            level,
        );
        assert!(
            line.contains("stage=password hash permit"),
            "{level}: {line}"
        );
    }
}

#[test]
fn log_an_error_that_is_not_a_timeout_has_no_stage_field_at_the_warn_and_info_levels() {
    let error = WebError::App(AppError::TaskFailed {
        stage: TaskStage::PasswordHash,
    });
    for level in [Level::WARN, Level::INFO] {
        let line = logged_line(&error, ErrorCode::WEB_011, level);
        assert!(!line.contains("stage="), "{level}: {line}");
    }
}
