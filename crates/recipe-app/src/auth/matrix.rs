//! Test that the code matches the permission matrix of `api.md` section 3.3 (design D7).
//!
//! The tables below are copied by hand from section 3.3. They do not call
//! `Operation::requirement`, so the test compares the code with the document.

use recipe_domain::identity::{Permission, Role};

use super::{Operation, Requirement, authorize};
use crate::error::AppError;
use crate::memory::MemorySink;
use crate::memory::fixtures::{actor, meta};

use self::Access::{Allowed, Denied};

/// The expected result of one (operation, role) pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Access {
    Allowed,
    Denied,
}

/// The roles of the matrix, in the column order of the table.
const ROLES: [Role; 3] = [Role::Administrator, Role::ProcessEngineer, Role::Operator];

/// One row per operation of `api.md` section 3.3, in the order of `Operation::ALL`.
///
/// The columns are Administrator, ProcessEngineer, and Operator.
const MATRIX: [(Operation, [Access; 3]); 68] = [
    (Operation::CreateMachine, [Allowed, Allowed, Denied]),
    (Operation::RenameMachine, [Allowed, Allowed, Denied]),
    (Operation::ArchiveMachine, [Allowed, Allowed, Denied]),
    (Operation::AddUnit, [Allowed, Allowed, Denied]),
    (Operation::RenameUnit, [Allowed, Allowed, Denied]),
    (Operation::ArchiveUnit, [Allowed, Allowed, Denied]),
    (Operation::DefineUdt, [Allowed, Allowed, Denied]),
    (Operation::CreateRecipe, [Allowed, Allowed, Denied]),
    (Operation::RenameRecipe, [Allowed, Allowed, Denied]),
    (Operation::DeleteRecipe, [Allowed, Allowed, Denied]),
    (Operation::RestoreRecipe, [Allowed, Denied, Denied]),
    (Operation::CreateVersion, [Allowed, Allowed, Denied]),
    (Operation::DiscardDraft, [Allowed, Allowed, Denied]),
    (Operation::ApproveVersion, [Allowed, Allowed, Denied]),
    (Operation::RetireVersion, [Allowed, Denied, Denied]),
    (Operation::AddParameter, [Allowed, Allowed, Denied]),
    (
        Operation::UpdateParameterDefinition,
        [Allowed, Allowed, Denied],
    ),
    (Operation::SetParameterValue, [Allowed, Allowed, Denied]),
    (Operation::ReplaceLimits, [Allowed, Allowed, Denied]),
    (Operation::UpsertTranslation, [Allowed, Allowed, Denied]),
    (Operation::RemoveTranslation, [Allowed, Allowed, Denied]),
    (Operation::RemoveParameter, [Allowed, Allowed, Denied]),
    (Operation::RebindTypeRevision, [Allowed, Allowed, Denied]),
    (Operation::Login, [Allowed, Allowed, Allowed]),
    (Operation::Logout, [Allowed, Allowed, Allowed]),
    (Operation::CreateUser, [Allowed, Denied, Denied]),
    (Operation::SetUserRole, [Allowed, Denied, Denied]),
    (Operation::DeactivateUser, [Allowed, Denied, Denied]),
    (Operation::ActivateUser, [Allowed, Denied, Denied]),
    (Operation::ResetPassword, [Allowed, Denied, Denied]),
    (Operation::ChangeOwnPassword, [Allowed, Allowed, Allowed]),
    (Operation::UnlockLogin, [Allowed, Denied, Denied]),
    (
        Operation::ExportApprovedVersion,
        [Allowed, Allowed, Allowed],
    ),
    (Operation::ExportVersionByNumber, [Allowed, Allowed, Denied]),
    (Operation::ListMachines, [Allowed, Allowed, Allowed]),
    (Operation::GetMachine, [Allowed, Allowed, Allowed]),
    (Operation::GetUnitTree, [Allowed, Allowed, Allowed]),
    (Operation::GetUnit, [Allowed, Allowed, Allowed]),
    (Operation::ListTypes, [Allowed, Allowed, Allowed]),
    (Operation::GetTypeTree, [Allowed, Allowed, Allowed]),
    (Operation::GetTypeUsage, [Allowed, Allowed, Allowed]),
    (Operation::ListRecipes, [Allowed, Allowed, Allowed]),
    (Operation::ListDeletedRecipes, [Allowed, Denied, Denied]),
    (Operation::GetRecipe, [Allowed, Allowed, Allowed]),
    (Operation::ListVersions, [Allowed, Allowed, Allowed]),
    (Operation::GetVersion, [Allowed, Allowed, Allowed]),
    (Operation::DiffVersions, [Allowed, Allowed, Allowed]),
    (Operation::GetRecipeHistory, [Allowed, Allowed, Allowed]),
    (Operation::ListParameters, [Allowed, Allowed, Allowed]),
    (Operation::GetParameter, [Allowed, Allowed, Allowed]),
    (Operation::GetLimits, [Allowed, Allowed, Allowed]),
    (Operation::GetTranslations, [Allowed, Allowed, Allowed]),
    (Operation::ListUsers, [Allowed, Denied, Denied]),
    (Operation::GetUser, [Allowed, Denied, Denied]),
    (Operation::ListAuditEntries, [Allowed, Allowed, Denied]),
    (Operation::GetAuditEntry, [Allowed, Allowed, Denied]),
    (Operation::ListSecurityEvents, [Allowed, Denied, Denied]),
    (Operation::GetOwnProfile, [Allowed, Allowed, Allowed]),
    (Operation::GetMachineForm, [Allowed, Allowed, Denied]),
    (Operation::GetUnitForm, [Allowed, Allowed, Denied]),
    (Operation::GetUdtForm, [Allowed, Allowed, Denied]),
    (Operation::GetRecipeForm, [Allowed, Allowed, Denied]),
    (Operation::GetVersionForm, [Allowed, Allowed, Denied]),
    (Operation::GetParameterForm, [Allowed, Allowed, Denied]),
    (Operation::GetApprovalPreview, [Allowed, Allowed, Denied]),
    (Operation::GetUserForm, [Allowed, Denied, Denied]),
    (Operation::GetLoginForm, [Allowed, Allowed, Allowed]),
    (Operation::GetOwnPasswordForm, [Allowed, Allowed, Allowed]),
];

/// The purposes of the form queries and the permission of each one, copied from ADR-0046 (c).
const PURPOSES: [(Operation, Operation, Permission); 16] = [
    (
        Operation::GetMachineForm,
        Operation::CreateMachine,
        Permission::MachineManage,
    ),
    (
        Operation::GetMachineForm,
        Operation::RenameMachine,
        Permission::MachineManage,
    ),
    (
        Operation::GetUnitForm,
        Operation::AddUnit,
        Permission::UnitManage,
    ),
    (
        Operation::GetUnitForm,
        Operation::RenameUnit,
        Permission::UnitManage,
    ),
    (
        Operation::GetUdtForm,
        Operation::DefineUdt,
        Permission::UdtManage,
    ),
    (
        Operation::GetRecipeForm,
        Operation::CreateRecipe,
        Permission::RecipeCreate,
    ),
    (
        Operation::GetRecipeForm,
        Operation::RenameRecipe,
        Permission::RecipeEdit,
    ),
    (
        Operation::GetVersionForm,
        Operation::CreateVersion,
        Permission::VersionCreate,
    ),
    (
        Operation::GetParameterForm,
        Operation::AddParameter,
        Permission::ParameterEdit,
    ),
    (
        Operation::GetParameterForm,
        Operation::UpdateParameterDefinition,
        Permission::ParameterEdit,
    ),
    (
        Operation::GetParameterForm,
        Operation::SetParameterValue,
        Permission::ParameterEdit,
    ),
    (
        Operation::GetParameterForm,
        Operation::ReplaceLimits,
        Permission::ParameterEdit,
    ),
    (
        Operation::GetParameterForm,
        Operation::UpsertTranslation,
        Permission::ParameterEdit,
    ),
    (
        Operation::GetParameterForm,
        Operation::RemoveTranslation,
        Permission::ParameterEdit,
    ),
    (
        Operation::GetParameterForm,
        Operation::RemoveParameter,
        Permission::ParameterEdit,
    ),
    (
        Operation::GetParameterForm,
        Operation::RebindTypeRevision,
        Permission::ParameterEdit,
    ),
];

/// The 30 commands that stop in read-only mode (Assumption A24, design D7).
const STOPPED_IN_READ_ONLY: [Operation; 30] = [
    Operation::CreateMachine,
    Operation::RenameMachine,
    Operation::ArchiveMachine,
    Operation::AddUnit,
    Operation::RenameUnit,
    Operation::ArchiveUnit,
    Operation::DefineUdt,
    Operation::CreateRecipe,
    Operation::RenameRecipe,
    Operation::DeleteRecipe,
    Operation::RestoreRecipe,
    Operation::CreateVersion,
    Operation::DiscardDraft,
    Operation::ApproveVersion,
    Operation::RetireVersion,
    Operation::AddParameter,
    Operation::UpdateParameterDefinition,
    Operation::SetParameterValue,
    Operation::ReplaceLimits,
    Operation::UpsertTranslation,
    Operation::RemoveTranslation,
    Operation::RemoveParameter,
    Operation::RebindTypeRevision,
    Operation::CreateUser,
    Operation::SetUserRole,
    Operation::DeactivateUser,
    Operation::ActivateUser,
    Operation::ResetPassword,
    Operation::ChangeOwnPassword,
    Operation::UnlockLogin,
];

/// The commands that still run in read-only mode.
const RUNS_IN_READ_ONLY: [Operation; 4] = [
    Operation::Login,
    Operation::Logout,
    Operation::ExportApprovedVersion,
    Operation::ExportVersionByNumber,
];

/// Return true when `result` and the number of events match the expected access.
fn is_as_expected(
    operation: Operation,
    expected: Access,
    result: &Result<(), AppError>,
    events: usize,
) -> bool {
    match (expected, result) {
        (Allowed, Ok(())) => events == 0,
        (Denied, Err(AppError::Forbidden { operation: refused })) => {
            *refused == operation && events == 1
        }
        _ => false,
    }
}

#[tokio::test]
async fn every_pair_of_the_matrix_gives_the_documented_result() {
    let mut mismatches = Vec::new();
    for (operation, expected_row) in MATRIX {
        for (role, expected) in ROLES.into_iter().zip(expected_row) {
            let sink = MemorySink::default();
            let result = authorize(operation, &actor(role), &meta(), &sink).await;
            let events = sink.events().len();
            if !is_as_expected(operation, expected, &result, events) {
                mismatches.push(format!(
                    "{operation} for {role:?}: expected {expected:?}, got {result:?} with {events} event(s)"
                ));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "matrix mismatches:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn matrix_has_exactly_one_row_per_operation() {
    let mut problems = Vec::new();
    for operation in Operation::ALL {
        let rows = MATRIX.iter().filter(|(row, _)| *row == operation).count();
        if rows != 1 {
            problems.push(format!("{operation}: {rows} rows"));
        }
    }
    assert!(
        problems.is_empty(),
        "operations without exactly one row: {}",
        problems.join(", ")
    );
    assert_eq!(MATRIX.len(), Operation::ALL.len());
}

#[test]
fn every_form_query_serves_exactly_the_purposes_of_adr_0046() {
    for operation in Operation::ALL {
        let expected: Vec<Operation> = PURPOSES
            .iter()
            .filter(|(form, _, _)| *form == operation)
            .map(|(_, purpose, _)| *purpose)
            .collect();
        assert_eq!(operation.purposes(), expected.as_slice(), "{operation}");
    }
}

#[test]
fn each_purpose_needs_the_permission_of_its_row() {
    for (form, purpose, permission) in PURPOSES {
        assert_eq!(
            purpose.requirement(),
            Requirement::Permission(permission),
            "{form} for {purpose}"
        );
    }
}

#[test]
fn a_form_query_allows_a_role_that_may_run_one_of_its_purposes() {
    for operation in Operation::ALL {
        let purposes = operation.purposes();
        if purposes.is_empty() {
            continue;
        }
        for role in ROLES {
            let by_purpose = purposes
                .iter()
                .any(|purpose| purpose.requirement().allows(role));
            assert_eq!(
                operation.requirement().allows(role),
                by_purpose,
                "{operation} for {role:?}"
            );
        }
    }
}

#[test]
fn read_only_flag_matches_the_stop_list_of_design_d7() {
    let mut wrong = Vec::new();
    for operation in Operation::ALL {
        let expected = !STOPPED_IN_READ_ONLY.contains(&operation);
        if operation.runs_in_read_only() != expected {
            wrong.push(format!(
                "{operation}: expected runs_in_read_only {expected}"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "read-only mismatches: {}",
        wrong.join(", ")
    );
}

#[test]
fn every_command_outside_the_exempt_list_stops_in_read_only_mode() {
    let unlisted: Vec<_> = Operation::ALL
        .into_iter()
        .filter(|operation| operation.is_command())
        .filter(|operation| !RUNS_IN_READ_ONLY.contains(operation))
        .filter(|operation| !STOPPED_IN_READ_ONLY.contains(operation))
        .collect();
    assert!(
        unlisted.is_empty(),
        "commands without a read-only decision: {unlisted:?}"
    );
    assert!(
        STOPPED_IN_READ_ONLY
            .iter()
            .all(|operation| operation.is_command()),
        "the stop list holds a query"
    );
}
