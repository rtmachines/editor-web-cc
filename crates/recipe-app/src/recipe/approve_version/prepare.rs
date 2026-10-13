//! Steps 1 to 3 of design D12: the reads before the transaction and the check with `Blocking`.

use recipe_domain::export::{ExportHeader, HashedBody, Sha256};
use recipe_domain::plant::{LiveUnits, Machine, MachineId, PlatformTypes, PlcType, TypeTrees};
use recipe_domain::recipe::{
    ApprovalOutcome, ApprovalReady, Draft, RecipeId, RecipeVersionId, StoredDraft,
};

use super::ApproveVersionInput;
use crate::context::AppContext;
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, Transaction};

/// What the root of the recipe says about the version.
enum Root {
    /// An earlier request approved the version. The use case ends with this outcome.
    Replay(ApprovalOutcome),
    /// The version is a live draft that an approval can take.
    Approvable {
        recipe: RecipeId,
        version: RecipeVersionId,
    },
}

/// The result of the steps before the transaction.
pub(super) enum Prepared {
    /// The use case ends with this outcome and opens no write transaction.
    Done(ApprovalOutcome),
    /// The draft passed the checks. The transaction can start.
    Ready(Ready),
}

/// A draft that passed the checks before the transaction.
pub(super) struct Ready {
    /// The recipe of the draft.
    pub(super) recipe: RecipeId,
    /// The token that owns the checked draft (ADR-0029).
    pub(super) token: ApprovalReady,
    /// The content hash of the hashed body.
    pub(super) hash: Sha256,
}

/// What the check needs, moved into the blocking closure.
struct Snapshot {
    draft: Draft,
    units: LiveUnits,
    types: Vec<PlcType>,
    header: ExportHeader,
    machine: MachineId,
    allow_outdated: bool,
}

/// Read the root and apply the replay rule and the state rule (design D12 step 1).
///
/// The function solves one problem: the replay, a retired version, and a deleted recipe must
/// give their own answers before any draft read. The order is the replay and then the state
/// (`api.md` section 4).
async fn read_root(
    tx: &mut dyn Transaction,
    input: &ApproveVersionInput,
) -> Result<Root, AppError> {
    let recipe = tx
        .recipes()
        .recipe_by_reference(&input.reference)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::Recipe(input.reference.clone()),
        })?;
    let header = recipe
        .version(input.number)
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::version(&input.reference, input.number),
        })?;
    if let Some(outcome) = recipe.approval_replay(input.number, input.expected_row_version) {
        return Ok(Root::Replay(outcome));
    }
    recipe.check_approvable(input.number)?;
    Ok(Root::Approvable {
        recipe: recipe.id(),
        version: header.id(),
    })
}

/// Read the root again after a draft that is no longer live (design D12 step 2).
///
/// The function solves one problem: a request that lost a race with another approval must
/// get `AlreadyApproved`, not `E-RCP-002` (`api.md` section 4). It returns `None` when the
/// root still names a live draft. The caller then reports the error of the draft.
async fn read_root_again(
    ctx: &AppContext,
    input: &ApproveVersionInput,
) -> Result<Option<ApprovalOutcome>, AppError> {
    let mut tx = ctx.uow.begin(Isolation::ReadOnly).await?;
    Ok(match read_root(tx.as_mut(), input).await? {
        Root::Replay(outcome) => Some(outcome),
        Root::Approvable { .. } => None,
    })
}

/// Read the draft, the machine, the live units, and the type rows (design D12 step 2).
async fn read_snapshot(
    tx: &mut dyn Transaction,
    draft: Draft,
    allow_outdated: bool,
) -> Result<Snapshot, AppError> {
    let machine_id = draft.machine();
    let machine: Machine =
        tx.plant()
            .machine_by_id(machine_id)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::machine_id(machine_id),
            })?;
    let units = tx
        .units()
        .live_units(machine_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::machine_id(machine_id),
        })?;
    let types = tx.types().type_rows(machine_id).await?;
    let header = ExportHeader {
        machine: machine.code(),
        platform: machine.platform(),
        reference: draft.reference().clone(),
        version: draft.number(),
    };
    Ok(Snapshot {
        draft,
        units,
        types,
        header,
        machine: machine_id,
        allow_outdated,
    })
}

/// Check the draft for the approval and hash the body (design D12 step 3).
///
/// The function runs on a blocking thread. The check consumes the draft and returns the token.
fn check_and_hash(snapshot: Snapshot) -> Result<(ApprovalReady, Sha256), AppError> {
    let Snapshot {
        draft,
        units,
        types,
        header,
        machine,
        allow_outdated,
    } = snapshot;
    let types = PlatformTypes::collect(machine, header.platform, types)?;
    let trees = TypeTrees::resolve(draft.parameters().iter().map(|p| p.plc_type()), &types);
    let token = draft.check_for_approval(&units, &trees, allow_outdated)?;
    let body = HashedBody::build(&header, token.draft().parameters(), &units, &trees)?;
    let hash = body.sha256()?;
    Ok((token, hash))
}

/// Run the steps before the transaction (design D12 steps 1 to 3).
///
/// # Errors
///
/// Returns `NotFound` (`E-WEB-005`) for an unknown recipe or version number.
/// Returns the domain errors of the root, of `Draft::load`, and of `check_for_approval`.
/// Returns `Timeout` (`E-WEB-009`) or `TaskFailed` (`E-WEB-011`) when the check does not complete.
pub(super) async fn prepare(
    ctx: &AppContext,
    input: &ApproveVersionInput,
) -> Result<Prepared, AppError> {
    let mut tx = ctx.uow.begin(Isolation::ReadOnly).await?;
    let (recipe, version) = match read_root(tx.as_mut(), input).await? {
        Root::Replay(outcome) => return Ok(Prepared::Done(outcome)),
        Root::Approvable { recipe, version } => (recipe, version),
    };
    let stored: StoredDraft =
        tx.drafts()
            .read_draft(version)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::version(&input.reference, input.number),
            })?;
    let draft = match Draft::load(stored) {
        Ok(draft) => draft,
        Err(error) => {
            drop(tx);
            return match read_root_again(ctx, input).await? {
                Some(outcome) => Ok(Prepared::Done(outcome)),
                None => Err(error.into()),
            };
        }
    };
    let snapshot = read_snapshot(tx.as_mut(), draft, input.allow_outdated).await?;
    drop(tx);
    let (token, hash) = ctx
        .settings
        .blocking
        .validate(move || check_and_hash(snapshot))
        .await??;
    Ok(Prepared::Ready(Ready {
        recipe,
        token,
        hash,
    }))
}
