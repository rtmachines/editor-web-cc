//! Tests of `ApproveVersion` for a stored type over the S7 nesting limit (ADR-0179 (g), (i)).

use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::ApprovalOutcome;

use super::{run, seed_with};
use crate::memory::fixtures::{
    Fixture, over_limit_top_parameter, parameter_on, seed_struct_chain, uid,
};

#[tokio::test]
async fn adr_0179_a_stored_parameter_with_nine_levels_on_s7_stays_approvable() {
    let fixture = Fixture::new();
    let speed = parameter_on(PlcPlatform::S7_1500, 1, "SPEED", uid(101), 0);
    seed_with(&fixture, PlcPlatform::S7_1500, &[speed]);
    let top = seed_struct_chain(&fixture, PlcPlatform::S7_1500, ("ST_Top", 9));
    fixture.store.seed_parameter(
        crate::memory::fixtures::vid(101),
        &over_limit_top_parameter(top),
    );
    let outcome = run(&fixture, 1, 1, false).await.unwrap();
    assert!(
        matches!(outcome, ApprovalOutcome::Approved { .. }),
        "{outcome:?}"
    );
}
