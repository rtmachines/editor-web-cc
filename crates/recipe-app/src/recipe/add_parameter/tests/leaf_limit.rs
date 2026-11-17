//! Tests of `AddParameter` for a stored struct above the leaf limit (`E-PLT-018`, not `E-DB-008`).

use recipe_domain::plant::{PlantError, PlcPlatform, S7Address, TypeLimit};
use serde_json::json;

use super::{assert_nothing_written, empty_draft, input, run};
use crate::error::AppError;
use crate::memory::fixtures::seed_oversized_struct;
use crate::recipe::test_support::stored_parameters;

#[tokio::test]
async fn a_stored_struct_above_the_leaf_limit_gives_plt_018_on_every_platform() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
        PlcPlatform::TwinCat3,
    ] {
        let fixture = empty_draft(platform);
        seed_oversized_struct(&fixture, platform, "ST_Wide");
        let request = super::AddParameterInput {
            s7_address: (platform == PlcPlatform::S7_300)
                .then(|| S7Address::parse("DB1.DBB0").unwrap()),
            ..input("ST_Wide@1", &json!({}).to_string(), Vec::new())
        };
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
        assert!(stored_parameters(&fixture).is_empty(), "{platform:?}");
        assert_nothing_written(&fixture);
    }
}
