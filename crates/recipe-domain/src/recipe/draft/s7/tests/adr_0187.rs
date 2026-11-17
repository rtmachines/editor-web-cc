//! Tests of the `BOOL` array dimension rule of ADR-0187.

use super::support::{S7_PLATFORMS, approval, assert_not_addressable, definition};
use super::{ARRAY_OF_ST_BOOL_3D, BOOL_2D, BOOL_3D, ST_BOOL_2D, ST_BOOL_3D, recipe_error};
use crate::error_codes::ErrorCode;
use crate::plant::PlcPlatform;
use crate::recipe::S7AccessReason;
/// ADR-0187 (a), (b): a `BOOL` array with three dimensions fails, also as a struct member and as
/// an array element.
#[test]
fn adr_0187_a_bool_array_with_three_dimensions_gives_e_rcp_058() {
    for platform in S7_PLATFORMS {
        for type_n in [BOOL_3D, ST_BOOL_3D] {
            assert_not_addressable(
                platform,
                type_n,
                "DB1.DBB0",
                S7AccessReason::BoolArrayWithThreeDimensions,
            );
        }
    }
    assert_not_addressable(
        PlcPlatform::S7_1500,
        ARRAY_OF_ST_BOOL_3D,
        "DB1.DBB0",
        S7AccessReason::BoolArrayWithThreeDimensions,
    );
    let error = recipe_error(definition(PlcPlatform::S7_300, BOOL_3D, Some("DB1.DBB0")));
    assert_eq!(error.code(), ErrorCode::RCP_058);
    assert_eq!(
        error.to_string(),
        "Parameter SPEED at DB1.DBB0 has a type that a data block with standard access cannot hold: a BOOL array with three dimensions has no verified S7 layout. Change the type."
    );
}

/// ADR-0187 (a): a `BOOL` array with two dimensions passes.
#[test]
fn adr_0187_a_bool_array_with_two_dimensions_passes() {
    for platform in S7_PLATFORMS {
        for type_n in [BOOL_2D, ST_BOOL_2D] {
            assert!(definition(platform, type_n, Some("DB1.DBB0")).is_ok());
            assert!(approval(platform, type_n, Some("DB1.DBB0")).is_ok());
        }
    }
}
