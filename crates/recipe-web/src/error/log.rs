//! The one log line of an error (`reliability.md` sections 1.1 and 3.3, design D13 item n).

use std::error::Error;

use axum::http::StatusCode;
use recipe_app::auth::{Operation, Requirement};
use recipe_app::error::AppError;
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Permission;
use tracing::Level;

use super::WebError;
use super::lists::errors_text;
use super::status::{is_database_rule, status_of};

/// Return the `Display` text of an error and of each of its sources, joined by `: `.
///
/// The text never comes from `Debug`. The `Debug` of a database error holds the row values of
/// the detail, for example a password hash (ADR-0035).
fn display_chain(error: &dyn Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(inner) = source {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        source = inner.source();
    }
    text
}

/// Return true when `operation` needs the permission `UserManage`.
fn is_user_management(operation: Operation) -> bool {
    operation.requirement() == Requirement::Permission(Permission::UserManage)
}

/// Return true when a code gives a 5xx status or is a rule of the database.
fn is_severe(code: ErrorCode) -> bool {
    status_of(code).is_server_error() || is_database_rule(code)
}

/// Return the name of a failed read that the answer could not use, and its failure.
///
/// A conflict keeps the failed read of the version. A refused form keeps the failed read of its
/// form. Every other error has no such read.
fn read_failure(error: &WebError) -> Option<(&'static str, &AppError)> {
    match error {
        WebError::Conflict { view, .. } => view.failure().map(|failure| ("version", failure)),
        WebError::Fields {
            failure: Some(failure),
            ..
        } => Some(("form", failure.as_ref())),
        _other => None,
    }
}

/// Return the level of the line for an error.
///
/// A 5xx status and a rule of the database give `ERROR`. A throttled login, a rejected CSRF
/// token, and a denied user-management operation give `WARN`. Every other 4xx gives `INFO`.
///
/// An error with a failed read takes the level of that failure, when it is higher.
pub(crate) fn level_of(error: &WebError, code: ErrorCode, status: StatusCode) -> Level {
    if status.is_server_error() || is_database_rule(code) {
        return Level::ERROR;
    }
    if read_failure(error).is_some_and(|(_read, failure)| is_severe(failure.code())) {
        return Level::ERROR;
    }
    let denied_user_management = matches!(
        error,
        WebError::App(AppError::Forbidden { operation }) if is_user_management(*operation)
    );
    if code == ErrorCode::IDN_005 || code == ErrorCode::WEB_007 || denied_user_management {
        Level::WARN
    } else {
        Level::INFO
    }
}

/// Write the one line of an error: the code, the request id, and the `Display` chain.
///
/// This is the only place that logs an error of a request. Lower layers propagate it.
///
/// An error with a failed read, for example of the version of a conflict, adds the chain of that
/// failure to the same line. A list of recipe errors writes its count and its first 20 entries.
pub(crate) fn log_error(error: &WebError, code: ErrorCode, level: Level, request_id: RequestId) {
    let mut chain = match error {
        WebError::App(AppError::Recipes(errors)) => errors_text(errors),
        _other => display_chain(error),
    };
    if let Some((read, failure)) = read_failure(error) {
        chain.push_str(": the read of the ");
        chain.push_str(read);
        chain.push_str(" failed: ");
        chain.push_str(&display_chain(failure));
    }
    if level == Level::ERROR {
        tracing::error!(error_code = %code, request_id = %request_id, chain = %chain, "request failed");
    } else if level == Level::WARN {
        tracing::warn!(error_code = %code, request_id = %request_id, chain = %chain, "request failed");
    } else {
        tracing::info!(error_code = %code, request_id = %request_id, chain = %chain, "request failed");
    }
}
