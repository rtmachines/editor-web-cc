//! The flow shared by both export routes (`data-model.md` section 6, row `ExportVersion`).

use std::sync::Arc;

use recipe_domain::audit::{AuditEntry, AuditTarget, StateAction};
use recipe_domain::export::{
    ExportError, ExportStatus, SCHEMA_VERSION, SCOPE_SCHEMA_VERSION, Sha256,
};
use recipe_domain::identity::Login;
use recipe_domain::plant::{Machine, MachineCode};
use recipe_domain::recipe::{
    Recipe, RecipeError, RecipeId, RecipeVersionId, ReferenceNumber, VersionState,
};
use recipe_domain::timestamp::Timestamp;
use serde_json::json;

use super::document::{KnownFacts, Written, read_rows, write_document};
use super::output::ExportedVersion;
use super::scope::{ExportScope, ScopeAudit};
use crate::auth::{CurrentUser, RequestMeta};
use crate::blocking::ExportPermit;
use crate::context::{AppContext, audit_context};
use crate::error::{AppError, Missing, StorageError};
use crate::ports::{DocumentWriter, Isolation, NonEmpty, SharedVersion, Transaction};

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

/// Check that the state allows the export on this route, and return the status of the document.
///
/// The function solves one problem: step 2 and the lock of step 3 apply the same rule, because
/// another request can change the state between them. The flow converts the state only here.
fn check_exportable(
    state: VersionState,
    target: Target<'_>,
    reference: &ReferenceNumber,
    number: u32,
) -> Result<ExportStatus, ExportError> {
    match (state, target) {
        (VersionState::Approved, _) => Ok(ExportStatus::Approved),
        (VersionState::Retired, Target::Number(..)) => Ok(ExportStatus::Retired),
        (_, Target::Approved(_)) => Err(ExportError::NoApprovedVersion {
            reference: reference.clone(),
        }),
        (_, Target::Number(..)) => Err(ExportError::VersionNotExportable {
            reference: reference.clone(),
            number,
        }),
    }
}

/// Check that the recipe belongs to the machine that the route names (ADR-0109 (h) and (j)).
///
/// The function solves one problem: the Basic route names a machine, and a recipe of another
/// machine must look like an unknown recipe. Both give the same `NotFound`, so a client cannot
/// probe for the references of other machines. The machine column of a recipe never changes, so
/// one check before the lock is enough.
async fn check_machine(
    tx: &mut dyn Transaction,
    recipe: &Recipe,
    code: MachineCode,
) -> Result<(), AppError> {
    let machine = tx.plant().export_machine_by_code(code).await?;
    if machine.is_some_and(|machine| machine.id() == recipe.machine()) {
        return Ok(());
    }
    Err(AppError::NotFound {
        entity: Missing::Recipe(recipe.reference().clone()),
    })
}

/// Read the recipe in a `ReadOnly` transaction and choose the version (step 2).
async fn find(
    ctx: &AppContext,
    target: Target<'_>,
    machine: Option<MachineCode>,
) -> Result<Found, AppError> {
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
    if let Some(code) = machine {
        check_machine(tx.as_mut(), &recipe, code).await?;
    }
    pick(recipe, target)
}

/// Build the entry `VersionExported` (`data-model.md` section 10).
///
/// A unit export sets the schema version `"2"` and adds `scope_unit_path` and `scope_sha256`
/// (ADR-0109 (ab)). An export of the whole version has neither key.
fn exported_entry(
    (recipe, version): (RecipeId, RecipeVersionId),
    (hash, scope): (Sha256, Option<&ScopeAudit>),
    (user, meta): (&CurrentUser, &RequestMeta),
) -> AuditEntry {
    let after = match scope {
        None => json!({
            "version_id": version.to_string(),
            "schema_version": SCHEMA_VERSION,
            "content_sha256": hash.to_string(),
        }),
        Some(scope) => json!({
            "version_id": version.to_string(),
            "schema_version": SCOPE_SCHEMA_VERSION,
            "content_sha256": hash.to_string(),
            "scope_unit_path": scope.unit_path.to_string(),
            "scope_sha256": scope.sha256.to_string(),
        }),
    };
    AuditEntry::state(
        StateAction::VersionExported,
        AuditTarget::recipe_version(recipe, version),
        after,
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

/// The locked version after the second state check.
struct Checked {
    shared: SharedVersion,
    found: Found,
    status: ExportStatus,
    machine: Machine,
}

/// Which pick of the approved version a lock follows (ADR-0164).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pick {
    /// The pick of step 2. A lost approved state ends the transaction and starts the second pick.
    First,
    /// The second pick. A lost approved state is an error, and no third pick runs.
    Second,
}

/// Lock the version `FOR SHARE`, read the recipe again, check the state, and read the machine.
///
/// These are steps 1 and 2 of design D6 (c) inside the transaction of the command. The function
/// returns `None` when the route `/approved` of the first pick finds a state other than
/// `Approved` under the lock. Another request changed the approved version after step 2.
async fn lock_and_check(
    tx: &mut dyn Transaction,
    (target, pick): (Target<'_>, Pick),
    found: Found,
) -> Result<Option<Checked>, AppError> {
    let shared: SharedVersion = tx
        .recipes()
        .lock_version_shared(found.version)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::version(found.recipe.reference(), found.number),
        })?;
    let found = reread_recipe(tx, target, found).await?;
    let approved_route = matches!(target, Target::Approved(_));
    if approved_route && pick == Pick::First && shared.state != VersionState::Approved {
        return Ok(None);
    }
    let status = check_exportable(shared.state, target, found.recipe.reference(), found.number)?;
    let machine_id = found.recipe.machine();
    let machine =
        tx.plant()
            .machine_by_id(machine_id)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::MachineId(machine_id),
            })?;
    Ok(Some(Checked {
        shared,
        found,
        status,
        machine,
    }))
}

/// Read the instant and the login of the approval of the locked version (step 3).
///
/// An approved or a retired version always has both. A missing value is corrupt data.
///
/// # Errors
///
/// Returns `StorageError::Other` (`E-DB-008`) when the row, the instant, or the login is missing.
async fn read_approval(
    tx: &mut dyn Transaction,
    version: RecipeVersionId,
) -> Result<(Timestamp, Login), AppError> {
    let detail = tx
        .recipe_reads()
        .version_detail(version)
        .await?
        .ok_or_else(|| StorageError::invariant("the locked version has no row"))?;
    let approved_at = detail
        .approved_at
        .ok_or_else(|| StorageError::invariant("an exported version has no approval time"))?;
    let approved_by = detail
        .approved_by
        .ok_or_else(|| StorageError::invariant("an exported version has no approver"))?;
    Ok((approved_at, approved_by))
}

/// The result of the timed part of the export: the open transaction and the written document.
struct Prepared {
    tx: Box<dyn Transaction>,
    recipe: RecipeId,
    version: RecipeVersionId,
    reference: ReferenceNumber,
    number: u32,
    status: ExportStatus,
    written: Written,
}

/// The result of one attempt to lock and write.
enum Attempt {
    /// The document is written. The transaction is open.
    Written(Box<Prepared>),
    /// The first pick lost its approved state. The attempt returns the permit that it did not use.
    Lost(ExportPermit),
}

/// Lock and check the version, read its rows, and write the document (steps 1 to 5).
///
/// The function returns the open transaction. The caller commits it after the limit
/// `timeout.export`, and a timeout or an error drops the transaction, which rolls it back.
/// The blocking task of step 5 takes `permit` only after the state check (ADR-0056 (c)).
/// The first pick of the route `/approved` can return `Attempt::Lost`. The function then drops the
/// transaction and holds no lock (ADR-0164).
async fn lock_and_write(
    ctx: &AppContext,
    (writer, user): (&Arc<dyn DocumentWriter>, &CurrentUser),
    (target, found, scope): (Target<'_>, Found, &ExportScope),
    (pick, permit): (Pick, ExportPermit),
) -> Result<Attempt, AppError> {
    let mut tx = ctx.uow.begin(Isolation::ReadCommitted).await?;
    let Some(checked) = lock_and_check(tx.as_mut(), (target, pick), found).await? else {
        return Ok(Attempt::Lost(permit));
    };
    let (version, number) = (checked.found.version, checked.found.number);
    let reference = checked.found.recipe.reference().clone();
    let (approved_at, approved_by) = read_approval(tx.as_mut(), version).await?;
    let rows = read_rows(tx.as_mut(), &checked.machine, (&reference, version, number)).await?;
    if let ExportScope::Unit(path) = scope {
        rows.require_unit(path)?;
    }
    let facts = KnownFacts {
        exported_at: ctx.clock.now(),
        exported_by: user.login().clone(),
        status: checked.status,
        approved_at,
        approved_by,
        machine_name: checked.machine.name().clone(),
        recipe_name: checked.found.recipe.name().clone(),
    };
    let task = (ctx.settings.blocking, permit);
    let stored = checked.shared.content_sha256;
    let written = write_document(
        task,
        Arc::clone(writer),
        (rows, facts),
        (stored, scope.clone()),
    )
    .await?;
    Ok(Attempt::Written(Box::new(Prepared {
        tx,
        recipe: checked.shared.recipe,
        version,
        reference,
        number,
        status: checked.status,
        written,
    })))
}

/// Pick, lock, and write. The route `/approved` picks a second time after a lost approved state.
///
/// The function solves one problem: an approval can retire the picked version between step 2 and
/// the lock of step 3. The second pick reads the recipe again and locks the new version. It
/// skips the machine check, because the machine of a recipe never changes. A third pick never
/// runs, so a second loss gives `E-EXP-001` (ADR-0164, ADR-0015: one version lock at a time).
async fn pick_lock_and_write(
    ctx: &AppContext,
    who: (&Arc<dyn DocumentWriter>, &CurrentUser),
    (target, machine, scope): (Target<'_>, Option<MachineCode>, &ExportScope),
    permit: ExportPermit,
) -> Result<Prepared, AppError> {
    let found = find(ctx, target, machine).await?;
    let first = lock_and_write(ctx, who, (target, found, scope), (Pick::First, permit)).await?;
    let permit = match first {
        Attempt::Written(prepared) => return Ok(*prepared),
        Attempt::Lost(permit) => permit,
    };
    let found = find(ctx, target, None).await?;
    let reference = found.recipe.reference().clone();
    let second = lock_and_write(ctx, who, (target, found, scope), (Pick::Second, permit)).await?;
    match second {
        Attempt::Written(prepared) => Ok(*prepared),
        Attempt::Lost(_) => Err(ExportError::NoApprovedVersion { reference }.into()),
    }
}

/// Commit the entry `VersionExported` and build the result, outside `timeout.export`.
async fn commit_export(
    ctx: &AppContext,
    (user, meta): (&CurrentUser, &RequestMeta),
    prepared: Prepared,
) -> Result<ExportedVersion, AppError> {
    let Prepared {
        tx,
        recipe,
        version,
        reference,
        number,
        status,
        written:
            Written {
                document,
                hash,
                scope,
                has_rule_fault,
            },
    } = prepared;
    let entry = exported_entry((recipe, version), (hash, scope.as_ref()), (user, meta));
    ctx.uow.commit(tx, NonEmpty::new(entry)).await?;
    Ok(ExportedVersion {
        document,
        reference,
        number,
        status,
        content_sha256: hash,
        has_rule_fault,
    })
}

/// Run the export (`E-WEB-009` with the stage `Export`, `E-WEB-011` with the stage `export`).
///
/// The function solves one problem: both routes read, lock, check, write, and audit in the same
/// way. `timeout.export` covers the wait for a permit of `export.permits`, the reads, the locks,
/// the rebuild, the hash compare, and the writer. The wait for the permit comes before the first
/// read opens a transaction (ADR-0056 (b)). The second pick of the route `/approved` runs inside
/// that limit and keeps the permit (ADR-0164). The commit runs after that limit, so a client that
/// gets `E-WEB-009` finds no entry. The caller has checked the permission of its route.
///
/// # Errors
///
/// Returns `NotFound` (`E-WEB-005`) for an unknown recipe or version number, and for a recipe
/// that does not belong to `machine` (ADR-0109 (h)). `machine` is `None` on the session routes.
/// Returns `NotFound` also when the unit of `scope` names no usable unit (ADR-0109 (ja)). The
/// check runs before the blocking task.
/// Returns `ExportError` (`E-EXP-001` to `E-EXP-005`) and `RecipeError::RecipeDeleted`
/// (`E-RCP-012`). Returns `Timeout` (`E-WEB-009`), `TaskFailed` (`E-WEB-011`), and `Storage`
/// errors.
pub(super) async fn export_version(
    ctx: &AppContext,
    writer: &Arc<dyn DocumentWriter>,
    who: (&CurrentUser, &RequestMeta),
    (target, machine, scope): (Target<'_>, Option<MachineCode>, &ExportScope),
) -> Result<ExportedVersion, AppError> {
    let prepared = ctx
        .settings
        .blocking
        .export(async {
            let permit = ctx.export_gate.acquire().await?;
            let flow = (target, machine, scope);
            pick_lock_and_write(ctx, (writer, who.0), flow, permit).await
        })
        .await
        .inspect_err(|error| count_hash_mismatch(ctx, error))?;
    commit_export(ctx, who, prepared).await
}

/// Count the result `E-EXP-002` for the job `alerts` (ADR-0060 (z)).
///
/// The function solves one problem: a hash mismatch shows that the stored rows of the version
/// changed. The flow decides it before the commit, so this is the one place that counts it.
fn count_hash_mismatch(ctx: &AppContext, error: &AppError) {
    if matches!(
        error,
        AppError::Export(ExportError::ContentHashMismatch { .. })
    ) {
        ctx.counters.record_export_hash_mismatch();
    }
}
