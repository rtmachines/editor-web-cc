//! Steps 4 to 6 of design D12: the locks, the domain method, and the writes.

use recipe_domain::audit::{AuditEntry, RowAction};
use recipe_domain::export::Sha256;
use recipe_domain::plant::{PlatformTypes, TypeTrees};
use recipe_domain::recipe::{
    ApprovalOutcome, ApprovalPolicy, ApprovalReady, Approver, Draft, Recipe, RecipeId,
    RecipeVersionId, VersionHeader,
};
use serde_json::Value;

use super::prepare::Ready;
use super::{ApproveVersionInput, ApproveVersionOutput};
use crate::audit::version_entry;
use crate::auth::{CurrentUser, RequestMeta};
use crate::context::AppContext;
use crate::error::{AppError, Missing, StorageError, conflict};
use crate::ports::{Isolation, Locked, NonEmpty, RowImage, Transaction};

/// The name of the field that `VersionApproved` adds to the row image (`data-model.md` section 10).
const ALLOW_OUTDATED_FIELD: &str = "allow_outdated";

/// Check the limits and the outdated types again with the type rows of the transaction.
///
/// The function solves one problem: a struct revision can commit between the check before the
/// transaction and the locks. The row version compare of `Recipe::approve` proves that the locked
/// draft equals the checked draft, so the check uses the draft of the token (ADR-0029).
/// A stale request and a replay must win over this check. The function therefore returns the
/// result of the two domain checks as a value, and the caller reads it only after `Recipe::approve`
/// returned `Approved`.
///
/// # Errors
///
/// The outer `Err` is a failure of the type read or of the type collection.
/// The inner `Err` is `E-RCP-016` for a leaf without a limit, or `E-RCP-024` for a revision
/// that is outdated now while `allow_outdated` is `false`.
async fn recheck(
    tx: &mut dyn Transaction,
    draft: &Draft,
    allow_outdated: bool,
) -> Result<Result<(), AppError>, AppError> {
    let machine = draft.machine();
    let rows = tx.types().type_rows(machine).await?;
    let types = PlatformTypes::collect(machine, draft.platform(), rows)?;
    let trees = TypeTrees::resolve(draft.parameters().iter().map(|p| p.plc_type()), &types);
    let checked = draft
        .check_limits(&trees)
        .and_then(|()| draft.check_outdated(&trees, allow_outdated));
    Ok(checked.map_err(AppError::from))
}

/// Add the field `allow_outdated` to the image of the approved row (`data-model.md` section 10).
fn with_allow_outdated(image: RowImage, allow_outdated: bool) -> Value {
    match image.into_value() {
        Value::Object(mut fields) => {
            fields.insert(ALLOW_OUTDATED_FIELD.to_owned(), Value::Bool(allow_outdated));
            Value::Object(fields)
        }
        other => other,
    }
}

/// What the domain method decided and the writes need.
struct Approval {
    recipe: RecipeId,
    approved: RecipeVersionId,
    retired: Option<RecipeVersionId>,
    hash: Sha256,
}

/// Retire the previous approved version and return its entry (design D12 step 6).
async fn write_retirement(
    tx: &mut dyn Transaction,
    (recipe, old): (RecipeId, RecipeVersionId),
    who: (&CurrentUser, &RequestMeta),
) -> Result<AuditEntry, AppError> {
    let before = tx.recipes().lock_version(old).await?.ok_or_else(|| {
        StorageError::invariant("the root lists an approved version that has no row")
    })?;
    let after = tx.recipes().retire_version(old).await?;
    Ok(version_entry(
        RowAction::VersionRetired,
        (recipe, old),
        (before, after.into_value()),
        who,
    ))
}

/// Write the retirement, then the approval, and return the entries in this order.
async fn write_approval(
    tx: &mut dyn Transaction,
    approval: Approval,
    (input, before): (&ApproveVersionInput, RowImage),
    who: (&CurrentUser, &RequestMeta),
) -> Result<NonEmpty<AuditEntry>, AppError> {
    let retirement = match approval.retired {
        Some(old) => Some(write_retirement(tx, (approval.recipe, old), who).await?),
        None => None,
    };
    let after = tx
        .recipes()
        .approve_version(approval.approved, who.0.user(), approval.hash)
        .await?;
    let approved = version_entry(
        RowAction::VersionApproved,
        (approval.recipe, approval.approved),
        (before, with_allow_outdated(after, input.allow_outdated)),
        who,
    );
    Ok(NonEmpty::with_last(
        retirement.into_iter().collect(),
        approved,
    ))
}

/// Lock the root and then the draft row in the order of ADR-0015 (design D12 step 4).
///
/// # Errors
///
/// Returns `NotFound` (`E-WEB-005`) when the root or the draft row is gone.
async fn lock_root_and_draft(
    tx: &mut dyn Transaction,
    (recipe, token): (RecipeId, &ApprovalReady),
    input: &ApproveVersionInput,
) -> Result<(Recipe, RowImage), AppError> {
    let Locked { state, .. } =
        tx.recipes()
            .lock_recipe(recipe)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::Recipe(input.reference.clone()),
            })?;
    let before = tx
        .recipes()
        .lock_version(token.version())
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::version(&input.reference, token.number()),
        })?;
    Ok((state, before))
}

/// Run the transaction of design D12 steps 4 to 6 and commit the approval.
///
/// # Errors
///
/// Returns `E-WEB-001` for a stale row version, `E-RCP-015` for the creator or an editor, and
/// the errors of the second check. Returns `AppError::Storage` or `AppError::AuditWriteFailed`
/// when the database fails.
pub(super) async fn approve_in_transaction(
    ctx: &AppContext,
    (user, meta): (&CurrentUser, &RequestMeta),
    input: &ApproveVersionInput,
    ready: Ready,
) -> Result<ApproveVersionOutput, AppError> {
    let Ready {
        recipe: recipe_id,
        token,
        hash,
    } = ready;
    let mut tx = ctx.uow.begin(Isolation::ReadCommitted).await?;
    let (mut recipe, before) = lock_root_and_draft(tx.as_mut(), (recipe_id, &token), input).await?;
    let checked = recheck(tx.as_mut(), token.draft(), input.allow_outdated).await?;
    let approver = Approver {
        user: user.user(),
        login: user.login().clone(),
    };
    let policy = &ctx.settings.approval_policy;
    let outcome = decide(&mut recipe, token, input, (&approver, policy))?;
    let ApprovalOutcome::Approved { approved, retired } = outcome else {
        return Ok(ApproveVersionOutput {
            outcome,
            retired: None,
        });
    };
    checked?;
    let retired_number = retired_number(&recipe, retired)?;
    let approval = Approval {
        recipe: recipe_id,
        approved,
        retired,
        hash,
    };
    let entries = write_approval(tx.as_mut(), approval, (input, before), (user, meta)).await?;
    ctx.uow.commit(tx, entries).await?;
    Ok(ApproveVersionOutput {
        outcome,
        retired: retired_number,
    })
}

/// Return the number of the retired version from the headers of the locked root (ADR-0049 (e)).
///
/// # Errors
///
/// Returns `E-DB-008` when the root retired a version that it does not list.
fn retired_number(
    recipe: &Recipe,
    retired: Option<RecipeVersionId>,
) -> Result<Option<u32>, AppError> {
    retired
        .map(|old| {
            recipe
                .versions()
                .iter()
                .find(|header| header.id() == old)
                .map(VersionHeader::number)
                .ok_or_else(|| {
                    StorageError::invariant("the root retired a version that it does not list")
                        .into()
                })
        })
        .transpose()
}

/// Run `Recipe::approve` on the locked root and map its conflict (design D10).
fn decide(
    recipe: &mut Recipe,
    token: ApprovalReady,
    input: &ApproveVersionInput,
    (approver, policy): (&Approver, &ApprovalPolicy),
) -> Result<ApprovalOutcome, AppError> {
    let number = token.number();
    let outcome = recipe.approve(token, input.expected_row_version, approver, policy)?;
    conflict(outcome, input.reference.clone(), number)
}
