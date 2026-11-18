//! Tests of `UpdateParameterDefinition` for a stored struct above the leaf limit (`E-PLT-018`).

use recipe_domain::plant::{PlantError, PlcPlatform, S7Address, TypeLimit};
use serde_json::json;

use super::requests::{run, speed_request, with_retype, with_type};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, parameter_on, seed_oversized_struct, uid, vid};
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot};

#[tokio::test]
async fn a_changed_type_to_a_stored_struct_above_the_leaf_limit_gives_plt_018_on_every_platform() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
        PlcPlatform::TwinCat3,
    ] {
        let fixture = Fixture::new();
        let machine = fixture.seed_machine_on(platform, 1, "M1");
        fixture.seed_draft(&machine, 1, "R-1");
        let speed = parameter_on(platform, 1, "SPEED", uid(101), 0);
        fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
        seed_oversized_struct(&fixture, platform, "ST_Wide");
        let before = DraftSnapshot::capture(&fixture);
        let request = super::UpdateParameterDefinitionInput {
            s7_address: (platform == PlcPlatform::S7_300)
                .then(|| S7Address::parse("DB1.DBB0").unwrap()),
            ..with_type(speed_request(), "ST_Wide@1")
        };
        let request = with_retype(request, &json!({}).to_string(), Vec::new());
        let error = run(&fixture, request).await.unwrap_err();
        assert!(
            matches!(
                &error,
                AppError::Plant(PlantError::TypeTooLarge {
                    limit: TypeLimit::Leaves,
                    ..
                })
            ),
            "{platform:?}: {error:?}"
        );
        before.assert_unchanged(&fixture);
    }
}
