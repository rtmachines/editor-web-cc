//! Tests of the S7 layout rules inside the overlap check of a draft (ADR-0147, design D1).
//!
//! The struct `ST_Bytes` holds `BYTE`, `CHAR`, `SINT`, and `USINT`. A `SINT` and a `USINT`
//! member start at an even byte, so the struct takes 6 bytes and not 4.
//! `SINT` and `USINT` do not exist on `S7_300`, so these cases run on `S7_1500`.
//! Each line of the last index of a `BOOL` array starts at a byte (ADR-0147 (d), design D2).
//! The `BOOL` array cases run on `S7_300`.

use super::super::add::support::{draft_with, new_parameter_at, try_add};
use super::super::support::{address, check_every, stored_draft_with, stored_parameter};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{
    array_row, id, machine, on_platform, primitive_row, struct_row, types_for,
};
use crate::plant::{PlcPlatform, PlcType, Primitive, TypeTrees};
use crate::recipe::{Draft, DraftError, RecipeError, StoredParameter, VersionState};

const INT: u128 = 1;
const BYTE: u128 = 2;
const CHAR: u128 = 3;
const SINT: u128 = 4;
const USINT: u128 = 5;
const ST_BYTES: u128 = 6;
const BOOL: u128 = 7;
const GRID: u128 = 8;

/// The rows of the struct cases. `SINT` and `USINT` exist on `S7_1500` and not on `S7_300`.
fn bytes_rows() -> Vec<PlcType> {
    vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(BYTE, Primitive::Byte),
        primitive_row(CHAR, Primitive::Char),
        primitive_row(SINT, Primitive::Sint),
        primitive_row(USINT, Primitive::Usint),
        struct_row(
            ST_BYTES,
            "ST_Bytes",
            &[("a", BYTE), ("b", CHAR), ("c", SINT), ("d", USINT)],
        ),
    ]
}

/// The rows of the `BOOL` array cases.
fn grid_rows() -> Vec<PlcType> {
    vec![
        primitive_row(BOOL, Primitive::Bool),
        array_row(GRID, BOOL, &[(1, 3), (1, 3)]),
        primitive_row(BYTE, Primitive::Byte),
    ]
}

/// The trees of the struct cases on `S7_1500`.
fn trees() -> TypeTrees {
    let platform = PlcPlatform::S7_1500;
    let ids = [INT, BYTE, CHAR, SINT, USINT, ST_BYTES].map(id);
    TypeTrees::resolve(
        ids,
        &types_for(machine(), platform, on_platform(platform, bytes_rows())),
    )
}

/// The trees of the `BOOL` array cases on `S7_300`.
fn grid_trees() -> TypeTrees {
    let platform = PlcPlatform::S7_300;
    let ids = [BYTE, BOOL, GRID].map(id);
    TypeTrees::resolve(
        ids,
        &types_for(machine(), platform, on_platform(platform, grid_rows())),
    )
}

/// A live draft on `S7_1500` with the parameter `BYTES` of type `ST_Bytes` at `DB1.DBB0`.
fn draft_with_bytes() -> Draft {
    draft_with(
        PlcPlatform::S7_1500,
        vec![stored_parameter(
            1,
            "BYTES",
            id(ST_BYTES),
            Some(address("DB1.DBB0")),
        )],
    )
}

fn add_error(
    draft: &mut Draft,
    trees: &TypeTrees,
    type_n: u128,
    address_text: &str,
    tag: &str,
) -> Option<ErrorCode> {
    let new = new_parameter_at(2, tag, type_n, address_text);
    match try_add(draft, trees, new) {
        Ok(()) => None,
        Err(DraftError::Recipe(error)) => Some(error.code()),
        Err(DraftError::Plant(_) | DraftError::List(_)) => None,
    }
}

/// Scenario "SINT and USINT members start on an even byte".
#[test]
fn a_byte_at_byte_4_overlaps_a_struct_with_sint_and_usint_members() {
    let mut draft = draft_with_bytes();
    let error = add_error(&mut draft, &trees(), BYTE, "DB1.DBB4", "MODE");
    assert_eq!(error, Some(ErrorCode::RCP_022));
}

/// Scenario "Parameter after a struct with SINT and USINT members".
#[test]
fn an_int_at_byte_6_fits_after_a_struct_with_sint_and_usint_members() {
    let mut draft = draft_with_bytes();
    let new = new_parameter_at(2, "TEMP", INT, "DB1.DBW6");
    assert!(try_add(&mut draft, &trees(), new).is_ok());
}

/// ADR-0147 (c): the rule applies to a struct member only.
#[test]
fn a_sint_parameter_at_an_odd_byte_keeps_its_place() {
    let draft = draft_of(&[("LEVEL", SINT, "DB1.DBB1")]);
    assert!(check_every(&draft, &trees()).is_ok());
}

/// ADR-0147 (c): a `USINT` parameter keeps its place too. A `SINT` next to it does not move.
#[test]
fn a_sint_and_a_usint_parameter_on_neighbor_bytes_do_not_overlap() {
    let draft = draft_of(&[("LEVEL", SINT, "DB1.DBB1"), ("COUNT", USINT, "DB1.DBB2")]);
    assert!(check_every(&draft, &trees()).is_ok());
}

/// A live draft on `S7_300` with the parameter `GRID` of type `ARRAY[1..3,1..3] OF BOOL` at
/// `DB1.DBB0`.
fn draft_with_grid() -> Draft {
    draft_with(
        PlcPlatform::S7_300,
        vec![stored_parameter(
            1,
            "GRID",
            id(GRID),
            Some(address("DB1.DBB0")),
        )],
    )
}

/// Scenario "Lines of a BOOL array with two dimensions".
#[test]
fn a_byte_at_byte_2_overlaps_a_bool_array_with_two_dimensions() {
    let mut draft = draft_with_grid();
    let error = add_error(&mut draft, &grid_trees(), BYTE, "DB1.DBB2", "MODE");
    assert_eq!(error, Some(ErrorCode::RCP_022));
}

/// Scenario "Parameter after a BOOL array with two dimensions".
#[test]
fn a_byte_at_byte_4_fits_after_a_bool_array_with_two_dimensions() {
    let mut draft = draft_with_grid();
    let new = new_parameter_at(2, "MODE", BYTE, "DB1.DBB4");
    assert!(try_add(&mut draft, &grid_trees(), new).is_ok());
}

fn draft_of(specs: &[(&str, u128, &str)]) -> Draft {
    let parameters: Vec<StoredParameter> = specs
        .iter()
        .enumerate()
        .map(|(index, (tag_text, type_n, text))| {
            let n = u128::try_from(index).unwrap().checked_add(1).unwrap();
            stored_parameter(n, tag_text, id(*type_n), Some(address(text)))
        })
        .collect();
    Draft::load(stored_draft_with(
        VersionState::LiveDraft,
        PlcPlatform::S7_1500,
        parameters,
    ))
    .unwrap()
}

/// The struct ends at byte 6, so a `BYTE` at byte 5 overlaps the padding at the end of the struct.
#[test]
fn the_check_names_the_struct_when_a_byte_lies_in_its_end_padding() {
    let draft = draft_of(&[("BYTES", ST_BYTES, "DB1.DBB0"), ("MODE", BYTE, "DB1.DBB5")]);
    let result = check_every(&draft, &trees());
    let tags = match result {
        Err(DraftError::Recipe(RecipeError::S7AddressOverlap { tag, other_tag, .. })) => {
            Some((tag.to_string(), other_tag.to_string()))
        }
        _ => None,
    };
    assert_eq!(tags, Some(("MODE".to_owned(), "BYTES".to_owned())));
}
