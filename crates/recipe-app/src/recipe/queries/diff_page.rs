//! The page core of `DiffVersions` and `GetApprovalPreview` (ADR-0050 (d), (e), design D4).
//!
//! The use case reads the change flags in its snapshot and picks the entries of one page from
//! them. An entry is one key in one section, so a key in two sections counts twice. Then it reads
//! the pairs of these keys only. After the transaction, `build_page` runs the domain diff and
//! keeps the rows of the picked entries.

use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::plant::{Machine, PlatformTypes, PlcTypeId, TypeTrees, UnitId, UnitPath};
use recipe_domain::recipe::{
    DiffKind, DiffPair, DiffRow, Parameter, ParameterKey, RecipeVersionId, diff_pairs,
};

use super::parameters::text_of;
use super::{cut_page, page_window};
use crate::error::{AppError, StorageError};
use crate::ports::{ChangeFlags, DiffReads, DiffSides};

/// The section and the page number of a diff page (ADR-0050 (e)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffChoice {
    /// The section of the page, or `None` for a page that fills the sections in their order.
    pub kind: Option<DiffKind>,
    /// The page number. The first page is page 0.
    pub page: u32,
}

/// The counts of the summary of a diff, from the change flags of every key (ADR-0050 (d)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiffSummary {
    /// The parameters of the limits section.
    pub limits: u32,
    /// The parameters of the values section.
    pub values: u32,
    /// The parameters of the definitions section.
    pub definitions: u32,
    /// The parameters of the translations section.
    pub translations: u32,
    /// The parameters only in the new version.
    pub added: u32,
    /// The parameters only in the old version.
    pub removed: u32,
    /// The rows of the limits section.
    pub limit_rows: u32,
    /// The rows of the definitions section.
    pub definition_rows: u32,
    /// The rows of the translations section.
    pub translation_rows: u32,
    /// The parameters on both sides without a change.
    pub unchanged: u32,
}

impl DiffSummary {
    /// Return the number of parameters of the section `kind`.
    #[must_use]
    pub const fn parameters(&self, kind: DiffKind) -> u32 {
        match kind {
            DiffKind::Limits => self.limits,
            DiffKind::Values => self.values,
            DiffKind::Definitions => self.definitions,
            DiffKind::Translations => self.translations,
            DiffKind::Added => self.added,
            DiffKind::Removed => self.removed,
        }
    }
}

/// The rows of one section on one page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSection {
    /// The section.
    pub kind: DiffKind,
    /// The rows of the parameters that the page shows in this section, in the order of the table.
    pub rows: Vec<DiffRow>,
}

/// One page of the diff of two versions (ADR-0050 (d), (e)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffPage {
    /// The counts of every key of both versions, not only of this page.
    pub summary: DiffSummary,
    /// The sections that hold an entry of this page, in the order of [`DiffKind::ALL`].
    pub sections: Vec<DiffSection>,
    /// The path of each unit of a parameter of the page, also of an archived unit.
    pub unit_paths: BTreeMap<UnitId, UnitPath>,
    /// The type text of each type of a parameter of the page, for example `ST_Zone@2`.
    pub type_texts: BTreeMap<PlcTypeId, String>,
    /// The section of the request, or `None`.
    pub kind: Option<DiffKind>,
    /// The page number. The first page is page 0.
    pub page: u32,
    /// Whether a page after this page holds an entry.
    pub has_next: bool,
    /// The sections of the request with an entry outside this page, in the order of [`DiffKind::ALL`].
    ///
    /// A section without an entry on this page counts too. The web layer links each section to
    /// its own pages with `kind` (ADR-0050 (e), design D4). With `kind`, only that section can
    /// show here.
    pub cut: Vec<DiffKind>,
}

/// The entries of one page, picked from the change flags.
#[derive(Debug)]
pub(super) struct PagePlan {
    /// The keys of each section of the page, in the order of the table.
    sections: Vec<(DiffKind, Vec<ParameterKey>)>,
    /// The distinct keys of the page, in the order of the table.
    keys: Vec<ParameterKey>,
    choice: DiffChoice,
    has_next: bool,
    cut: Vec<DiffKind>,
}

impl PagePlan {
    /// Return the distinct keys of the page, in the order of the table.
    pub(super) fn keys(&self) -> &[ParameterKey] {
        &self.keys
    }
}

/// What a page reads in its transaction: the plan, the summary, and the pairs of the plan.
#[derive(Debug)]
pub(super) struct PageRead {
    pub(super) plan: PagePlan,
    pub(super) summary: DiffSummary,
    pub(super) sides: DiffSides,
}

/// Return `true` when the flags put the key into the section `kind`.
const fn is_in(kind: DiffKind, flags: &ChangeFlags) -> bool {
    match kind {
        DiffKind::Limits => flags.limits,
        DiffKind::Values => flags.value,
        DiffKind::Definitions => flags.definition,
        DiffKind::Translations => flags.translations,
        DiffKind::Added => flags.only_new,
        DiffKind::Removed => flags.only_old,
    }
}

/// Convert a count to `u32`. A version holds at most 4 000 parameters, so a failure is a defect.
fn count_of(count: usize) -> Result<u32, AppError> {
    u32::try_from(count).map_err(|_| StorageError::invariant("a diff count is out of range").into())
}

/// Add the field `rows` of every key. A failure is a defect, as in [`count_of`].
fn rows_of(flags: &[ChangeFlags], rows: impl Fn(&ChangeFlags) -> u32) -> Result<u32, AppError> {
    flags
        .iter()
        .try_fold(0_u32, |sum, row| sum.checked_add(rows(row)))
        .ok_or_else(|| StorageError::invariant("a diff row count has no sum").into())
}

/// Count the summary of a diff from the flags of every key (design D4).
fn summary_of(flags: &[ChangeFlags]) -> Result<DiffSummary, AppError> {
    let section = |kind| count_of(flags.iter().filter(|row| is_in(kind, row)).count());
    Ok(DiffSummary {
        limits: section(DiffKind::Limits)?,
        values: section(DiffKind::Values)?,
        definitions: section(DiffKind::Definitions)?,
        translations: section(DiffKind::Translations)?,
        added: section(DiffKind::Added)?,
        removed: section(DiffKind::Removed)?,
        limit_rows: rows_of(flags, |row| row.limit_rows)?,
        definition_rows: rows_of(flags, |row| row.definition_rows)?,
        translation_rows: rows_of(flags, |row| row.translation_rows)?,
        unchanged: count_of(flags.iter().filter(|row| row.is_unchanged()).count())?,
    })
}

/// One key in one section.
type Entry = (DiffKind, ParameterKey);

/// Return the entries of the sections `kinds`, in the order of the sections and then of the table.
fn entries_of(flags: &[ChangeFlags], kinds: &[DiffKind]) -> Vec<Entry> {
    kinds
        .iter()
        .flat_map(|kind| {
            let kind = *kind;
            flags
                .iter()
                .filter(move |row| is_in(kind, row))
                .map(move |row| (kind, row.key))
        })
        .collect()
}

/// Return the keys of the section `kind` among `entries`, in their order.
fn keys_in(entries: &[Entry], kind: DiffKind) -> Vec<ParameterKey> {
    entries
        .iter()
        .filter(|(found, _)| *found == kind)
        .map(|(_, key)| *key)
        .collect()
}

/// Pick the entries of the page `choice` from the flags (design D4).
///
/// The entries come in the order of the sections, then in the order of the table. A page after
/// the last page gives no entry.
fn plan_page(flags: &[ChangeFlags], choice: DiffChoice, size: u32) -> Result<PagePlan, AppError> {
    let kinds: Vec<DiffKind> = choice
        .kind
        .map_or_else(|| DiffKind::ALL.to_vec(), |k| vec![k]);
    let entries = entries_of(flags, &kinds);
    let (offset, limit) = page_window(choice.page, size)?;
    let window = entries
        .iter()
        .skip(usize::try_from(offset).unwrap_or(usize::MAX))
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .copied()
        .collect();
    let (shown, has_next) = cut_page(window, size);
    let sections: Vec<(DiffKind, Vec<ParameterKey>)> = kinds
        .iter()
        .map(|kind| (*kind, keys_in(&shown, *kind)))
        .filter(|(_, keys)| !keys.is_empty())
        .collect();
    let count = |list: &[Entry], kind: DiffKind| list.iter().filter(|(k, _)| *k == kind).count();
    let cut = kinds
        .iter()
        .copied()
        .filter(|kind| count(&shown, *kind) < count(&entries, *kind))
        .collect();
    let picked: BTreeSet<ParameterKey> = shown.iter().map(|(_, key)| *key).collect();
    let keys = flags
        .iter()
        .map(|row| row.key)
        .filter(|key| picked.contains(key))
        .collect();
    Ok(PagePlan {
        sections,
        keys,
        choice,
        has_next,
        cut,
    })
}

/// Read the flags, plan the page, and read the pairs of its keys, in one snapshot of `reads`.
///
/// `old` is `None` for a first approval. With `read_new` set to `false`, the new side reads
/// nothing, because the caller holds the loaded draft (design D3).
///
/// # Errors
///
/// Returns `AppError::Storage` when the database fails.
pub(super) async fn read_page(
    reads: &mut dyn DiffReads,
    (old, new): (Option<RecipeVersionId>, RecipeVersionId),
    read_new: bool,
    (choice, size): (DiffChoice, u32),
) -> Result<PageRead, AppError> {
    let flags = reads.change_flags(old, new).await?;
    let summary = summary_of(&flags)?;
    let plan = plan_page(&flags, choice, size)?;
    let new_side = read_new.then_some(new);
    let sides = reads.pairs(old, new_side, &plan.keys).await?;
    Ok(PageRead {
        plan,
        summary,
        sides,
    })
}

/// Require a resolved tree for every type of both sides (design D4).
///
/// The domain diff shows the stored JSON of a value without a tree. The use case gives an error
/// instead, because the type rows of a version always resolve unless the storage is broken.
fn require_trees(ids: &BTreeSet<PlcTypeId>, trees: &TypeTrees) -> Result<(), AppError> {
    for id in ids {
        match trees.get(*id) {
            Some(Ok(_)) => {}
            Some(Err(error)) => return Err(error.clone().into()),
            None => return Err(StorageError::invariant("a diff type was not resolved").into()),
        }
    }
    Ok(())
}

/// Return the path of each unit in `units`, live or archived.
fn unit_paths_of(
    machine: &Machine,
    units: BTreeSet<UnitId>,
) -> Result<BTreeMap<UnitId, UnitPath>, AppError> {
    units
        .into_iter()
        .map(|unit| {
            let found = machine.units().find(|candidate| candidate.id() == unit);
            let path = found
                .map(|candidate| candidate.path().clone())
                .ok_or_else(|| {
                    StorageError::invariant("the unit of a parameter is not a unit of its machine")
                })?;
            Ok((unit, path))
        })
        .collect()
}

/// Run the domain diff on the pairs of the plan and keep the rows of its entries (ADR-0050 (e)).
///
/// `trees` holds the resolved tree of each type of both sides, and it can hold more. The caller
/// runs the function after the transaction ends, on a blocking thread.
///
/// # Errors
///
/// Returns `AppError::Plant` when a type of a pair does not resolve. Returns `AppError::Storage`
/// when a unit, a tree, or a type text is missing.
pub(super) fn build_page(
    plan: PagePlan,
    summary: DiffSummary,
    (old, new): (&[Parameter], &[Parameter]),
    (machine, types, trees): (&Machine, &PlatformTypes, &TypeTrees),
    leaf_rows: u32,
) -> Result<DiffPage, AppError> {
    let both = || old.iter().chain(new.iter());
    let ids: BTreeSet<PlcTypeId> = both().map(Parameter::plc_type).collect();
    require_trees(&ids, trees)?;
    let pairs = DiffPair::match_by_key(&plan.keys, old, new);
    let rows = diff_pairs(&pairs, trees, leaf_rows);
    let sections = plan
        .sections
        .into_iter()
        .map(|(kind, keys)| DiffSection {
            kind,
            rows: rows
                .section(kind)
                .iter()
                .filter(|row| keys.contains(&row.key()))
                .cloned()
                .collect(),
        })
        .collect();
    let type_texts = ids
        .iter()
        .map(|id| Ok((*id, text_of(types, *id)?)))
        .collect::<Result<_, AppError>>()?;
    Ok(DiffPage {
        summary,
        sections,
        unit_paths: unit_paths_of(machine, both().map(Parameter::unit).collect())?,
        type_texts,
        kind: plan.choice.kind,
        page: plan.choice.page,
        has_next: plan.has_next,
        cut: plan.cut,
    })
}
