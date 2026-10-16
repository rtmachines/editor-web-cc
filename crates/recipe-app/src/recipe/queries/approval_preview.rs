//! The query `GetApprovalPreview` (`api.md` section 2.5, ADR-0049 (c), ADR-0050, design D3).
//!
//! The preview is the review step of an approval. It reads the draft, the current Approved
//! version, and one page of the diff in one `RepeatableRead` transaction. After the transaction,
//! one blocking task runs the separation of duties, the diff, and `check_for_approval` in this
//! order. The preview writes nothing.

use std::collections::BTreeMap;

use recipe_domain::plant::{LiveUnits, Machine, PlatformTypes, PlcType, TypeTrees};
use recipe_domain::recipe::{
    ApprovalPolicy, Approver, Draft, Parameter, ParameterKey, ParameterTag, Recipe, RecipeError,
    ReferenceNumber, SnapshotItem, VersionState,
};

use super::diff::{DiffChoice, DiffPage, DiffSummary};
use super::diff_page::{PagePlan, PageRead, build_page, read_page};
use super::outdated::{OutdatedParameter, RebindBudget, outdated_with_trees};
use super::{read_machine, read_recipe};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::blocking::DiffPermit;
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing, StorageError};
use crate::ports::{Isolation, ParameterRow, Transaction};

/// The parsed request of `GetApprovalPreview` (`GET /recipes/{ref}/versions/{n}/approve`).
///
/// The web layer compares the query key `rv` with [`ApprovalPreview::row_version`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalPreviewInput {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The number of the draft.
    pub number: u32,
    /// The section and the page of the diff.
    pub choice: DiffChoice,
}

/// The result of `GetApprovalPreview`.
///
/// A blocker is data, not an error. When the review of the page fails (the outdated list or the
/// diff) and a blocker exists, `diff` is `None` and `outdated` is empty (design D3). The result
/// holds the permit of the diff gate, and the holder drops the result after the page renders
/// (ADR-0050 (g)).
#[derive(Debug)]
pub struct ApprovalPreview {
    /// The recipe with the headers of all its versions.
    pub recipe: Recipe,
    /// The machine of the recipe.
    pub machine: Machine,
    /// The number of the draft.
    pub number: u32,
    /// The row version of the draft in the snapshot of the preview (ADR-0050 (h)).
    pub row_version: i64,
    /// The number of the current Approved version, which the approval retires, or `None`.
    pub retired: Option<u32>,
    /// The diff of the draft (new side) against the current Approved version (old side), or `None`.
    ///
    /// When the review of the page fails (the outdated list or the diff) and a blocker exists,
    /// `diff` is `None` and `outdated` is empty. The page then shows the header and the blockers,
    /// with no summary and no section (design D3). A review that does not fail always gives
    /// `Some`, also next to a blocker.
    pub diff: Option<DiffPage>,
    /// The blocker `E-RCP-015` of the separation of duties for the current user, or `None`.
    pub approver_blocker: Option<RecipeError>,
    /// The error of `check_for_approval` with `allow_outdated` set to `true`, or `None`.
    ///
    /// It is the error that `ApproveVersion` returns on the same state, for example a list of
    /// `E-RCP-016` entries or `E-RCP-020` with every item of the snapshot check.
    pub draft_blocker: Option<AppError>,
    /// The key of each parameter tag that `draft_blocker` names (ADR-0049 (c), design D3).
    ///
    /// The web layer uses the key to link each blocker line to the page of its parameter.
    /// The map also holds `other_tag` of `E-RCP-022` and the tag of each item of `E-RCP-020`.
    /// The map is empty when `draft_blocker` is `None` or names no tag, for example `E-RCP-008`.
    pub blocker_keys: BTreeMap<ParameterTag, ParameterKey>,
    /// The parameters whose type uses an older struct revision. They do not block (A29).
    ///
    /// When the review of the page fails (the outdated list or the diff) and a blocker exists,
    /// `diff` is `None` and `outdated` is empty (design D3).
    pub outdated: Vec<OutdatedParameter>,
    /// `true` when no blocker exists, so the page shows the approve form.
    pub may_approve: bool,
    /// The permit of the diff gate.
    pub permit: DiffPermit,
}

/// What the transaction of the preview reads.
struct PreviewRead {
    recipe: Recipe,
    machine: Machine,
    draft: Draft,
    units: LiveUnits,
    type_rows: Vec<PlcType>,
    retired: Option<u32>,
    page: PageRead,
}

/// Read the root and run the state check in the order of `ApproveVersion` (ADR-0049 (d)).
///
/// # Errors
///
/// Returns `NotFound` (`E-WEB-005`) for an unknown recipe or version number. Returns
/// `E-RCP-012` for a deleted recipe and `E-RCP-007` for a version that is not a live draft.
async fn read_draft_of(
    tx: &mut dyn Transaction,
    input: &ApprovalPreviewInput,
) -> Result<(Recipe, Draft), AppError> {
    let not_found = || AppError::NotFound {
        entity: Missing::version(&input.reference, input.number),
    };
    let recipe = read_recipe(tx, &input.reference).await?;
    let version = recipe.version(input.number).ok_or_else(not_found)?.id();
    recipe.check_approvable(input.number)?;
    let stored = tx
        .drafts()
        .read_draft(version)
        .await?
        .ok_or_else(not_found)?;
    Ok((recipe, Draft::load(stored)?))
}

/// Read the draft, the machine, the live units, the type rows, and one page of the diff.
async fn read_preview(
    tx: &mut dyn Transaction,
    input: &ApprovalPreviewInput,
    size: u32,
) -> Result<PreviewRead, AppError> {
    let (recipe, draft) = read_draft_of(tx, input).await?;
    let machine = read_machine(tx.plant(), &recipe).await?;
    let units = tx
        .units()
        .live_units(machine.id())
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::machine_id(machine.id()),
        })?;
    let type_rows = tx.types().type_rows(machine.id()).await?;
    let approved = recipe
        .versions()
        .iter()
        .find(|header| header.state() == VersionState::Approved);
    let old = approved.map(|header| header.id());
    let sides = (old, draft.version());
    let page = read_page(tx.diff_reads(), sides, false, (input.choice, size)).await?;
    Ok(PreviewRead {
        retired: approved.map(|header| header.number()),
        recipe,
        machine,
        draft,
        units,
        type_rows,
        page,
    })
}

/// Shows the review step of an approval: the diff, the blockers, and the outdated parameters.
///
/// The query solves one problem: the approver must see the changes and every reason that stops
/// the approval before the `POST`. It runs the domain checks of `ApproveVersion` on one snapshot
/// and writes nothing (ADR-0049 (c), ADR-0050 (f)).
#[derive(Clone)]
pub struct GetApprovalPreview {
    ctx: AppContext,
}

impl UseCase for GetApprovalPreview {
    const OPERATION: Operation = Operation::GetApprovalPreview;
}

impl GetApprovalPreview {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the preview of the approval of draft `number`.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks `VersionApprove`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotDraft` (`E-RCP-007`) when the version is not a live draft.
    /// Returns `AppError::Timeout` (`E-WEB-009`) when the permit wait and the checks pass `timeout.validation`.
    /// Returns `AppError::TaskFailed` (`E-WEB-011`) with the stage `Diff` when the task fails.
    /// Returns `AppError::Plant` when a type of a pair does not resolve and no blocker exists.
    /// With a blocker, the result holds the blockers and no diff (design D3).
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ApprovalPreviewInput,
    ) -> Result<ApprovalPreview, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let blocking = self.ctx.settings.blocking;
        let deadline = blocking.diff_deadline()?;
        let permit = self.ctx.diff_gate.acquire(deadline).await?;
        let mut tx = self.ctx.uow.begin(Isolation::RepeatableRead).await?;
        let read = read_preview(tx.as_mut(), &input, self.ctx.settings.pages.diff).await?;
        drop(tx);
        let approver = Approver {
            user: user.user(),
            login: user.login().clone(),
        };
        let rules = (
            self.ctx.settings.approval_policy,
            self.ctx.settings.diff.leaf_rows,
        );
        let (checked, permit) = blocking
            .diff(deadline, permit, move || check(read, &approver, rules))
            .await?;
        let checked = checked?;
        Ok(ApprovalPreview {
            may_approve: checked.approver_blocker.is_none() && checked.draft_blocker.is_none(),
            recipe: checked.recipe,
            machine: checked.machine,
            number: input.number,
            row_version: checked.row_version,
            retired: checked.retired,
            diff: checked.diff,
            approver_blocker: checked.approver_blocker,
            draft_blocker: checked.draft_blocker,
            blocker_keys: checked.blocker_keys,
            outdated: checked.outdated,
            permit,
        })
    }
}

/// What the blocking task of the preview gives back.
struct Checked {
    recipe: Recipe,
    machine: Machine,
    row_version: i64,
    retired: Option<u32>,
    diff: Option<DiffPage>,
    approver_blocker: Option<RecipeError>,
    draft_blocker: Option<AppError>,
    blocker_keys: BTreeMap<ParameterTag, ParameterKey>,
    outdated: Vec<OutdatedParameter>,
}

/// Return the parameter tags that one error of `check_for_approval` names.
///
/// An item of `E-RCP-020` for the whole version names no tag.
fn tags_of(error: &RecipeError) -> Vec<&ParameterTag> {
    match error {
        RecipeError::S7AddressOverlap { tag, other_tag, .. } => vec![tag, other_tag],
        RecipeError::ValidationFailed { items } => items
            .iter()
            .filter_map(|item| match item {
                SnapshotItem::Parameter { tag, .. } | SnapshotItem::UnknownUnit { tag, .. } => {
                    Some(tag)
                }
                SnapshotItem::Version { .. } => None,
            })
            .collect(),
        RecipeError::LimitMissing { tag, .. }
        | RecipeError::S7AddressMisaligned { tag, .. }
        | RecipeError::S7AddressRequired { tag, .. }
        | RecipeError::S7AddressNotAllowed { tag, .. }
        | RecipeError::S7AddressWidthMismatch { tag, .. }
        | RecipeError::TypeRevisionOutdated { tag, .. } => vec![tag],
        _ => Vec::new(),
    }
}

/// Run `check_for_approval` and find the key of each tag that its error names (ADR-0049 (c)).
///
/// The check consumes the draft, so the function first copies the tag and the key of each
/// parameter. A tag that the draft does not hold stays out of the map.
fn draft_check(
    draft: Draft,
    units: &LiveUnits,
    trees: &TypeTrees,
) -> (Option<AppError>, BTreeMap<ParameterTag, ParameterKey>) {
    let index: BTreeMap<ParameterTag, ParameterKey> = draft
        .parameters()
        .into_iter()
        .map(|parameter| (parameter.tag().clone(), parameter.key()))
        .collect();
    let blocker = draft
        .check_for_approval(units, trees, true)
        .err()
        .map(AppError::from);
    let named: Vec<&ParameterTag> = match &blocker {
        Some(AppError::Recipe(error)) => tags_of(error),
        Some(AppError::Recipes(errors)) => errors.iter().flat_map(tags_of).collect(),
        _ => Vec::new(),
    };
    let keys = named
        .into_iter()
        .filter_map(|tag| index.get_key_value(tag))
        .map(|(tag, key)| (tag.clone(), *key))
        .collect();
    (blocker, keys)
}

/// Return the parameter rows of the draft that `outdated_with_trees` reads, in tag order.
fn rows_of(draft: &Draft, machine: &Machine) -> Result<Vec<ParameterRow>, AppError> {
    draft
        .parameters()
        .into_iter()
        .map(|parameter| {
            let unit = machine
                .units()
                .find(|candidate| candidate.id() == parameter.unit())
                .ok_or_else(|| {
                    StorageError::invariant("the unit of a parameter is not a unit of its machine")
                })?;
            Ok(ParameterRow {
                key: parameter.key(),
                unit: parameter.unit(),
                unit_path: unit.path().clone(),
                tag: parameter.tag().clone(),
                name_en: parameter.name_en().clone(),
                ordinal: parameter.ordinal(),
                plc_type: parameter.plc_type(),
                s7_address: parameter.s7_address(),
            })
        })
        .collect()
}

/// The page of the diff after the read: the plan, the summary, and the old side of its pairs.
struct LoadedPage {
    plan: PagePlan,
    summary: DiffSummary,
    old: Vec<Parameter>,
}

impl From<PageRead> for LoadedPage {
    fn from(page: PageRead) -> Self {
        let PageRead {
            plan,
            summary,
            sides,
        } = page;
        Self {
            plan,
            summary,
            old: sides.old.into_iter().map(Parameter::from).collect(),
        }
    }
}

/// Run the diff of the page with the new side from the loaded draft (design D3).
fn diff_of(
    draft: &Draft,
    page: LoadedPage,
    context: (&Machine, &PlatformTypes, &TypeTrees),
    leaf_rows: u32,
) -> Result<DiffPage, AppError> {
    let LoadedPage { plan, summary, old } = page;
    let new: Vec<Parameter> = draft
        .parameters()
        .into_iter()
        .filter(|parameter| plan.keys().contains(&parameter.key()))
        .cloned()
        .collect();
    build_page(plan, summary, (&old, &new), context, leaf_rows)
}

/// The parts of the preview that come from the diff: the rows of the page and the outdated list.
struct Review {
    diff: DiffPage,
    outdated: Vec<OutdatedParameter>,
}

/// Build the outdated parameters and the diff of the page (design D3 step 4).
///
/// Both steps use the type trees of `context`, so the preview resolves the draft types once.
fn review(
    draft: &Draft,
    page: LoadedPage,
    context: (&Machine, &PlatformTypes, &TypeTrees),
    leaf_rows: u32,
) -> Result<Review, AppError> {
    let budget = RebindBudget {
        computes: false,
        bound: 0,
    };
    let (machine, types, trees) = context;
    let rows = rows_of(draft, machine)?;
    let outdated = outdated_with_trees(&rows, (types, trees), budget)?.parameters;
    let diff = diff_of(draft, page, context, leaf_rows)?;
    Ok(Review { diff, outdated })
}

/// Run the checks of design D3 step 4 on a blocking thread, in this order.
///
/// 1. `Draft::check_approver` for the current user,
/// 2. the domain diff of the page,
/// 3. `check_for_approval` with `allow_outdated` set to `true`. It consumes the draft.
///
/// When the review of the page fails (the outdated list or the diff) and a blocker exists, the
/// result holds the blockers, no diff, and no outdated parameters. Without a blocker, the
/// function returns the error of the review (design D3).
fn check(
    read: PreviewRead,
    approver: &Approver,
    (policy, leaf_rows): (ApprovalPolicy, u32),
) -> Result<Checked, AppError> {
    let PreviewRead {
        recipe,
        machine,
        draft,
        units,
        type_rows,
        retired,
        page,
    } = read;
    let approver_blocker = draft.check_approver(approver, &policy).err();
    let types = PlatformTypes::collect(machine.id(), machine.platform(), type_rows)?;
    let page = LoadedPage::from(page);
    // One resolve serves the outdated list, the diff, and `check_for_approval`.
    // Each step looks up its own types only.
    let olds = page.old.iter().map(Parameter::plc_type);
    let trees = TypeTrees::resolve(
        draft.parameters().iter().map(|p| p.plc_type()).chain(olds),
        &types,
    );
    let context = (&machine, &types, &trees);
    let reviewed = review(&draft, page, context, leaf_rows);
    let row_version = draft.row_version();
    let (draft_blocker, blocker_keys) = draft_check(draft, &units, &trees);
    let (diff, outdated) = match reviewed {
        Ok(Review { diff, outdated }) => (Some(diff), outdated),
        Err(error) if approver_blocker.is_none() && draft_blocker.is_none() => return Err(error),
        Err(_) => (None, Vec::new()),
    };
    Ok(Checked {
        recipe,
        machine,
        row_version,
        retired,
        diff,
        approver_blocker,
        draft_blocker,
        blocker_keys,
        outdated,
    })
}
