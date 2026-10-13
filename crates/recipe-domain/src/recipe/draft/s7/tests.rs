//! Tests of the S7 rules of a draft (design D10, invariants I5 and I20, ADR-0024).

use super::super::support::{address, stored_draft_with, stored_parameter, tag};
use crate::error_codes::{ErrorCode, REGISTRY};
use crate::plant::fixtures::{
    id, machine, on_platform, primitive_row, string_row, struct_row, types, types_for,
};
use crate::plant::{PlcPlatform, PlcType, Primitive, S7Width, TypeTree, TypeTrees};
use crate::recipe::{Draft, DraftError, RecipeError, StoredParameter, VersionState};

const INT: u128 = 1;
const REAL: u128 = 2;
const BOOL: u128 = 3;
const BYTE: u128 = 4;
const STRING_3: u128 = 5;
const ST_FLAGS: u128 = 6;
const ST_MIX: u128 = 7;
const UNKNOWN: u128 = 99;

/// The type rows of the tests. The trees come from the shared fixtures.
fn rows() -> Vec<PlcType> {
    vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(REAL, Primitive::Real),
        primitive_row(BOOL, Primitive::Bool),
        primitive_row(BYTE, Primitive::Byte),
        string_row(STRING_3, 3),
        struct_row(
            ST_FLAGS,
            "ST_Flags",
            &[("a", BOOL), ("b", BOOL), ("c", BOOL)],
        ),
        struct_row(ST_MIX, "ST_Mix", &[("a", BOOL), ("b", INT), ("c", BYTE)]),
    ]
}

fn trees() -> TypeTrees {
    trees_on(PlcPlatform::TwinCat3)
}

fn trees_on(platform: PlcPlatform) -> TypeTrees {
    let ids = [INT, REAL, BOOL, BYTE, STRING_3, ST_FLAGS, ST_MIX, UNKNOWN].map(id);
    TypeTrees::resolve(
        ids,
        &types_for(machine(), platform, on_platform(platform, rows())),
    )
}

fn tree_of(type_n: u128) -> TypeTree {
    TypeTree::resolve(id(type_n), &types(rows())).unwrap()
}

/// One parameter of a test: the tag, the type, and the address text.
type Spec = (&'static str, u128, Option<&'static str>);

fn draft_of(platform: PlcPlatform, specs: &[Spec]) -> Draft {
    let parameters: Vec<StoredParameter> = specs
        .iter()
        .enumerate()
        .map(|(index, (tag_text, type_n, text))| {
            let n = u128::try_from(index).unwrap().checked_add(1).unwrap();
            stored_parameter(n, tag_text, id(*type_n), text.map(address))
        })
        .collect();
    Draft::load(stored_draft_with(
        VersionState::LiveDraft,
        platform,
        parameters,
    ))
    .unwrap()
}

fn check(platform: PlcPlatform, specs: &[Spec]) -> Result<(), DraftError> {
    draft_of(platform, specs).check_s7_layout(&trees_on(platform))
}

fn recipe_error(result: Result<(), DraftError>) -> RecipeError {
    let error = result.err().and_then(|error| match error {
        DraftError::Recipe(inner) => Some(inner),
        DraftError::Plant(_) | DraftError::List(_) => None,
    });
    error.expect("the check returns a RecipeError")
}

#[test]
fn s7_300_without_an_address_gives_e_rcp_003() {
    let error = recipe_error(check(PlcPlatform::S7_300, &[("SPEED", INT, None)]));
    assert_eq!(error.code(), ErrorCode::RCP_003);
    assert_eq!(
        error.to_string(),
        "Parameter SPEED needs a PLC address. The machine uses an S7-300 controller."
    );
}

#[test]
fn twincat3_with_an_address_gives_e_rcp_053() {
    let error = recipe_error(check(
        PlcPlatform::TwinCat3,
        &[("SPEED", INT, Some("DB1.DBW4"))],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_053);
    assert_eq!(
        error.to_string(),
        "Parameter SPEED cannot have a PLC address on platform TwinCat3. Delete the address."
    );
}

#[test]
fn codesys_with_an_address_gives_e_rcp_053() {
    let error = recipe_error(check(
        PlcPlatform::CodesysIec,
        &[("SPEED", INT, Some("DB1.DBW4"))],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_053);
}

#[test]
fn s7_1200_and_s7_1500_accept_a_missing_and_a_present_address() {
    for platform in [PlcPlatform::S7_1200, PlcPlatform::S7_1500] {
        assert!(check(platform, &[("SPEED", INT, None)]).is_ok());
        assert!(check(platform, &[("SPEED", INT, Some("DB1.DBW4"))]).is_ok());
    }
}

#[test]
fn twincat3_without_addresses_passes() {
    assert!(check(PlcPlatform::TwinCat3, &[("SPEED", INT, None)]).is_ok());
}

#[test]
fn int_at_a_double_word_address_gives_e_rcp_054_with_width_w() {
    let error = recipe_error(check(
        PlcPlatform::S7_300,
        &[("SPEED", INT, Some("DB1.DBD0"))],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_054);
    assert_eq!(
        error.to_string(),
        "Address DB1.DBD0 of SPEED does not match the type of SPEED. Use the width W."
    );
    assert!(matches!(
        error,
        RecipeError::S7AddressWidthMismatch {
            width: S7Width::W,
            ..
        }
    ));
}

#[test]
fn bool_at_a_byte_address_gives_e_rcp_054_with_width_x() {
    let error = recipe_error(check(
        PlcPlatform::S7_300,
        &[("RUN", BOOL, Some("DB1.DBB0"))],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_054);
    assert!(matches!(
        error,
        RecipeError::S7AddressWidthMismatch {
            width: S7Width::X,
            ..
        }
    ));
}

#[test]
fn real_at_a_double_word_address_passes() {
    assert!(check(PlcPlatform::S7_300, &[("TEMP", REAL, Some("DB1.DBD4"))]).is_ok());
}

#[test]
fn int_at_an_odd_offset_gives_e_rcp_023() {
    let error = recipe_error(check(
        PlcPlatform::S7_300,
        &[("SPEED", INT, Some("DB1.DBW3"))],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_023);
    assert_eq!(
        error.to_string(),
        "Address DB1.DBW3 of SPEED must start at an even byte."
    );
}

#[test]
fn a_struct_at_an_odd_offset_gives_e_rcp_023() {
    let error = recipe_error(check(
        PlcPlatform::S7_300,
        &[("FLAGS", ST_FLAGS, Some("DB1.DBB1"))],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_023);
}

#[test]
fn a_byte_and_a_bool_may_start_at_an_odd_offset() {
    assert!(
        check(
            PlcPlatform::S7_300,
            &[
                ("B", BYTE, Some("DB1.DBB1")),
                ("X", BOOL, Some("DB1.DBX3.0"))
            ]
        )
        .is_ok()
    );
}

#[test]
fn two_parameters_at_one_address_give_e_rcp_022_for_the_later_tag() {
    let error = recipe_error(check(
        PlcPlatform::S7_300,
        &[
            ("SPEED", INT, Some("DB1.DBW4")),
            ("TEMP", INT, Some("DB1.DBW4")),
        ],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_022);
    assert_eq!(
        error.to_string(),
        "Address DB1.DBW4 of TEMP overlaps with SPEED."
    );
}

#[test]
fn a_struct_of_three_bools_covers_two_bytes() {
    let error = recipe_error(check(
        PlcPlatform::S7_300,
        &[
            ("FLAGS", ST_FLAGS, Some("DB1.DBB0")),
            ("OTHER", BYTE, Some("DB1.DBB1")),
        ],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_022);
}

#[test]
fn an_int_after_a_struct_of_three_bools_passes() {
    assert!(
        check(
            PlcPlatform::S7_300,
            &[
                ("FLAGS", ST_FLAGS, Some("DB1.DBB0")),
                ("SPEED", INT, Some("DB1.DBW2")),
            ]
        )
        .is_ok()
    );
}

#[test]
fn a_mixed_struct_has_six_bytes() {
    let error = recipe_error(check(
        PlcPlatform::S7_300,
        &[
            ("MIX", ST_MIX, Some("DB1.DBB0")),
            ("OTHER", BYTE, Some("DB1.DBB4")),
        ],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_022);
    assert!(
        check(
            PlcPlatform::S7_300,
            &[
                ("MIX", ST_MIX, Some("DB1.DBB0")),
                ("OTHER", BYTE, Some("DB1.DBB6")),
            ]
        )
        .is_ok()
    );
}

#[test]
fn two_bools_at_different_bits_of_one_byte_pass() {
    assert!(
        check(
            PlcPlatform::S7_300,
            &[
                ("A", BOOL, Some("DB1.DBX0.0")),
                ("B", BOOL, Some("DB1.DBX0.1")),
            ]
        )
        .is_ok()
    );
}

#[test]
fn two_bools_at_one_bit_give_e_rcp_022() {
    let error = recipe_error(check(
        PlcPlatform::S7_300,
        &[
            ("A", BOOL, Some("DB1.DBX0.0")),
            ("B", BOOL, Some("DB1.DBX0.0")),
        ],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_022);
}

#[test]
fn a_bool_inside_the_bytes_of_a_word_gives_e_rcp_022() {
    let error = recipe_error(check(
        PlcPlatform::S7_300,
        &[
            ("SPEED", INT, Some("DB1.DBW0")),
            ("RUN", BOOL, Some("DB1.DBX1.7")),
        ],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_022);
}

#[test]
fn the_same_offsets_in_two_data_blocks_pass() {
    assert!(
        check(
            PlcPlatform::S7_300,
            &[
                ("SPEED", INT, Some("DB1.DBW4")),
                ("TEMP", INT, Some("DB2.DBW4")),
            ]
        )
        .is_ok()
    );
}

#[test]
fn an_unresolved_parameter_with_an_address_skips_width_alignment_and_overlap() {
    assert!(
        check(
            PlcPlatform::S7_300,
            &[
                ("LOST", UNKNOWN, Some("DB1.DBW1")),
                ("SPEED", INT, Some("DB1.DBW0"))
            ]
        )
        .is_ok()
    );
}

#[test]
fn an_unresolved_parameter_without_an_address_on_s7_300_gives_e_rcp_003() {
    let error = recipe_error(check(
        PlcPlatform::S7_300,
        &[("LOST", UNKNOWN, None), ("SPEED", INT, Some("DB1.DBW0"))],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_003);
}

#[test]
fn an_unresolved_parameter_with_an_address_on_twincat3_gives_e_rcp_053() {
    let error = recipe_error(check(
        PlcPlatform::TwinCat3,
        &[("LOST", UNKNOWN, Some("DB1.DBW0"))],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_053);
}

#[test]
fn the_first_error_is_the_one_of_the_first_tag() {
    let error = recipe_error(check(
        PlcPlatform::S7_300,
        &[
            ("ZETA", INT, Some("DB1.DBW3")),
            ("ALPHA", INT, Some("DB1.DBD0")),
        ],
    ));
    assert_eq!(error.code(), ErrorCode::RCP_054);
}

#[test]
fn the_single_parameter_check_reports_the_platform_width_and_alignment_rules() {
    let int = tree_of(INT);
    let check_one = |platform, text: Option<&str>| {
        super::check_definition(platform, &tag("SPEED"), text.map(address), &int)
    };
    assert!(check_one(PlcPlatform::S7_300, Some("DB1.DBW2")).is_ok());
    assert_eq!(
        check_one(PlcPlatform::S7_300, None).unwrap_err().code(),
        ErrorCode::RCP_003
    );
    assert_eq!(
        check_one(PlcPlatform::TwinCat3, Some("DB1.DBW2"))
            .unwrap_err()
            .code(),
        ErrorCode::RCP_053
    );
    assert_eq!(
        check_one(PlcPlatform::S7_300, Some("DB1.DBD0"))
            .unwrap_err()
            .code(),
        ErrorCode::RCP_054
    );
    assert_eq!(
        check_one(PlcPlatform::S7_300, Some("DB1.DBW3"))
            .unwrap_err()
            .code(),
        ErrorCode::RCP_023
    );
}

#[test]
fn the_five_new_codes_have_registry_rows_and_texts() {
    for (code, text) in [
        (ErrorCode::RCP_003, "E-RCP-003"),
        (ErrorCode::RCP_022, "E-RCP-022"),
        (ErrorCode::RCP_023, "E-RCP-023"),
        (ErrorCode::RCP_053, "E-RCP-053"),
        (ErrorCode::RCP_054, "E-RCP-054"),
    ] {
        assert!(REGISTRY.contains(&code), "{text}");
        assert_eq!(code.as_str(), text);
    }
}

#[test]
fn trees_that_do_not_hold_a_type_of_the_draft_give_e_plt_013() {
    let draft = draft_of(PlcPlatform::TwinCat3, &[("SPEED", INT, None)]);
    let partial = TypeTrees::resolve([id(REAL)], &types(rows()));
    let error = draft.check_s7_layout(&partial).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013, "{error}");
}

#[test]
fn a_type_that_failed_to_resolve_stays_out_of_the_layout_check() {
    let draft = draft_of(PlcPlatform::TwinCat3, &[("LOST", UNKNOWN, None)]);
    assert!(draft.check_s7_layout(&trees()).is_ok());
}
