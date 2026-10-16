//! Tests of ADR-0046 (b) and (c): a form query checks the permission of its purpose first.
//!
//! An Operator has no permission for any purpose. Each call returns `E-WEB-002` with the
//! operation of the purpose before it opens a transaction. The one `forbidden` event names the
//! command of the purpose, not the form query.

use recipe_domain::plant::MachineCode;
use serde_json::json;

use super::first_statement::{assert_refused, operator};
use super::{
    FormPurpose, MachineFormPurpose, Operation, ParameterFormPurpose, RecipeFormPurpose,
    UdtFormPurpose, UnitFormPurpose, VersionFormPurpose,
};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, meta, reference, scenarios};
use crate::plant::{
    GetMachineForm, GetUdtForm, GetUnitForm, MachineFormInput, UdtFormInput, UnitFormInput,
};
use crate::recipe::{
    GetParameterForm, GetParameterFormInput, GetRecipeForm, GetVersionForm, GetVersionFormInput,
    RecipeFormInput,
};

/// Assert the refusal of `purpose` and that the one `forbidden` event names its command.
fn assert_refused_for<T>(fixture: &Fixture, result: Result<T, AppError>, purpose: Operation) {
    assert_refused(fixture, result, purpose, purpose.name());
    let events = fixture.sink.events();
    let detail = events.first().and_then(|event| event.detail().cloned());
    assert_eq!(
        detail,
        Some(json!({ "operation": purpose.name() })),
        "{purpose}: the event names the command"
    );
}

/// Return the commands that the purposes of `P` name, in the order of `P::ALL`.
fn operations_of<P: FormPurpose>() -> Vec<Operation> {
    P::ALL.iter().map(|purpose| purpose.operation()).collect()
}

#[test]
fn each_purpose_type_names_the_purposes_of_its_form_query_in_the_catalog() {
    let types = [
        (
            MachineFormPurpose::FORM,
            operations_of::<MachineFormPurpose>(),
        ),
        (UnitFormPurpose::FORM, operations_of::<UnitFormPurpose>()),
        (UdtFormPurpose::FORM, operations_of::<UdtFormPurpose>()),
        (
            RecipeFormPurpose::FORM,
            operations_of::<RecipeFormPurpose>(),
        ),
        (
            VersionFormPurpose::FORM,
            operations_of::<VersionFormPurpose>(),
        ),
        (
            ParameterFormPurpose::FORM,
            operations_of::<ParameterFormPurpose>(),
        ),
    ];
    for (form, purposes) in types {
        assert_eq!(form.purposes(), purposes.as_slice(), "{form}");
    }
}

#[tokio::test]
async fn get_machine_form_checks_the_permission_of_each_purpose_first() {
    for &purpose in MachineFormPurpose::ALL {
        let fixture = scenarios::plant();
        let result = GetMachineForm::new(fixture.context())
            .execute(&operator(), &meta(), MachineFormInput { purpose })
            .await;
        assert_refused_for(&fixture, result, purpose.operation());
    }
}

#[tokio::test]
async fn get_unit_form_checks_the_permission_of_each_purpose_first() {
    for &purpose in UnitFormPurpose::ALL {
        let fixture = scenarios::plant();
        let input = UnitFormInput {
            purpose,
            code: MachineCode::parse("M1").unwrap(),
            parent: None,
        };
        let result = GetUnitForm::new(fixture.context())
            .execute(&operator(), &meta(), input)
            .await;
        assert_refused_for(&fixture, result, purpose.operation());
    }
}

#[tokio::test]
async fn get_udt_form_checks_the_permission_of_its_purpose_first() {
    for &purpose in UdtFormPurpose::ALL {
        let fixture = scenarios::plant();
        let input = UdtFormInput {
            purpose,
            code: MachineCode::parse("M1").unwrap(),
            from: None,
        };
        let result = GetUdtForm::new(fixture.context())
            .execute(&operator(), &meta(), input)
            .await;
        assert_refused_for(&fixture, result, purpose.operation());
    }
}

#[tokio::test]
async fn get_recipe_form_checks_the_permission_of_each_purpose_first() {
    for &purpose in RecipeFormPurpose::ALL {
        let fixture = scenarios::draft();
        let result = GetRecipeForm::new(fixture.context())
            .execute(&operator(), &meta(), RecipeFormInput { purpose })
            .await;
        assert_refused_for(&fixture, result, purpose.operation());
    }
}

#[test]
fn the_rename_purpose_of_get_recipe_form_needs_recipe_edit() {
    assert_eq!(
        RecipeFormPurpose::RenameRecipe.operation(),
        Operation::RenameRecipe
    );
    assert_eq!(
        RecipeFormPurpose::RenameRecipe.operation().requirement(),
        super::Requirement::Permission(recipe_domain::identity::Permission::RecipeEdit)
    );
}

#[tokio::test]
async fn get_version_form_checks_the_permission_of_its_purpose_first() {
    for &purpose in VersionFormPurpose::ALL {
        let fixture = scenarios::draft();
        let input = GetVersionFormInput {
            purpose,
            reference: reference("R-1"),
        };
        let result = GetVersionForm::new(fixture.context())
            .execute(&operator(), &meta(), input)
            .await;
        assert_refused_for(&fixture, result, purpose.operation());
    }
}

#[tokio::test]
async fn get_parameter_form_checks_the_permission_of_each_purpose_first() {
    for &purpose in ParameterFormPurpose::ALL {
        let fixture = scenarios::draft();
        let input = GetParameterFormInput {
            purpose,
            reference: reference("R-1"),
            number: 1,
            key: None,
            type_expr: None,
            unit: None,
        };
        let result = GetParameterForm::new(fixture.context())
            .execute(&operator(), &meta(), input)
            .await;
        assert_refused_for(&fixture, result, purpose.operation());
    }
}
