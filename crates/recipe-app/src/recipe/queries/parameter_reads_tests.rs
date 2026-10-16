//! Tests of `GetParameter`, `GetLimits`, and `GetTranslations` (design D4, ADR-0042, ADR-0043 (e)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{PlcPlatform, UnitPath};
use recipe_domain::recipe::ParameterKey;

use super::parameters::{
    GetLimits, GetParameter, GetTranslations, LimitsView, ParameterRef, ParameterView,
    TranslationsView,
};
use super::parameters_tests::{TYPE_TEXTS, typed_world};
use super::test_world::seed_users;
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::JournalEntry;
use crate::memory::fixtures::{
    Fixture, actor, meta, parameter_on, pkey, reference, scenarios, uid, vid,
};
use crate::ports::Isolation;

/// The draft `R-1` with the parameter `SPEED` (key 1) and the four users.
fn world() -> Fixture {
    let fixture = scenarios::draft();
    seed_users(&fixture);
    fixture
}

/// The deleted recipe `R-1` whose discarded draft holds the parameter `SPEED` (key 1).
fn deleted_world() -> Fixture {
    let fixture = scenarios::deleted();
    seed_users(&fixture);
    fixture.store.seed_parameter(
        vid(101),
        &parameter_on(PlcPlatform::TwinCat3, 1, "SPEED", uid(101), 0),
    );
    fixture
}

fn target(key: ParameterKey) -> ParameterRef {
    ParameterRef {
        reference: reference("R-1"),
        number: 1,
        key,
    }
}

async fn parameter(fixture: &Fixture, role: Role, key: u128) -> Result<ParameterView, AppError> {
    GetParameter::new(fixture.context())
        .execute(&actor(role), &meta(), target(pkey(key)))
        .await
}

async fn limits(fixture: &Fixture, role: Role, key: u128) -> Result<LimitsView, AppError> {
    GetLimits::new(fixture.context())
        .execute(&actor(role), &meta(), target(pkey(key)))
        .await
}

async fn translations(
    fixture: &Fixture,
    role: Role,
    key: u128,
) -> Result<TranslationsView, AppError> {
    GetTranslations::new(fixture.context())
        .execute(&actor(role), &meta(), target(pkey(key)))
        .await
}

/// Tell whether `error` is `E-WEB-005` for the parameter `key` of version 1 of `R-1`.
fn is_missing_parameter(error: &AppError, key: u128) -> bool {
    let expected = Missing::Parameter {
        reference: reference("R-1"),
        number: 1,
        key: pkey(key),
    };
    matches!(error, AppError::NotFound { entity } if entity == &expected)
}

#[test]
fn the_queries_declare_their_operations() {
    assert_eq!(GetParameter::OPERATION, Operation::GetParameter);
    assert_eq!(GetLimits::OPERATION, Operation::GetLimits);
    assert_eq!(GetTranslations::OPERATION, Operation::GetTranslations);
}

#[tokio::test]
async fn get_parameter_returns_the_parameter_its_tree_the_unit_and_the_platform() {
    let fixture = world();

    let view = parameter(&fixture, Role::Operator, 1).await.unwrap();

    assert_eq!(view.parameter.key, pkey(1));
    assert_eq!(view.parameter.tag.to_string(), "SPEED");
    assert_eq!(view.tree.leaf_count(), 1);
    assert_eq!(view.tree.root_type(), view.parameter.plc_type);
    assert_eq!(view.unit_path, UnitPath::parse("U1").unwrap());
    assert_eq!(view.machine.platform(), PlcPlatform::TwinCat3);
    assert_eq!(view.detail.row_version, 1);
    assert_eq!(view.recipe.reference().to_string(), "R-1");
}

#[tokio::test]
async fn get_parameter_gives_the_type_text_of_each_kind_of_type() {
    let fixture = typed_world();

    for (key, text) in TYPE_TEXTS {
        let view = parameter(&fixture, Role::Operator, key).await.unwrap();

        assert_eq!(view.type_text, text, "key {key}");
    }
}

#[tokio::test]
async fn get_limits_returns_the_limit_rows_with_the_row_version() {
    let fixture = world();

    let view = limits(&fixture, Role::Operator, 1).await.unwrap();

    assert_eq!(view.key, pkey(1));
    assert_eq!(view.tag.to_string(), "SPEED");
    assert_eq!(view.limits.len(), 1);
    let row = view.limits.first().unwrap();
    assert_eq!(
        row.min.map(|bound| bound.to_string()),
        Some("10".to_owned())
    );
    assert_eq!(
        row.max.map(|bound| bound.to_string()),
        Some("250".to_owned())
    );
    assert_eq!(view.detail.row_version, 1);
}

#[tokio::test]
async fn get_limits_returns_the_type_tree_that_the_limits_grid_needs() {
    // ADR-0046 (d): the limits route runs `GetLimits`, so the view carries the tree.
    let fixture = typed_world();

    for (key, _text) in TYPE_TEXTS {
        let view = limits(&fixture, Role::Operator, key).await.unwrap();
        let shown = parameter(&fixture, Role::Operator, key).await.unwrap();

        assert_eq!(view.tree, shown.tree, "key {key}");
        assert_eq!(view.tree.root_type(), shown.parameter.plc_type, "key {key}");
    }
}

#[tokio::test]
async fn get_translations_returns_the_rows_with_the_english_base_text() {
    let fixture = world();

    let view = translations(&fixture, Role::Operator, 1).await.unwrap();

    assert_eq!(view.key, pkey(1));
    assert_eq!(view.tag.to_string(), "SPEED");
    assert_eq!(view.name_en.as_str(), "Name");
    assert_eq!(view.description_en.as_str(), "Description");
    let langs: Vec<String> = view
        .translations
        .iter()
        .map(|row| row.lang.to_string())
        .collect();
    assert_eq!(langs, ["pl"]);
    assert_eq!(view.detail.row_version, 1);
}

#[tokio::test]
async fn the_three_queries_read_in_one_repeatable_read_transaction_each_and_write_nothing() {
    let fixture = world();

    parameter(&fixture, Role::Operator, 1).await.unwrap();
    limits(&fixture, Role::Operator, 1).await.unwrap();
    translations(&fixture, Role::Operator, 1).await.unwrap();

    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::RepeatableRead); 3]
    );
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn a_key_that_the_version_does_not_hold_is_not_found() {
    let fixture = world();

    let parameter = parameter(&fixture, Role::Operator, 9).await.unwrap_err();
    let limits = limits(&fixture, Role::Operator, 9).await.unwrap_err();
    let translations = translations(&fixture, Role::Operator, 9).await.unwrap_err();

    assert_eq!(parameter.code(), ErrorCode::WEB_005);
    assert!(is_missing_parameter(&parameter, 9), "{parameter:?}");
    assert!(is_missing_parameter(&limits, 9), "{limits:?}");
    assert!(is_missing_parameter(&translations, 9), "{translations:?}");
}

#[tokio::test]
async fn a_parameter_of_a_deleted_recipe_is_refused_except_for_the_role_that_restores() {
    let fixture = deleted_world();

    let engineer = parameter(&fixture, Role::ProcessEngineer, 1)
        .await
        .unwrap_err();
    let operator_limits = limits(&fixture, Role::Operator, 1).await.unwrap_err();
    let operator_translations = translations(&fixture, Role::Operator, 1).await.unwrap_err();
    let administrator = parameter(&fixture, Role::Administrator, 1).await.unwrap();

    assert_eq!(engineer.code(), ErrorCode::RCP_012);
    assert_eq!(operator_limits.code(), ErrorCode::RCP_012);
    assert_eq!(operator_translations.code(), ErrorCode::RCP_012);
    assert!(administrator.recipe.is_deleted());
    assert_eq!(administrator.parameter.key, pkey(1));
}
