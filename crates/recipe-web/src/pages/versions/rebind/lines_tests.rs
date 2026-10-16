//! Tests of the lines of a refused rebind (RR1-2, ADR-0041 (e)).
//!
//! A refusal of the command that names no input still gets a line, so the answer swaps the form
//! with the sent texts, and no line of an earlier answer stays under a valid input.

use recipe_app::auth::Operation;
use recipe_app::error::AppError;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::PlcTypeId;
use recipe_domain::recipe::{RebindTargetReason, RecipeError};

use super::list_lines;
use crate::error::FormErrors;
use crate::pages::versions::rebind_form::RebindForm;

/// Return a form of `ST_Zone@2` with no parameter.
fn form() -> RebindForm {
    let zone = PlcTypeId::from(uuid::Uuid::from_u128(2));
    RebindForm {
        id: "rebind-zone".to_owned(),
        action: "/recipes/R-100/versions/3/rebind".to_owned(),
        title: "Rebind".to_owned(),
        csrf_token: String::new(),
        row_version: "1".to_owned(),
        from: zone,
        to: zone,
        to_text: "ST_Zone@3".to_owned(),
        parameters: Vec::new(),
        bool_options: Vec::new(),
        oob: false,
        errors: FormErrors::default(),
        notice: None,
    }
}

#[test]
fn rebind_lines_give_a_single_e_rcp_052_a_line_without_an_input() {
    let error = AppError::Recipe(RecipeError::RebindTargetInvalid {
        type_name: "ST_Zone@2".to_owned(),
        reason: RebindTargetReason::PlanMismatch,
    });
    let lines = list_lines(&error, &form()).unwrap();
    assert_eq!(lines.iter().count(), 1);
    assert_eq!(lines.first().code(), ErrorCode::RCP_052);
    assert_eq!(lines.first().target(), None);
}

#[test]
fn rebind_lines_keep_an_access_error_without_a_line() {
    let error = AppError::Forbidden {
        operation: Operation::RebindTypeRevision,
    };
    assert!(list_lines(&error, &form()).is_none());
}
