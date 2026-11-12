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
const ROLES: [Role; 4] = [
    Role::Administrator,
    Role::ProcessEngineer,
    Role::Operator,
    Role::ExportClient,
];

/// One row per operation of `api.md` section 3.3, in the order of `Operation::ALL`.
///
/// The columns are Administrator, ProcessEngineer, Operator, and ExportClient.
const MATRIX: [(Operation, [Access; 4]); 73] = [
    (Operation::CreateMachine, [Allowed, Allowed, Denied, Denied]),
    (Operation::RenameMachine, [Allowed, Allowed, Denied, Denied]),
    (
        Operation::SetMachineDataBlock,
        [Allowed, Allowed, Denied, Denied],
    ),
    (
        Operation::ArchiveMachine,
        [Allowed, Allowed, Denied, Denied],
    ),
    (Operation::AddUnit, [Allowed, Allowed, Denied, Denied]),
    (Operation::RenameUnit, [Allowed, Allowed, Denied, Denied]),
    (Operation::ArchiveUnit, [Allowed, Allowed, Denied, Denied]),
    (Operation::DefineUdt, [Allowed, Allowed, Denied, Denied]),
    (
        Operation::SetPlcConnection,
        [Allowed, Allowed, Denied, Denied],
    ),
    (Operation::CreateRecipe, [Allowed, Allowed, Denied, Denied]),
    (Operation::RenameRecipe, [Allowed, Allowed, Denied, Denied]),
    (Operation::DeleteRecipe, [Allowed, Allowed, Denied, Denied]),
    (Operation::RestoreRecipe, [Allowed, Denied, Denied, Denied]),
    (Operation::CreateVersion, [Allowed, Allowed, Denied, Denied]),
    (Operation::DiscardDraft, [Allowed, Allowed, Denied, Denied]),
    (
        Operation::ApproveVersion,
        [Allowed, Allowed, Denied, Denied],
    ),
    (Operation::RetireVersion, [Allowed, Denied, Denied, Denied]),
    (Operation::AddParameter, [Allowed, Allowed, Denied, Denied]),
    (
        Operation::UpdateParameterDefinition,
        [Allowed, Allowed, Denied, Denied],
    ),
    (
        Operation::SetParameterValue,
        [Allowed, Allowed, Denied, Denied],
    ),
    (Operation::ReplaceLimits, [Allowed, Allowed, Denied, Denied]),
    (
        Operation::UpsertTranslation,
        [Allowed, Allowed, Denied, Denied],
    ),
    (
        Operation::RemoveTranslation,
        [Allowed, Allowed, Denied, Denied],
    ),
    (
        Operation::RemoveParameter,
        [Allowed, Allowed, Denied, Denied],
    ),
    (
        Operation::RebindTypeRevision,
        [Allowed, Allowed, Denied, Denied],
    ),
    (Operation::Login, [Allowed, Allowed, Allowed, Denied]),
    (Operation::Logout, [Allowed, Allowed, Allowed, Denied]),
    (Operation::CreateUser, [Allowed, Denied, Denied, Denied]),
    (Operation::SetUserRole, [Allowed, Denied, Denied, Denied]),
    (Operation::DeactivateUser, [Allowed, Denied, Denied, Denied]),
    (Operation::ActivateUser, [Allowed, Denied, Denied, Denied]),
    (Operation::ResetPassword, [Allowed, Denied, Denied, Denied]),
    (
        Operation::ChangeOwnPassword,
        [Allowed, Allowed, Allowed, Denied],
    ),
    (Operation::UnlockLogin, [Allowed, Denied, Denied, Denied]),
    (
        Operation::ExportApprovedVersion,
        [Allowed, Allowed, Allowed, Allowed],
    ),
    (
        Operation::ExportVersionByNumber,
        [Allowed, Allowed, Denied, Denied],
    ),
    (
        Operation::ExportPlcConnection,
        [Allowed, Allowed, Allowed, Allowed],
    ),
    (Operation::ListMachines, [Allowed, Allowed, Allowed, Denied]),
    (Operation::GetMachine, [Allowed, Allowed, Allowed, Denied]),
    (Operation::GetUnitTree, [Allowed, Allowed, Allowed, Denied]),
    (Operation::GetUnit, [Allowed, Allowed, Allowed, Denied]),
    (Operation::ListTypes, [Allowed, Allowed, Allowed, Denied]),
    (Operation::GetTypeTree, [Allowed, Allowed, Allowed, Denied]),
    (Operation::GetTypeUsage, [Allowed, Allowed, Allowed, Denied]),
    (Operation::ListRecipes, [Allowed, Allowed, Allowed, Denied]),
    (
        Operation::ListDeletedRecipes,
        [Allowed, Denied, Denied, Denied],
    ),
    (
        Operation::ListExportableRecipes,
        [Allowed, Allowed, Allowed, Allowed],
    ),
    (Operation::GetRecipe, [Allowed, Allowed, Allowed, Denied]),
    (Operation::ListVersions, [Allowed, Allowed, Allowed, Denied]),
    (Operation::GetVersion, [Allowed, Allowed, Allowed, Denied]),
    (Operation::DiffVersions, [Allowed, Allowed, Allowed, Denied]),
    (
        Operation::GetRecipeHistory,
        [Allowed, Allowed, Allowed, Denied],
    ),
    (
        Operation::ListParameters,
        [Allowed, Allowed, Allowed, Denied],
    ),
    (Operation::GetParameter, [Allowed, Allowed, Allowed, Denied]),
    (Operation::GetLimits, [Allowed, Allowed, Allowed, Denied]),
    (
        Operation::GetTranslations,
        [Allowed, Allowed, Allowed, Denied],
    ),
    (Operation::ListUsers, [Allowed, Denied, Denied, Denied]),
    (Operation::GetUser, [Allowed, Denied, Denied, Denied]),
    (
        Operation::ListAuditEntries,
        [Allowed, Allowed, Denied, Denied],
    ),
    (Operation::GetAuditEntry, [Allowed, Allowed, Denied, Denied]),
    (
        Operation::ListSecurityEvents,
        [Allowed, Denied, Denied, Denied],
    ),
    (
        Operation::GetOwnProfile,
        [Allowed, Allowed, Allowed, Denied],
    ),
    (
        Operation::GetMachineForm,
        [Allowed, Allowed, Denied, Denied],
    ),
    (Operation::GetUnitForm, [Allowed, Allowed, Denied, Denied]),
    (Operation::GetUdtForm, [Allowed, Allowed, Denied, Denied]),
    (Operation::GetRecipeForm, [Allowed, Allowed, Denied, Denied]),
    (
        Operation::GetVersionForm,
        [Allowed, Allowed, Denied, Denied],
    ),
    (
        Operation::GetParameterForm,
        [Allowed, Allowed, Denied, Denied],
    ),
    (
        Operation::GetApprovalPreview,
        [Allowed, Allowed, Denied, Denied],
    ),
    (Operation::GetUserForm, [Allowed, Denied, Denied, Denied]),
    (Operation::GetLoginForm, [Allowed, Allowed, Allowed, Denied]),
    (
        Operation::GetOwnPasswordForm,
        [Allowed, Allowed, Allowed, Denied],
    ),
    (
        Operation::GetPlcConnectionForm,
        [Allowed, Allowed, Denied, Denied],
    ),
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

/// The 32 commands that stop in read-only mode (Assumption A24, design D7, ADR-0156 (e)).
const STOPPED_IN_READ_ONLY: [Operation; 32] = [
    Operation::CreateMachine,
    Operation::RenameMachine,
    Operation::SetMachineDataBlock,
    Operation::ArchiveMachine,
    Operation::AddUnit,
    Operation::RenameUnit,
    Operation::ArchiveUnit,
    Operation::DefineUdt,
    Operation::SetPlcConnection,
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
const RUNS_IN_READ_ONLY: [Operation; 5] = [
    Operation::Login,
    Operation::Logout,
    Operation::ExportApprovedVersion,
    Operation::ExportVersionByNumber,
    Operation::ExportPlcConnection,
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
