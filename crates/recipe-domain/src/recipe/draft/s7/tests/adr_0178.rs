//! Tests of the array bound rule of ADR-0178.

use super::super::super::support::{address, tag};
use super::support::{S7_PLATFORMS, approval, assert_not_addressable, definition};
use super::{
    ARRAY_OF_ST_WIDE, ARRAY_OF_WIDE, BOOL_3D, LOWER_EDGE, LOWER_WIDE, ST_EDGE, ST_WIDE, UPPER_EDGE,
    UPPER_WIDE, check, recipe_error,
};
use crate::error_codes::{ErrorCode, REGISTRY};
use crate::plant::PlcPlatform;
use crate::recipe::{RecipeError, S7AccessReason};
/// ADR-0178 (b), ADR-0187 (b): the variant `S7TypeNotAddressable` returns `E-RCP-058` for each
/// reason, and the code has a registry row.
#[test]
fn adr_0178_error_code() {
    assert!(REGISTRY.contains(&ErrorCode::RCP_058));
    assert_eq!(ErrorCode::RCP_058.as_str(), "E-RCP-058");
    for reason in [
        S7AccessReason::ArrayBoundOutsideInt,
        S7AccessReason::BoolArrayWithThreeDimensions,
    ] {
        let error = RecipeError::S7TypeNotAddressable {
            tag: tag("SPEED"),
            address: address("DB1.DBW0"),
            reason,
        };
        assert_eq!(error.code(), ErrorCode::RCP_058, "{reason:?}");
    }
}

/// ADR-0178 (a): the bounds `32767` and `-32768` pass.
#[test]
fn adr_0178_the_bounds_32767_and_minus_32768_pass() {
    for platform in S7_PLATFORMS {
        for type_n in [UPPER_EDGE, LOWER_EDGE, ST_EDGE] {
            let text = Some("DB1.DBB0");
            assert!(definition(platform, type_n, text).is_ok(), "{type_n}");
            assert!(approval(platform, type_n, text).is_ok(), "{type_n}");
        }
    }
}

/// ADR-0178 (a), (b): the bounds `32768` and `-32769` fail with `E-RCP-058`.
#[test]
fn adr_0178_the_bounds_32768_and_minus_32769_give_e_rcp_058() {
    for platform in S7_PLATFORMS {
        for type_n in [UPPER_WIDE, LOWER_WIDE] {
            assert_not_addressable(
                platform,
                type_n,
                "DB1.DBB0",
                S7AccessReason::ArrayBoundOutsideInt,
            );
        }
    }
    let error = recipe_error(definition(
        PlcPlatform::S7_1500,
        UPPER_WIDE,
        Some("DB1.DBB0"),
    ));
    assert_eq!(error.code(), ErrorCode::RCP_058);
    assert_eq!(
        error.to_string(),
        "Parameter SPEED at DB1.DBB0 has a type that a data block with standard access cannot hold: an array bound is outside -32768 to 32767. Change the type."
    );
}

/// ADR-0178 (a): the rule reads struct members and array elements.
#[test]
fn adr_0178_a_bound_in_a_struct_member_and_in_an_array_element_fails() {
    for platform in S7_PLATFORMS {
        assert_not_addressable(
            platform,
            ST_WIDE,
            "DB1.DBB0",
            S7AccessReason::ArrayBoundOutsideInt,
        );
    }
    for type_n in [ARRAY_OF_WIDE, ARRAY_OF_ST_WIDE] {
        assert_not_addressable(
            PlcPlatform::S7_1500,
            type_n,
            "DB1.DBB0",
            S7AccessReason::ArrayBoundOutsideInt,
        );
    }
}

/// ADR-0178 (a), ADR-0187 (f): a parameter without an address passes.
#[test]
fn adr_0178_a_parameter_without_an_address_passes() {
    let types = [UPPER_WIDE, LOWER_WIDE, ST_WIDE, ARRAY_OF_ST_WIDE, BOOL_3D];
    for platform in [PlcPlatform::S7_1200, PlcPlatform::S7_1500] {
        for type_n in types {
            assert!(definition(platform, type_n, None).is_ok(), "{type_n}");
            assert!(approval(platform, type_n, None).is_ok(), "{type_n}");
        }
    }
}

/// ADR-0178 (d): the standard-access rule runs after the end rule.
#[test]
fn adr_0178_the_rule_runs_after_the_end_rule() {
    let result = definition(PlcPlatform::S7_1500, ARRAY_OF_WIDE, Some("DB1.DBB65534"));
    assert_eq!(recipe_error(result).code(), ErrorCode::RCP_056);
    let result = approval(PlcPlatform::S7_1500, ARRAY_OF_WIDE, Some("DB1.DBB65534"));
    assert_eq!(recipe_error(result).code(), ErrorCode::RCP_056);
}

/// ADR-0178 (c): the layout check after a command skips the address rules, so a stored fault of
/// another parameter does not refuse the command (ADR-0116 (e1)).
#[test]
fn adr_0178_the_layout_check_alone_does_not_apply_the_rule() {
    for type_n in [UPPER_WIDE, BOOL_3D] {
        let result = check(PlcPlatform::S7_1500, &[("SPEED", type_n, Some("DB1.DBB0"))]);
        assert!(result.is_ok(), "{type_n}");
    }
}
