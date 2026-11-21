//! `UpdateParameterDefinition` replaces a stored type that does not resolve (ADR-0195 (q)).

use recipe_domain::plant::{PlantError, PlatformTypes, PlcPlatform, StringEncoding, TypeTree};
use recipe_domain::recipe::StoredParameter;
use serde_json::json;

use super::requests::{retyped, run, speed_request, with_type};
use crate::memory::fixtures::{
    STRUCT_CYCLE_ROOT, mid, parameter_on, pkey, seed_struct_cycle, tid, uid, vid,
};
use crate::recipe::test_support::{DRAFT_VERSION, stored_parameters};
use recipe_domain::recipe::ValueInput;

const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;
const CYCLE_PARAMETER: u128 = 5;

#[tokio::test]
async fn a_retype_from_a_struct_cycle_commits_and_audits_the_old_value_under_the_empty_key() {
    let fixture = crate::memory::fixtures::scenarios::zone_draft();
    let rows = seed_struct_cycle(&fixture, (1, PLATFORM));
    let types = PlatformTypes::collect(mid(1), PLATFORM, StringEncoding::Latin1, rows).unwrap();
    assert!(matches!(
        TypeTree::resolve(tid(STRUCT_CYCLE_ROOT), &types).unwrap_err(),
        PlantError::TypeCycle { .. }
    ));
    let cycle = StoredParameter {
        plc_type: tid(STRUCT_CYCLE_ROOT),
        value: json!(0).into(),
        limits: Vec::new(),
        ..parameter_on(PLATFORM, CYCLE_PARAMETER, "CYCLE", uid(101), 1)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &cycle);
    let mut request = with_type(speed_request(), "INT");
    request.key = pkey(CYCLE_PARAMETER);
    request.tag = recipe_domain::recipe::ParameterTag::parse("CYCLE").unwrap();
    request.ordinal = recipe_domain::recipe::Ordinal::parse(1).unwrap();
    let request = retyped(request, ValueInput::Json("0".to_owned()), Vec::new());

    run(&fixture, request).await.unwrap();

    let stored = stored_parameters(&fixture)
        .into_iter()
        .find(|parameter| parameter.key == pkey(CYCLE_PARAMETER))
        .unwrap();
    assert_ne!(stored.plc_type, tid(STRUCT_CYCLE_ROOT));
    let entries = fixture.store.audit_entries();
    let entry = entries.last().unwrap();
    let before = entry.before().unwrap();
    assert_eq!(before.get("value"), Some(&json!({"": 0})));
    let after = entry.after().unwrap();
    assert_eq!(after.get("value"), Some(&json!({"": 0})));
}
