use recipe_domain::error_codes::ErrorCode;
use recipe_domain::recipe::{ParameterKey, ReferenceNumber};
use uuid::Uuid;

use super::Missing;
use crate::error::AppError;

#[test]
fn missing_malformed_recipe_version_and_parameter_name_the_entity_and_never_the_typed_text() {
    let cases = [
        (Missing::MalformedRecipe, "Recipe"),
        (Missing::MalformedVersion, "Version"),
        (Missing::MalformedParameter, "Parameter"),
    ];
    for (missing, text) in cases {
        assert_eq!(missing.to_string(), text);
        let error = AppError::NotFound { entity: missing };
        assert_eq!(error.code(), ErrorCode::WEB_005);
    }
}

#[test]
fn missing_parameter_names_the_key_the_version_and_the_recipe() {
    let missing = Missing::Parameter {
        reference: ReferenceNumber::parse("R-7").unwrap(),
        number: 3,
        key: ParameterKey::from(Uuid::nil()),
    };

    assert_eq!(
        missing.to_string(),
        "Parameter 00000000-0000-0000-0000-000000000000 of version 3 of recipe R-7"
    );
    let error = AppError::NotFound { entity: missing };
    assert_eq!(error.code(), ErrorCode::WEB_005);
}
