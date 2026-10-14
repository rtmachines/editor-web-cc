//! The HTTP status of each error code (`reliability.md` section 1.2, column HTTP).

use axum::http::StatusCode;
use recipe_domain::error_codes::ErrorCode;

/// The first number of the trigger range in `reliability.md` section 1.2.
const TRIGGER_FIRST: u32 = 30;
/// The last number of the trigger range in `reliability.md` section 1.2.
const TRIGGER_LAST: u32 = 49;

/// Return the HTTP status of a code.
///
/// The match follows the column HTTP of `reliability.md` section 1.2, and a test compares both
/// for every row. `E-IDN-004` has the status of `E-IDN-003`. `E-IDN-007` is the status `303` of
/// the redirect to `/me/password`. A code that the table does not list gives `500`.
pub(crate) fn status_of(code: ErrorCode) -> StatusCode {
    match code.as_str() {
        "E-IDN-007" => StatusCode::SEE_OTHER,
        "E-IDN-003" | "E-IDN-004" | "E-WEB-004" => StatusCode::UNAUTHORIZED,
        "E-WEB-002" | "E-WEB-007" => StatusCode::FORBIDDEN,
        "E-PLT-019" | "E-PLT-020" | "E-RCP-050" | "E-RCP-051" | "E-EXP-001" | "E-WEB-005" => {
            StatusCode::NOT_FOUND
        }
        "E-WEB-003" => StatusCode::PAYLOAD_TOO_LARGE,
        "E-IDN-005" => StatusCode::TOO_MANY_REQUESTS,
        "E-DB-001" | "E-DB-002" | "E-DB-004" | "E-DB-005" | "E-WEB-006" | "E-WEB-009" => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        "E-PLT-001" | "E-PLT-004" | "E-PLT-008" | "E-PLT-009" | "E-PLT-010" | "E-PLT-011"
        | "E-PLT-012" | "E-PLT-014" | "E-PLT-015" | "E-PLT-030" | "E-PLT-033" | "E-PLT-034"
        | "E-PLT-037" | "E-PLT-038" | "E-RCP-001" | "E-RCP-002" | "E-RCP-007" | "E-RCP-009"
        | "E-RCP-010" | "E-RCP-011" | "E-RCP-012" | "E-RCP-014" | "E-RCP-015" | "E-RCP-025"
        | "E-RCP-026" | "E-RCP-030" | "E-RCP-031" | "E-RCP-033" | "E-RCP-034" | "E-IDN-001"
        | "E-IDN-002" | "E-IDN-009" | "E-IDN-010" | "E-EXP-003" | "E-DB-003" | "E-DB-006"
        | "E-DB-007" | "E-WEB-001" => StatusCode::CONFLICT,
        "E-PLT-002" | "E-PLT-003" | "E-PLT-005" | "E-PLT-006" | "E-PLT-013" | "E-PLT-016"
        | "E-PLT-017" | "E-PLT-018" | "E-PLT-021" | "E-PLT-031" | "E-PLT-032" | "E-PLT-035"
        | "E-PLT-036" | "E-RCP-003" | "E-RCP-004" | "E-RCP-005" | "E-RCP-006" | "E-RCP-008"
        | "E-RCP-013" | "E-RCP-016" | "E-RCP-017" | "E-RCP-018" | "E-RCP-019" | "E-RCP-020"
        | "E-RCP-021" | "E-RCP-022" | "E-RCP-023" | "E-RCP-024" | "E-RCP-027" | "E-RCP-028"
        | "E-RCP-029" | "E-RCP-032" | "E-RCP-035" | "E-RCP-036" | "E-RCP-052" | "E-RCP-053"
        | "E-RCP-054" | "E-IDN-006" | "E-IDN-008" | "E-WEB-008" => StatusCode::UNPROCESSABLE_ENTITY,
        _unlisted => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Return true when the code is a rule of the database that the domain check should have caught.
///
/// The codes are `E-DB-003`, `E-DB-006`, `E-DB-007`, `E-AUD-003`, and the trigger codes
/// `E-PLT-030..049` and `E-RCP-030..049`. The event log gets an `ERROR` line for each of them
/// (`api.md` section 5.1).
pub(crate) fn is_database_rule(code: ErrorCode) -> bool {
    let mut parts = code.as_str().split('-');
    let (Some(_prefix), Some(context), Some(number)) = (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    match context {
        "DB" => matches!(number, "003" | "006" | "007"),
        "AUD" => number == "003",
        "PLT" | "RCP" => number
            .parse::<u32>()
            .is_ok_and(|n| (TRIGGER_FIRST..=TRIGGER_LAST).contains(&n)),
        _other => false,
    }
}

#[cfg(test)]
mod tests;
