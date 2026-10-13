//! Requests and the runner that the tests of `UpdateParameterDefinition` share.

use recipe_domain::identity::Role;
use recipe_domain::plant::{TypeExpr, UnitPath};
use recipe_domain::recipe::{LimitInput, LimitList, Ordinal, ParameterTag};
use recipe_domain::text::LongText;

use super::super::DraftOutput;
use super::super::test_support::speed_key;
use super::{RetypeInput, UpdateParameterDefinition, UpdateParameterDefinitionInput};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, actor, meta, reference, text};

/// A request that keeps the definition of `SPEED` as the seed stores it.
pub(super) fn speed_request() -> UpdateParameterDefinitionInput {
    UpdateParameterDefinitionInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: speed_key(),
        unit_path: UnitPath::parse("U1").unwrap(),
        tag: ParameterTag::parse("SPEED").unwrap(),
        name_en: text("Name"),
        description_en: LongText::parse("Description").unwrap(),
        type_expr: TypeExpr::parse("INT").unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(0).unwrap(),
        retype: None,
    }
}

/// The same request with the type `type_text`.
pub(super) fn with_type(
    request: UpdateParameterDefinitionInput,
    type_text: &str,
) -> UpdateParameterDefinitionInput {
    UpdateParameterDefinitionInput {
        type_expr: TypeExpr::parse(type_text).unwrap(),
        ..request
    }
}

/// The same request with a new value and new limits.
pub(super) fn with_retype(
    request: UpdateParameterDefinitionInput,
    value: &str,
    limits: Vec<LimitInput>,
) -> UpdateParameterDefinitionInput {
    UpdateParameterDefinitionInput {
        retype: Some(RetypeInput {
            value_json: value.to_owned(),
            limits: LimitList::parse(limits).unwrap(),
        }),
        ..request
    }
}

/// Run the use case as a process engineer.
pub(super) async fn run(
    fixture: &Fixture,
    request: UpdateParameterDefinitionInput,
) -> Result<DraftOutput, AppError> {
    UpdateParameterDefinition::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
}
