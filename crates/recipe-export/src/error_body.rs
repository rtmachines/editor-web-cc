//! JSON body of an error answer on an export route (`api.md` section 5.3, ADR-0053).

use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use serde::Serialize;

/// The fields of the error body. The field order is part of the contract (ADR-0053 (a)).
///
/// The type solves one problem: a typed struct fixes the order of the three fields, and a
/// `serde_json::Value` would sort them.
#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
    request_id: String,
}

/// Write the compact JSON object `{"code":…,"message":…,"request_id":…}` of an error answer.
///
/// The three fields come in this order. The function escapes quotes, backslashes, and control
/// characters in `message`. Other characters stay as UTF-8. The `request_id` is the text of a UUID
/// and needs no escape. `RequestId` is `Copy`, so the function takes it by value. When
/// `serde_json` fails, the function writes a fixed body with the same code, an empty `message`,
/// and the same `request_id`. The function never panics.
#[must_use]
pub fn error_body(code: ErrorCode, message: &str, request_id: RequestId) -> Vec<u8> {
    let body = ErrorBody {
        code: code.as_str(),
        message,
        request_id: request_id.to_string(),
    };
    serde_json::to_vec(&body).unwrap_or_else(|_| fallback_body(code, request_id))
}

/// Write the body for the case that `serde_json` fails.
///
/// The code comes from the registry and the request id is a UUID text. Neither needs an escape.
fn fallback_body(code: ErrorCode, request_id: RequestId) -> Vec<u8> {
    format!(
        r#"{{"code":"{}","message":"","request_id":"{request_id}"}}"#,
        code.as_str()
    )
    .into_bytes()
}

#[cfg(test)]
mod tests;
