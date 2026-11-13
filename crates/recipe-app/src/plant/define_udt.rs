//! The use case `DefineUdt` (`domain-model.md` section 5.1, `data-model.md` section 6).

use std::collections::BTreeSet;

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{
    DerivedTypeReason, MachineCode, MachineId, MemberName, PlantError, PlatformTypes, PlcPlatform,
    PlcType, PlcTypeId, PlcTypeRecord, RecordShape, TypeExpr, TypeMember, TypeName, TypeTree,
};
use uuid::Uuid;

use super::current_type::require_current_type;
use super::derived_lock::derived_type_locks;
use super::ensure_derived::DerivedTypes;
use super::resolve::{live_machine_id, require_live};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{
    AppError, Missing, PLC_TYPE_MEMBER_NAME_CI_UNIQUE, PLC_TYPE_MEMBER_NAME_UNIQUE,
    PLC_TYPE_NAMED_UNIQUE, PLC_TYPE_STRUCT_NAME_CI_UNIQUE, map_storage,
};
use crate::ports::{Isolation, NonEmpty, RowImage, TypeCatalog};

/// Whether `DefineUdt` creates a new struct or a new revision of a struct.
///
/// The type solves one problem: the two routes of `api.md` section 2.3 run one use case,
/// and the server, not the client, sets `supersedes` for a revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefineUdtMode {
    /// A new struct (`POST /machines/{code}/types`). The name must be unused.
    New,
    /// A new revision of a struct (`POST /machines/{code}/types/{name}/revisions`).
    Revision,
}

/// The parsed request of `DefineUdt`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefineUdtInput {
    /// The code of the live machine in the URL.
    pub code: MachineCode,
    /// The name of the struct.
    pub name: TypeName,
    /// The members in their order, each with the type expression of its type.
    pub members: Vec<(MemberName, TypeExpr)>,
    /// Whether the command creates a new struct or a new revision.
    pub mode: DefineUdtMode,
}

/// The result of `DefineUdt`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefineUdtOutput {
    /// The identifier of the new struct row.
    pub type_id: PlcTypeId,
    /// The revision of the new struct row.
    pub revision: u32,
}

/// Defines a new struct, or a new revision of a struct, on a live machine.
///
/// The use case solves one problem: a struct is immutable, so an edit creates a new row, and
/// two concurrent commands must not create the same derived types or the same revision twice.
/// It locks the machine `FOR SHARE` and then the class-2 advisory keys, in the order of
/// `data-model.md` section 1.4, before its first insert (ADR-0015).
#[derive(Clone)]
pub struct DefineUdt {
    ctx: AppContext,
}

impl UseCase for DefineUdt {
    const OPERATION: Operation = Operation::DefineUdt;
}

impl DefineUdt {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Define the struct.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `UdtManage`.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code, when the machine is archived under the lock, or when a revision names a struct without revision.
    /// Returns `PlantError::DuplicateTypeName` (`E-PLT-015`) when a new struct uses a name that exists in any letter case. The error names the stored spelling when the command finds the stored name in its read of the type rows. When a parallel command commits the other spelling first, the unique index refuses the row and the error names the input spelling.
    /// Returns `PlantError::MemberTypeNotOnPlatform` (`E-PLT-006`) for a member type that is not on the platform.
    /// Returns `PlantError::DerivedTypeInvalid` (`E-PLT-013`) for any other bad member type.
    /// Returns `PlantError::RevisionUnchanged` (`E-PLT-052`) when a revision has the members of the latest revision.
    /// Returns `PlantError::StructHasNoMembers` (`E-PLT-016`) or `DuplicateMemberName` (`E-PLT-021`).
    /// Returns `PlantError::TypeCycle` (`E-PLT-005`) or `TypeTooLarge` (`E-PLT-018`) for a bad tree.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: DefineUdtInput,
    ) -> Result<DefineUdtOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        for (_, expr) in &input.members {
            require_current_type(expr)?;
        }
        let machine_id = live_machine_id(&self.ctx, input.code).await?;
        let keys = derived_type_locks(
            Some(&input.name),
            input.members.iter().map(|(_, expr)| expr),
        );
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let locked = tx.plant().lock_machine_shared(machine_id).await?;
        let machine = require_live(locked, input.code)?.state;
        tx.types().lock_derived_types(machine_id, &keys).await?;
        let rows = tx.types().type_rows(machine_id).await?;
        let next = next_revision(&rows, machine_id, &input)?;
        let platform = machine.platform();
        let mut derived = DerivedTypes::new(machine_id, platform, rows, audit_context(user, meta));
        let members = ensure_members(&mut derived, tx.types(), &input, platform).await?;
        ensure_changed(&input, next.latest.as_ref(), &members)?;
        let row = struct_row(
            &input,
            (machine_id, platform),
            (next.number, next.supersedes),
            members,
        )?;
        check_tree(&derived, (machine_id, platform), &row)?;
        let after = insert_struct_row(tx.types(), &row, &input).await?;
        let defined = udt_defined_entry(user, meta, machine_id, &row, after);
        let (_, created) = derived.into_parts();
        self.ctx
            .uow
            .commit(tx, NonEmpty::with_last(created, defined))
            .await?;
        Ok(DefineUdtOutput {
            type_id: row.id(),
            revision: next.number,
        })
    }
}

/// Resolve the type expression of each member to a type row, in the order of the members.
///
/// A member type that is not on the platform gets the member name (`E-PLT-006`).
/// An array of an array on `S7_1200` and `S7_1500` is such a type (ADR-0148). The check runs
/// before the type is created, so a refused member leaves no new type row.
async fn ensure_members(
    derived: &mut DerivedTypes,
    catalog: &mut dyn TypeCatalog,
    input: &DefineUdtInput,
    platform: PlcPlatform,
) -> Result<Vec<TypeMember>, AppError> {
    let mut members = Vec::with_capacity(input.members.len());
    for (name, expr) in &input.members {
        expr.require_no_nested_array(platform)
            .map_err(|error| member_error(error.into(), &input.name, name, platform))?;
        let type_id = derived
            .ensure_expr(catalog, expr)
            .await
            .map_err(|error| member_error(error, &input.name, name, platform))?;
        members.push(TypeMember {
            name: name.clone(),
            type_id,
        });
    }
    Ok(members)
}

/// Build the new struct row with a fresh identifier.
fn struct_row(
    input: &DefineUdtInput,
    (machine, platform): (MachineId, PlcPlatform),
    (revision, supersedes): (u32, Option<PlcTypeId>),
    members: Vec<TypeMember>,
) -> Result<PlcType, AppError> {
    let row = PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(Uuid::new_v4()),
        machine: Some(machine),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Struct {
            name: input.name.clone(),
            revision,
            supersedes,
            members,
        },
    })?;
    Ok(row)
}

/// Insert the struct row and map the four unique constraints to their plant errors.
///
/// The error names the spelling of the input, because the adapter does not read the stored row.
async fn insert_struct_row(
    catalog: &mut dyn TypeCatalog,
    row: &PlcType,
    input: &DefineUdtInput,
) -> Result<RowImage, AppError> {
    catalog.insert_struct(row).await.map_err(|error| {
        map_storage(error, |constraint| match constraint {
            PLC_TYPE_NAMED_UNIQUE | PLC_TYPE_STRUCT_NAME_CI_UNIQUE => Some(
                PlantError::DuplicateTypeName {
                    type_name: input.name.clone(),
                }
                .into(),
            ),
            PLC_TYPE_MEMBER_NAME_UNIQUE | PLC_TYPE_MEMBER_NAME_CI_UNIQUE => {
                input.members.first().map(|(member, _)| {
                    PlantError::DuplicateMemberName {
                        type_name: input.name.clone(),
                        member: member.clone(),
                    }
                    .into()
                })
            }
            _ => None,
        })
    })
}

/// Build the audit entry of a defined struct from the image of the inserted row.
fn udt_defined_entry(
    user: &CurrentUser,
    meta: &RequestMeta,
    machine: MachineId,
    row: &PlcType,
    after: RowImage,
) -> AuditEntry {
    AuditEntry::row(
        RowAction::UdtDefined,
        AuditTarget::plc_type(machine, row.id()),
        RowChange::Create {
            after: after.into_value(),
        },
        audit_context(user, meta),
    )
}

/// The revision number of the new row, the row that it supersedes, and the latest revision.
struct NextRevision {
    /// The revision number of the new row.
    number: u32,
    /// The row that the new row supersedes. It is `None` for a new struct.
    supersedes: Option<PlcTypeId>,
    /// The number and the members of the latest revision. It is `None` for a new struct.
    latest: Option<(u32, Vec<TypeMember>)>,
}

/// The latest struct row of the machine whose name matches the input, as a tuple.
///
/// The tuple holds the revision, the row id, the stored name, and the members. A new struct matches
/// a name in any letter case. A revision matches a name in the exact case.
fn latest_struct_row<'a>(
    rows: &'a [PlcType],
    machine: MachineId,
    input: &DefineUdtInput,
) -> Option<(u32, PlcTypeId, &'a TypeName, &'a [TypeMember])> {
    let same_name = |name: &TypeName| match input.mode {
        DefineUdtMode::New => name.as_str().eq_ignore_ascii_case(input.name.as_str()),
        DefineUdtMode::Revision => *name == input.name,
    };
    rows.iter()
        .filter(|row| row.machine() == Some(machine))
        .filter_map(|row| match row.shape() {
            RecordShape::Struct {
                name,
                revision,
                members,
                ..
            } if same_name(name) => Some((*revision, row.id(), name, members.as_slice())),
            _ => None,
        })
        .max_by_key(|(revision, ..)| *revision)
}

/// Return the revision of the new row, the row that it supersedes, and the latest revision.
///
/// A new struct needs a name that no struct of the machine has in any letter case, and gets
/// revision 1. The duplicate error names the stored spelling. A revision needs a used name in the
/// exact case and gets the highest revision plus one (ADR-0143 (g), (j)). The result holds the
/// members of the latest revision, so the use case reads the rows once (ADR-0159 (c)).
fn next_revision(
    rows: &[PlcType],
    machine: MachineId,
    input: &DefineUdtInput,
) -> Result<NextRevision, AppError> {
    let latest = latest_struct_row(rows, machine, input);
    match (input.mode, latest) {
        (DefineUdtMode::New, None) => Ok(NextRevision {
            number: 1,
            supersedes: None,
            latest: None,
        }),
        (DefineUdtMode::New, Some((_, _, stored, _))) => Err(PlantError::DuplicateTypeName {
            type_name: stored.clone(),
        }
        .into()),
        (DefineUdtMode::Revision, None) => Err(AppError::NotFound {
            entity: Missing::Type {
                code: input.code,
                name: input.name.clone(),
            },
        }),
        (DefineUdtMode::Revision, Some((revision, id, _, members))) => {
            let next = revision
                .checked_add(1)
                .ok_or_else(|| PlantError::DerivedTypeInvalid {
                    type_expr: format!("{}@{revision}", input.name),
                    reason: DerivedTypeReason::RowShape,
                })?;
            Ok(NextRevision {
                number: next,
                supersedes: Some(id),
                latest: Some((revision, members.to_vec())),
            })
        }
    }
}

/// Refuse a revision whose members equal the members of the latest revision (ADR-0159 (b)).
///
/// The lists are equal when they have the same count and, at each position, the same member name in
/// the exact case and the same type row. A new struct has no latest revision and passes.
fn ensure_changed(
    input: &DefineUdtInput,
    latest: Option<&(u32, Vec<TypeMember>)>,
    members: &[TypeMember],
) -> Result<(), PlantError> {
    match latest {
        Some((revision, stored)) if stored.as_slice() == members => {
            Err(PlantError::RevisionUnchanged {
                type_name: input.name.clone(),
                revision: *revision,
            })
        }
        _ => Ok(()),
    }
}

/// Map `E-PLT-013` with the reason `NotOnPlatform` to `E-PLT-006` with the member name.
///
/// This is the one exception of `domain-model.md` section 2.1: the member name is the useful detail.
fn member_error(
    error: AppError,
    type_name: &TypeName,
    member: &MemberName,
    platform: PlcPlatform,
) -> AppError {
    match error {
        AppError::Plant(PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::NotOnPlatform,
            ..
        }) => PlantError::MemberTypeNotOnPlatform {
            type_name: type_name.clone(),
            member: member.clone(),
            platform,
        }
        .into(),
        other => other,
    }
}

/// Resolve the tree of the new struct over the rows and the new row.
///
/// The check finds a cycle through existing structs (`E-PLT-005`) and a tree above a size limit
/// (`E-PLT-018`). It then compares the struct names on each path, so a struct cannot hold an
/// earlier revision of itself (ADR-0038).
fn check_tree(
    derived: &DerivedTypes,
    (machine, platform): (MachineId, PlcPlatform),
    row: &PlcType,
) -> Result<(), PlantError> {
    let mut rows = derived.rows().to_vec();
    rows.push(row.clone());
    let types = PlatformTypes::collect(machine, platform, rows)?;
    TypeTree::resolve(row.id(), &types)?.check_struct_names()
}

#[cfg(test)]
mod name_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod unchanged_tests;
