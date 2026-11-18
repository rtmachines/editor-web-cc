//! Guards for the `Stored` wrap of the platform rule on the rebind path (ADR-0195 (m)).

use std::collections::BTreeMap;

use serde_json::json;

use super::super::test_support::{
    INT, ZONE_2, ZONE_3, draft_on, key, number, param, request_with, trees_on,
};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::id;
use crate::plant::{PlcPlatform, S7Address};
use crate::recipe::draft::support::assert_stored;
use crate::recipe::{Draft, DraftError};

/// Rebind `SPEED` from `ST_Zone@2` to `ST_Zone@3` on `platform`, with or without an S7 address.
fn rebind_zone(platform: PlcPlatform, address: Option<&str>) -> (Draft, Result<(), DraftError>) {
    let mut speed = param(1, "SPEED", ZONE_2, json!({"setpoint": 5}), Vec::new());
    speed.s7_address = address.map(|text| S7Address::parse(text).unwrap());
    let mut draft = draft_on(platform, vec![speed]);
    let zone_trees = trees_on(platform, &[("setpoint", INT), ("temp_max", INT)]);
    let request = request_with(ZONE_2, ZONE_3, &[(1, ".temp_max", number(70))]);
    let plan = draft.plan_rebind(&request, &zone_trees).unwrap();
    let result = draft
        .apply_rebind(plan, &BTreeMap::new(), &zone_trees)
        .map(|_| ());
    (draft, result)
}

#[test]
fn rebind_of_a_stored_parameter_without_an_address_on_s7_300_gives_stored_e_rcp_003() {
    let (draft, result) = rebind_zone(PlcPlatform::S7_300, None);

    assert_stored(&result.unwrap_err(), Some(key(1)), ErrorCode::RCP_003);
    assert_eq!(draft.parameter(key(1)).unwrap().plc_type(), id(ZONE_2));
}

#[test]
fn rebind_of_a_stored_parameter_with_an_address_on_twincat3_gives_stored_e_rcp_053() {
    let (_draft, result) = rebind_zone(PlcPlatform::TwinCat3, Some("DB1.DBB0"));

    assert_stored(&result.unwrap_err(), Some(key(1)), ErrorCode::RCP_053);
}
