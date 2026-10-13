//! The unit and type steps that `AddParameter` and `UpdateParameterDefinition` share (design D11).

use recipe_domain::audit::{AuditContext, AuditEntry};
use recipe_domain::plant::{LiveUnit, PlantError, PlcTypeId, TypeExpr, TypeTrees, UnitPath};
use recipe_domain::recipe::{ParameterTag, RecipeError};

use super::draft_session::OpenDraft;
use crate::error::{AppError, Missing, PARAMETER_TAG_UNIQUE, StorageError, map_storage};
use crate::plant::{DerivedTypes, derived_type_locks};

/// The type of a definition command, and what the command needs to commit it.
///
/// The type solves one problem: `AddParameter` and `UpdateParameterDefinition` resolve a
/// `TypeExpr` in the same way, and both need the new type id, the trees with the new rows, and
/// the `DerivedTypeCreated` entries.
pub(crate) struct ResolvedType {
    /// The row of the type expression.
    pub(crate) type_id: PlcTypeId,
    /// The trees of the types of every parameter of the draft and of the new type.
    pub(crate) trees: TypeTrees,
    /// The `DerivedTypeCreated` entries, in creation order.
    pub(crate) created: Vec<AuditEntry>,
}

/// Take `unit FOR SHARE` at `path` and require a live unit (`E-PLT-020`, `E-PLT-009`).
///
/// The function reads the units after the lock, so the state that it checks is the locked state.
///
/// # Errors
///
/// Returns `PlantError::UnitNotFound` (`E-PLT-020`) when the machine has no unit at `path`.
/// Returns `PlantError::UnitArchived` (`E-PLT-009`) when the unit is archived.
/// Returns `NotFound` when the machine row is missing, and `AppError::Storage` on a database failure.
pub(crate) async fn lock_live_unit(
    open: &mut OpenDraft,
    path: &UnitPath,
) -> Result<LiveUnit, AppError> {
    let machine = open.draft.machine();
    open.tx
        .units()
        .lock_unit_shared(machine, path)
        .await?
        .ok_or_else(|| PlantError::UnitNotFound { path: path.clone() })?;
    let units = open
        .tx
        .units()
        .live_units(machine)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::machine_id(machine),
        })?;
    Ok(units.require(path)?)
}

/// Take the class-2 advisory locks of `expr`, resolve it, and resolve the trees of the draft.
///
/// The locks come before the first type insert (`data-model.md` section 1.4). The function reads
/// the type rows after the locks, creates the missing derived rows, and resolves the trees of
/// every parameter type of the draft and of the new type over the rows including the new ones.
///
/// # Errors
///
/// Returns `PlantError::DerivedTypeInvalid` (`E-PLT-013`) or `PlantError::TypeTooLarge`
/// (`E-PLT-018`) for a bad expression. Returns `AppError::Storage` on a database failure.
pub(crate) async fn resolve_type(
    open: &mut OpenDraft,
    expr: &TypeExpr,
    context: AuditContext,
) -> Result<ResolvedType, AppError> {
    let machine = open.draft.machine();
    let keys = derived_type_locks(None, [expr]);
    open.tx.types().lock_derived_types(machine, &keys).await?;
    let rows = open.tx.types().type_rows(machine).await?;
    let mut derived = DerivedTypes::new(machine, open.draft.platform(), rows, context);
    let type_id = derived.ensure_expr(open.tx.types(), expr).await?;
    let types = derived.platform_types()?;
    let trees = TypeTrees::resolve(
        open.draft
            .parameters()
            .iter()
            .map(|parameter| parameter.plc_type())
            .chain([type_id]),
        &types,
    );
    let (_, created) = derived.into_parts();
    Ok(ResolvedType {
        type_id,
        trees,
        created,
    })
}

/// Map a failed write of a parameter tag to the domain error of the command.
///
/// The function solves one problem: the unique index `parameter_tag_unique` is the second line
/// behind the domain check of the tag (`E-RCP-014`). `AddParameter` and `UpdateParameterDefinition`
/// both map it the same way.
pub(crate) fn map_tag_error(error: StorageError, tag: &ParameterTag, number: u32) -> AppError {
    map_storage(error, |constraint| match constraint {
        PARAMETER_TAG_UNIQUE => Some(
            RecipeError::DuplicateTag {
                tag: tag.clone(),
                number,
            }
            .into(),
        ),
        _ => None,
    })
}
