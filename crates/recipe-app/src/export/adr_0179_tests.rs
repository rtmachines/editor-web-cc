//! Tests of the export of a version with a stored type over the S7 nesting limit (ADR-0179 (i)).

use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::ApprovalOutcome;

use super::tests::export_approved;
use crate::memory::fixtures::{
    Fixture, meta, over_limit_top_parameter, parameter_on, reference, second_engineer,
    seed_struct_chain, uid, vid,
};
use crate::recipe::{ApproveVersion, ApproveVersionInput};

#[tokio::test]
async fn adr_0179_a_version_with_a_stored_type_of_nine_levels_on_s7_is_approved_and_loadable() {
    let fixture = Fixture::new();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let speed = parameter_on(PlcPlatform::S7_1500, 1, "SPEED", uid(102), 0);
    fixture.store.seed_parameter(vid(101), &speed);
    let top = seed_struct_chain(&fixture, PlcPlatform::S7_1500, ("ST_Top", 9));
    fixture
        .store
        .seed_parameter(vid(101), &over_limit_top_parameter(top));
    let approved = ApproveVersion::new(fixture.context())
        .execute(
            &second_engineer(),
            &meta(),
            ApproveVersionInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: 1,
                allow_outdated: false,
            },
        )
        .await
        .unwrap();
    assert!(
        matches!(approved.outcome, ApprovalOutcome::Approved { .. }),
        "{:?}",
        approved.outcome
    );
    let exported = export_approved(&fixture, Role::Operator).await.unwrap();
    assert!(exported.loadable());
}
