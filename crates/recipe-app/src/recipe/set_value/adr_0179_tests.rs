//! Tests of `SetParameterValue` for a stored type over the S7 nesting limit (ADR-0179 (g)).

use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::ValueInput;

use super::{SetParameterValue, SetParameterValueInput};
use crate::memory::fixtures::{
    Fixture, actor, meta, over_limit_top_parameter, parameter_on, reference, seed_struct_chain,
    uid, vid,
};
use crate::recipe::test_support::{DRAFT_VERSION, speed_key};

#[tokio::test]
async fn adr_0179_a_stored_parameter_over_the_limit_does_not_stop_a_value_of_another_parameter() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let speed = parameter_on(PlcPlatform::S7_1500, 1, "SPEED", uid(101), 0);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    let top = seed_struct_chain(&fixture, PlcPlatform::S7_1500, ("ST_Top", 9));
    fixture
        .store
        .seed_parameter(vid(DRAFT_VERSION), &over_limit_top_parameter(top));
    let request = SetParameterValueInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: speed_key(),
        value: ValueInput::Json("130".to_owned()),
    };
    let output = SetParameterValue::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
        .unwrap();
    assert_eq!(output.row_version, 2);
}
