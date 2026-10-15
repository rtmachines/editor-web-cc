//! The use case `DefineUdt` (`domain-model.md` section 5.1, `data-model.md` section 6).

use std::collections::BTreeSet;

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{
    DerivedTypeReason, MachineCode, MachineId, MemberName, PlantError, PlatformTypes, PlcPlatform,
    PlcType, PlcTypeId, PlcTypeRecord, RecordShape, TypeExpr, TypeMember, TypeName, TypeTree,
};
use uuid::Uuid;

use super::derived_lock::derived_type_locks;
use super::ensure_derived::DerivedTypes;
use super::resolve::{live_machine_id, require_live};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{
    AppError, Missing, PLC_TYPE_MEMBER_NAME_UNIQUE, PLC_TYPE_NAMED_UNIQUE, map_storage,
};
use crate::ports::{Isolation, NonEmpty};

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
    /// Returns `PlantError::DuplicateTypeName` (`E-PLT-015`) when a new struct uses a name that exists.
    /// Returns `PlantError::MemberTypeNotOnPlatform` (`E-PLT-006`) for a member type that is not on the platform.
    /// Returns `PlantError::DerivedTypeInvalid` (`E-PLT-013`) for any other bad member type.
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
        let (revision, supersedes) = next_revision(&rows, machine_id, &input)?;
        let platform = machine.platform();
        let mut derived = DerivedTypes::new(machine_id, platform, rows, audit_context(user, meta));
        let mut members = Vec::with_capacity(input.members.len());
        for (name, expr) in &input.members {
            let type_id = derived
                .ensure_expr(tx.types(), expr)
                .await
                .map_err(|error| member_error(error, &input.name, name, platform))?;
            members.push(TypeMember {
                name: name.clone(),
                type_id,
            });
        }
        let row = PlcType::try_from(PlcTypeRecord {
            id: PlcTypeId::from(Uuid::new_v4()),
            machine: Some(machine_id),
            platforms: BTreeSet::from([platform]),
            shape: RecordShape::Struct {
                name: input.name.clone(),
                revision,
                supersedes,
                members,
            },
        })?;
        check_tree(&derived, (machine_id, platform), &row)?;
        let after = tx.types().insert_struct(&row).await.map_err(|error| {
            map_storage(error, |constraint| match constraint {
                PLC_TYPE_NAMED_UNIQUE => Some(
                    PlantError::DuplicateTypeName {
                        type_name: input.name.clone(),
                    }
                    .into(),
                ),
                PLC_TYPE_MEMBER_NAME_UNIQUE => input.members.first().map(|(member, _)| {
                    PlantError::DuplicateMemberName {
                        type_name: input.name.clone(),
                        member: member.clone(),
                    }
                    .into()
                }),
                _ => None,
            })
        })?;
        let defined = AuditEntry::row(
            RowAction::UdtDefined,
            AuditTarget::plc_type(machine_id, row.id()),
            RowChange::Create {
                after: after.into_value(),
            },
            audit_context(user, meta),
        );
        let (_, created) = derived.into_parts();
        self.ctx
            .uow
            .commit(tx, NonEmpty::with_last(created, defined))
            .await?;
        Ok(DefineUdtOutput {
            type_id: row.id(),
            revision,
        })
    }
}

/// Return the revision of the new row and the row that it supersedes.
///
/// A new struct needs an unused name and gets revision 1. A revision needs a used name and gets
/// the highest revision plus one.
fn next_revision(
    rows: &[PlcType],
    machine: MachineId,
    input: &DefineUdtInput,
) -> Result<(u32, Option<PlcTypeId>), AppError> {
    let latest = rows
        .iter()
        .filter(|row| row.machine() == Some(machine))
        .filter_map(|row| match row.shape() {
            RecordShape::Struct { name, revision, .. } if *name == input.name => {
                Some((*revision, row.id()))
            }
            _ => None,
        })
        .max_by_key(|(revision, _)| *revision);
    match (input.mode, latest) {
        (DefineUdtMode::New, None) => Ok((1, None)),
        (DefineUdtMode::New, Some(_)) => Err(PlantError::DuplicateTypeName {
            type_name: input.name.clone(),
        }
        .into()),
        (DefineUdtMode::Revision, None) => Err(AppError::NotFound {
            entity: Missing::Type {
                code: input.code,
                name: input.name.clone(),
            },
        }),
        (DefineUdtMode::Revision, Some((revision, id))) => {
            let next = revision
                .checked_add(1)
                .ok_or_else(|| PlantError::DerivedTypeInvalid {
                    type_expr: format!("{}@{revision}", input.name),
                    reason: DerivedTypeReason::RowShape,
                })?;
            Ok((next, Some(id)))
        }
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
mod tests;
