//! The flow shared by both export routes (`data-model.md` section 6, row `ExportVersion`).

use recipe_domain::audit::{AuditEntry, AuditTarget, StateAction};
use recipe_domain::export::{ExportError, ExportHeader, HashedBody, Sha256};
use recipe_domain::plant::{Machine, PlatformTypes, TypeTrees};
use recipe_domain::recipe::{
    Parameter, Recipe, RecipeError, RecipeId, RecipeVersionId, ReferenceNumber, VersionState,
};
use serde_json::json;

use super::output::ExportedVersion;
use crate::auth::{CurrentUser, RequestMeta};
use crate::blocking::Blocking;
use crate::context::{AppContext, audit_context};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, NonEmpty, SharedVersion, Transaction};

/// The value of `schema_version` in the document and in the audit entry (`api.md` section 6.1).
const SCHEMA_VERSION: &str = "1";

/// The version that an export route names.
#[derive(Clone, Copy, Debug)]
pub(super) enum Target<'a> {
    /// The route `/approved`.
    Approved(&'a ReferenceNumber),
    /// The route `/versions/{n}`.
    Number(&'a ReferenceNumber, u32),
}

/// What step 2 found: the recipe and the version to lock.
struct Found {
    recipe: Recipe,
    version: RecipeVersionId,
    number: u32,
}

/// Choose the version of the route from the recipe (step 2).
///
/// The function solves one problem: the two routes name the version in different ways and
/// give different errors for a deleted recipe (`reliability.md` section 1.2, note of `E-RCP-012`).
fn pick(recipe: Recipe, target: Target<'_>) -> Result<Found, AppError> {
    let reference = recipe.reference().clone();
    let header = match target {
        Target::Approved(_) => {
            recipe
                .approved_version()
                .ok_or_else(|| ExportError::NoApprovedVersion {
                    reference: reference.clone(),
                })?
        }
        Target::Number(_, number) => {
            let header = recipe.version(number).ok_or_else(|| AppError::NotFound {
                entity: Missing::version(&reference, number),
            })?;
            if recipe.is_deleted() {
                return Err(RecipeError::RecipeDeleted { reference }.into());
            }
            header
        }
    };
    check_exportable(header.state(), target, &reference, header.number())?;
    let (version, number) = (header.id(), header.number());
    Ok(Found {
        recipe,
        version,
        number,
    })
}

/// Check that the state allows the export on this route.
///
/// The function solves one problem: step 2 and the lock of step 3 apply the same rule, because
/// another request can change the state between them.
fn check_exportable(
    state: VersionState,
    target: Target<'_>,
    reference: &ReferenceNumber,
    number: u32,
) -> Result<(), ExportError> {
    match (state, target) {
        (VersionState::Approved, _) | (VersionState::Retired, Target::Number(..)) => Ok(()),
        (_, Target::Approved(_)) => Err(ExportError::NoApprovedVersion {
            reference: reference.clone(),
        }),
        (_, Target::Number(..)) => Err(ExportError::VersionNotExportable {
            reference: reference.clone(),
            number,
        }),
    }
}

/// Read the recipe in a `ReadOnly` transaction and choose the version (step 2).
async fn find(ctx: &AppContext, target: Target<'_>) -> Result<Found, AppError> {
    let reference = match target {
        Target::Approved(reference) | Target::Number(reference, _) => reference,
    };
    let mut tx = ctx.uow.begin(Isolation::ReadOnly).await?;
    let recipe = tx
        .recipes()
        .recipe_by_reference(reference)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::Recipe(reference.clone()),
        })?;
    pick(recipe, target)
}

/// Rebuild the hashed body of the locked version (step 3).
///
/// The function reads the rows in the transaction. Then the build of the body and its
/// SHA-256 run in `spawn_blocking`, so the limit `timeout.export` can stop the wait.
async fn rebuild(
    tx: &mut dyn Transaction,
    blocking: Blocking,
    found: &Found,
    machine: &Machine,
) -> Result<(HashedBody, Sha256), AppError> {
    let machine_id = machine.id();
    let parameters: Vec<Parameter> = tx
        .recipes()
        .version_parameters(found.version)
        .await?
        .into_iter()
        .map(Parameter::from)
        .collect();
    let units = tx
        .units()
        .live_units(machine_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::MachineId(machine_id),
        })?;
    let rows = tx.types().type_rows(machine_id).await?;
    let types = PlatformTypes::collect(machine_id, machine.platform(), rows)?;
    let trees = TypeTrees::resolve(parameters.iter().map(Parameter::plc_type), &types);
    let header = ExportHeader {
        machine: machine.code(),
        platform: machine.platform(),
        reference: found.recipe.reference().clone(),
        version: found.number,
    };
    blocking
        .rebuild_export(move || -> Result<(HashedBody, Sha256), AppError> {
            let body = HashedBody::build(&header, &parameters, &units, &trees)?;
            let hash = body.sha256()?;
            Ok((body, hash))
        })
        .await?
}

/// Build the entry `VersionExported` (`data-model.md` section 10).
fn exported_entry(
    (recipe, version): (RecipeId, RecipeVersionId),
    hash: Sha256,
    (user, meta): (&CurrentUser, &RequestMeta),
) -> AuditEntry {
    AuditEntry::state(
        StateAction::VersionExported,
        AuditTarget::recipe_version(recipe, version),
        json!({
            "version_id": version.to_string(),
            "schema_version": SCHEMA_VERSION,
            "content_sha256": hash.to_string(),
        }),
        audit_context(user, meta),
    )
}

/// Read the recipe header again under the lock of the version (step 3).
///
/// The function solves one problem: a delete or a rename can commit between the read of step 2
/// and the lock. The deleted flag of the route by number and the recipe name must come from
/// the state that the lock protects (design D10).
async fn reread_recipe(
    tx: &mut dyn Transaction,
    target: Target<'_>,
    found: Found,
) -> Result<Found, AppError> {
    let reference = found.recipe.reference().clone();
    let recipe = tx
        .recipes()
        .recipe_by_reference(&reference)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::Recipe(reference.clone()),
        })?;
    if matches!(target, Target::Number(..)) && recipe.is_deleted() {
        return Err(RecipeError::RecipeDeleted { reference }.into());
    }
    Ok(Found { recipe, ..found })
}

/// The result of the timed part of the export: the open transaction and the checked body.
struct Prepared {
    tx: Box<dyn Transaction>,
    shared: SharedVersion,
    found: Found,
    machine: Machine,
    body: HashedBody,
    hash: Sha256,
}

/// Lock the version `FOR SHARE`, rebuild its body, and compare the hash.
///
/// The function returns the open transaction. The caller commits it after the limit
/// `timeout.export`, and a timeout drops the transaction, which rolls it back.
async fn lock_and_rebuild(
    ctx: &AppContext,
    target: Target<'_>,
    found: Found,
) -> Result<Prepared, AppError> {
    let mut tx = ctx.uow.begin(Isolation::ReadCommitted).await?;
    let shared: SharedVersion = tx
        .recipes()
        .lock_version_shared(found.version)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::version(found.recipe.reference(), found.number),
        })?;
    let found = reread_recipe(tx.as_mut(), target, found).await?;
    let reference = found.recipe.reference();
    check_exportable(shared.state, target, reference, found.number)?;
    let machine_id = found.recipe.machine();
    let machine =
        tx.plant()
            .machine_by_id(machine_id)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::MachineId(machine_id),
            })?;
    let (body, hash) = rebuild(tx.as_mut(), ctx.settings.blocking, &found, &machine).await?;
    if shared.content_sha256 != Some(hash) {
        return Err(ExportError::ContentHashMismatch {
            reference: found.recipe.reference().clone(),
            number: found.number,
        }
        .into());
    }
    Ok(Prepared {
        tx,
        shared,
        found,
        machine,
        body,
        hash,
    })
}

/// Commit the entry `VersionExported` and build the result, outside `timeout.export`.
async fn commit_export(
    ctx: &AppContext,
    (user, meta): (&CurrentUser, &RequestMeta),
    prepared: Prepared,
) -> Result<ExportedVersion, AppError> {
    let Prepared {
        tx,
        shared,
        found,
        machine,
        body,
        hash,
    } = prepared;
    let entry = exported_entry((shared.recipe, found.version), hash, (user, meta));
    ctx.uow.commit(tx, NonEmpty::new(entry)).await?;
    Ok(ExportedVersion {
        body,
        content_sha256: hash,
        status: shared.state.status(),
        machine_name: machine.name().clone(),
        recipe_name: found.recipe.name().clone(),
        approved_by: shared.approved_by,
        approved_at: shared.approved_at,
        exported_at: ctx.clock.now(),
        exported_by: user.login().clone(),
    })
}

/// Run the export (`E-WEB-009` with the stage `Export`, `E-WEB-011` with the stage `export`).
///
/// The function solves one problem: both routes read, lock, check, and audit in the same way.
/// `timeout.export` covers the read, the locks, the rebuild, and the hash compare. The commit
/// runs after that limit, so a client that gets `E-WEB-009` finds no entry. The caller has
/// checked the permission of its route.
///
/// # Errors
///
/// Returns `NotFound` (`E-WEB-005`) for an unknown recipe or version number.
/// Returns `ExportError` (`E-EXP-001`, `E-EXP-002`, `E-EXP-003`) and `RecipeError::RecipeDeleted`
/// (`E-RCP-012`). Returns `Timeout` (`E-WEB-009`) and `Storage` errors.
pub(super) async fn export_version(
    ctx: &AppContext,
    who: (&CurrentUser, &RequestMeta),
    target: Target<'_>,
) -> Result<ExportedVersion, AppError> {
    let prepared = ctx
        .settings
        .blocking
        .export(async {
            let found = find(ctx, target).await?;
            lock_and_rebuild(ctx, target, found).await
        })
        .await?;
    commit_export(ctx, who, prepared).await
}
