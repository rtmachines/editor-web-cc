//! Queries of the Plant Model (design D3).
//!
//! Each query is a struct over an `AppContext`. Its first statement checks the permission. Then
//! it reads in one `ReadOnly` transaction, and it ends that transaction before it builds the result.

use std::cmp::Reverse;

use recipe_domain::plant::{Machine, MachineCode, PlatformTypes, PlcTypeId, RecordShape, TypeName};

use crate::context::AppContext;
use crate::error::{AppError, Missing, StorageError};
use crate::ports::{Isolation, PlantRepository};

mod machines;
#[cfg(test)]
mod machines_tests;
#[cfg(test)]
mod source_tests;
mod type_text;
#[cfg(test)]
mod type_text_tests;
mod types;
#[cfg(test)]
mod types_tests;
#[cfg(test)]
mod udt_form_tests;
mod units;
#[cfg(test)]
mod units_tests;
mod usage;
#[cfg(test)]
mod usage_tests;
#[cfg(test)]
mod world;

pub use machines::{
    GetMachine, GetMachineForm, GetMachineInput, ListMachines, ListMachinesInput, MachineForm,
    MachineFormInput, MachineList, MachineRef, MachineView,
};
pub(crate) use type_text::type_text;
pub use types::{
    GetTypeTree, GetUdtForm, ListTypes, TypeList, TypeListRow, TypeView, TypesInput, UdtForm,
    UdtFormInput,
};
pub use units::{
    GetUnit, GetUnitForm, GetUnitTree, UnitForm, UnitFormInput, UnitRef, UnitTreeInput, UnitView,
};
pub use usage::{GetTypeUsage, TypeUsage, TypeUsageInput, TypeUsageLine};

/// A struct revision of a machine, as a URL names it.
///
/// The type solves one problem: `GetTypeTree` and `GetTypeUsage` read the same three values
/// from a URL, and one struct keeps them together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeRef {
    /// The code of the live machine.
    pub code: MachineCode,
    /// The name of the struct.
    pub name: TypeName,
    /// The revision, or `None` for the latest revision.
    pub revision: Option<u32>,
}

/// Compute the offset and the limit of a page. The limit is one row more than the page.
///
/// The extra row tells whether a next page exists.
///
/// # Errors
///
/// Returns a `StorageError` when the offset has no product or the page size has no successor.
fn page_window(page: u32, size: u32) -> Result<(u64, u32), StorageError> {
    // The product of two `u32` values always fits `u64`, so `None` never happens.
    let offset = u64::from(page)
        .checked_mul(u64::from(size))
        .ok_or_else(|| StorageError::invariant("the page offset has no product"))?;
    let limit = size
        .checked_add(1)
        .ok_or_else(|| StorageError::invariant("the page size has no successor"))?;
    Ok((offset, limit))
}

/// Read the live machine with `code`, or return `NotFound` (`E-WEB-005`).
///
/// Every query of the Plant Model that a `/machines/{code}` route runs uses this function.
async fn live_machine(
    plant: &mut dyn PlantRepository,
    code: MachineCode,
) -> Result<Machine, AppError> {
    let found = plant.machine_by_code(code).await?;
    found.ok_or(AppError::NotFound {
        entity: Missing::Machine(code),
    })
}

/// Read the live machine with `code` in one `ReadOnly` transaction.
///
/// The transaction ends when the function returns, so the caller builds its result after it.
async fn read_live_machine(ctx: &AppContext, code: MachineCode) -> Result<Machine, AppError> {
    let mut tx = ctx.uow.begin(Isolation::ReadOnly).await?;
    live_machine(tx.plant(), code).await
}

/// Return the revision number and the row of each revision of the struct `name`, the latest first.
fn revisions_of(types: &PlatformTypes, name: &TypeName) -> Vec<(u32, PlcTypeId)> {
    let mut revisions: Vec<(u32, PlcTypeId)> = types
        .rows()
        .filter_map(|row| match row.shape() {
            RecordShape::Struct {
                name: row_name,
                revision,
                ..
            } if row_name == name => Some((*revision, row.id())),
            _ => None,
        })
        .collect();
    revisions.sort_by_key(|(number, _)| Reverse(*number));
    revisions
}

/// Pick the revision that `type_ref` names from `revisions`, or the latest revision.
///
/// Returns the revision number and the row. `revisions` comes from `revisions_of`.
fn pick_revision(
    revisions: &[(u32, PlcTypeId)],
    type_ref: &TypeRef,
) -> Result<(u32, PlcTypeId), AppError> {
    let found = match type_ref.revision {
        Some(wanted) => revisions.iter().find(|(number, _)| *number == wanted),
        None => revisions.first(),
    };
    found.copied().ok_or_else(|| AppError::NotFound {
        entity: missing_type(type_ref, !revisions.is_empty()),
    })
}

/// Name the missing entity: the revision when the struct exists, and the struct when it does not.
fn missing_type(type_ref: &TypeRef, struct_exists: bool) -> Missing {
    match type_ref.revision {
        Some(revision) if struct_exists => Missing::TypeRevision {
            code: type_ref.code,
            name: type_ref.name.clone(),
            revision,
        },
        _ => Missing::Type {
            code: type_ref.code,
            name: type_ref.name.clone(),
        },
    }
}
