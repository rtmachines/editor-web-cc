//! User messages of the database trigger codes (`reliability.md` section 1.2).
//!
//! The table holds the message of each trigger code and of `E-DB-003`. The HTTP layer adds the
//! sentence `Reference: {request_id}.` to each message.

use recipe_domain::error_codes::ErrorCode;

/// Return the user message of a trigger code or of `E-DB-003`.
///
/// Any other code gives `None`. The message ends before the reference sentence.
#[must_use]
pub(crate) fn user_message(code: ErrorCode) -> Option<&'static str> {
    match code {
        ErrorCode::PLT_030 => Some("The PLC platform of a machine with recipes cannot change."),
        ErrorCode::PLT_031 => Some("The type does not exist on the platform of the version."),
        ErrorCode::PLT_032 => Some("The type belongs to another machine."),
        ErrorCode::PLT_033 => Some("A type definition cannot change. Create a new revision."),
        ErrorCode::PLT_034 => {
            Some("The members of a type cannot change after the type is created.")
        }
        ErrorCode::PLT_035 => Some("Only a struct type has members."),
        ErrorCode::PLT_036 => Some("A struct needs at least one member."),
        ErrorCode::PLT_037 => {
            Some("The machine code cannot change while a version that is not a draft uses it.")
        }
        ErrorCode::PLT_038 => Some(
            "The unit path or the machine of a unit cannot change while a version uses the unit.",
        ),
        ErrorCode::RCP_030 => Some("A parameter row cannot move to another version."),
        ErrorCode::RCP_031 => Some("The version is not a live draft."),
        ErrorCode::RCP_032 => Some("The unit does not belong to the machine of the recipe."),
        ErrorCode::RCP_033 => {
            Some("The version status or a frozen column of the version cannot change in this way.")
        }
        ErrorCode::RCP_034 => Some(
            "The reference number or the machine of a recipe cannot change while a version uses it.",
        ),
        ErrorCode::RCP_035 => Some("The version has more than 4 000 parameters."),
        ErrorCode::RCP_036 => Some("The version has more than 32 768 limit rows."),
        ErrorCode::DB_003 => Some("The change violates a database rule."),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
