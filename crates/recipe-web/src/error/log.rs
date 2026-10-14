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
use super::status::is_database_rule;

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

/// Return the level of the line for an error.
///
/// A 5xx status and a rule of the database give `ERROR`. A throttled login, a rejected CSRF
/// token, and a denied user-management operation give `WARN`. Every other 4xx gives `INFO`.
pub(crate) fn level_of(error: &WebError, code: ErrorCode, status: StatusCode) -> Level {
    if status.is_server_error() || is_database_rule(code) {
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
pub(crate) fn log_error(error: &WebError, code: ErrorCode, level: Level, request_id: RequestId) {
    let chain = display_chain(error);
    if level == Level::ERROR {
        tracing::error!(error_code = %code, request_id = %request_id, chain = %chain, "request failed");
    } else if level == Level::WARN {
        tracing::warn!(error_code = %code, request_id = %request_id, chain = %chain, "request failed");
    } else {
        tracing::info!(error_code = %code, request_id = %request_id, chain = %chain, "request failed");
    }
}
