//! Tests of the field lines of a failed command (ADR-0041 (e), design D5 of `plc-type-ranges`).
//!
//! The codes `E-RCP-056` and `E-RCP-057` stand under the input `s7_address`. The code `E-RCP-055`
//! stands under the input that the grid names for its leaf.

use recipe_app::error::AppError;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{LeafPath, S7Address};
use recipe_domain::recipe::{ParameterTag, PrecisionField, RecipeError, RecipeErrors};

use super::command_lines;
use crate::error::{FieldError, leaf_of};

fn tag() -> ParameterTag {
    ParameterTag::parse("SPEED").unwrap()
}

fn address(text: &str) -> S7Address {
    S7Address::parse(text).unwrap()
}

fn precision(leaf: &str) -> RecipeError {
    RecipeError::PrecisionExceeded {
        tag: tag(),
        leaf: LeafPath::parse(leaf).unwrap(),
        field: PrecisionField::Value,
        type_name: "REAL".to_owned(),
        max_digits: 7,
    }
}

fn past_block_end() -> RecipeError {
    RecipeError::S7AddressPastBlockEnd {
        tag: tag(),
        address: address("DB1.DBD65534"),
        size: 4,
    }
}

fn block_out_of_range() -> RecipeError {
    RecipeError::S7DataBlockOutOfRange {
        tag: tag(),
        address: address("DB16001.DBW0"),
        max: 16_000,
    }
}

/// Return the codes and the targets of the lines of `error`.
fn lines_of(
    error: RecipeError,
    input_id: &dyn Fn(&RecipeError) -> Option<String>,
) -> Vec<(ErrorCode, Option<String>)> {
    let lines = command_lines(&AppError::from(error), input_id).unwrap();
    lines
        .iter()
        .map(|line: &FieldError| (line.code(), line.target().map(str::to_owned)))
        .collect()
}

#[test]
fn reply_e_rcp_056_stands_under_the_input_s7_address() {
    let found = lines_of(past_block_end(), &|_fault| None);
    assert_eq!(found, [(ErrorCode::RCP_056, Some("s7_address".to_owned()))]);
}

#[test]
fn reply_e_rcp_057_stands_under_the_input_s7_address() {
    let found = lines_of(block_out_of_range(), &|_fault| None);
    assert_eq!(found, [(ErrorCode::RCP_057, Some("s7_address".to_owned()))]);
}

#[test]
fn reply_e_rcp_055_stands_under_the_input_that_the_grid_names_for_its_leaf() {
    let found = lines_of(precision(""), &|_fault| Some("value-0".to_owned()));
    assert_eq!(found, [(ErrorCode::RCP_055, Some("value-0".to_owned()))]);
    let found = lines_of(precision("[*]"), &|_fault| Some("limits-0-min".to_owned()));
    assert_eq!(
        found,
        [(ErrorCode::RCP_055, Some("limits-0-min".to_owned()))]
    );
}

#[test]
fn reply_e_rcp_055_in_a_list_of_faults_stands_under_the_input_of_its_leaf() {
    let list = RecipeErrors::from_vec(vec![precision("[1]"), precision("[2]")]).unwrap();
    let lines = command_lines(&AppError::Recipes(list), &leaf_input).unwrap();
    let targets: Vec<Option<&str>> = lines.iter().map(FieldError::target).collect();
    assert_eq!(targets, [Some("value-1"), Some("value-2")]);
}

/// Return `value-<n>` for the leaf of a fault, as a grid of a list does.
fn leaf_input(fault: &RecipeError) -> Option<String> {
    let leaf = leaf_of(fault)?;
    Some(format!(
        "value-{}",
        leaf.to_string().trim_matches(['[', ']'])
    ))
}
