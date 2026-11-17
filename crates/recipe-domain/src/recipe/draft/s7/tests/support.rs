//! Shared helpers of the tests of ADR-0178 and ADR-0187.

use super::super::super::support::{address, tag};
use super::super::check_definition;
use super::{check_for_approval_rules, recipe_error, tree_of};
use crate::plant::PlcPlatform;
use crate::recipe::{DraftError, RecipeError, S7AccessReason};
pub(super) const S7_PLATFORMS: [PlcPlatform; 3] = [
    PlcPlatform::S7_300,
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
];

/// Run `check_definition` for a type at an address.
pub(super) fn definition(
    platform: PlcPlatform,
    type_n: u128,
    text: Option<&str>,
) -> Result<(), DraftError> {
    check_definition(platform, &tag("SPEED"), text.map(address), &tree_of(type_n))
}

/// Run the approval check for a type at an address.
pub(super) fn approval(
    platform: PlcPlatform,
    type_n: u128,
    text: Option<&'static str>,
) -> Result<(), DraftError> {
    check_for_approval_rules(platform, &[("SPEED", type_n, text)])
}

/// The reason of an `E-RCP-058` error. The function fails when the result is another outcome.
pub(super) fn access_reason(result: Result<(), DraftError>) -> S7AccessReason {
    let reason = match recipe_error(result) {
        RecipeError::S7TypeNotAddressable { reason, .. } => Some(reason),
        _ => None,
    };
    reason.expect("the error is E-RCP-058")
}

/// Assert that both checks give `E-RCP-058` with `reason` for a type at an address.
pub(super) fn assert_not_addressable(
    platform: PlcPlatform,
    type_n: u128,
    text: &'static str,
    reason: S7AccessReason,
) {
    assert_eq!(
        access_reason(definition(platform, type_n, Some(text))),
        reason,
        "check_definition, type {type_n}, {platform:?}"
    );
    assert_eq!(
        access_reason(approval(platform, type_n, Some(text))),
        reason,
        "approval check, type {type_n}, {platform:?}"
    );
}
