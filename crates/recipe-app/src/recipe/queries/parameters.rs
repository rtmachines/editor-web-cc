//! The queries `ListParameters`, `GetParameter`, `GetLimits`, and `GetTranslations` (design D4).
//!
//! The specs are "Permission first in each recipe query", "Read of a deleted recipe", and "One
//! read state for a draft query" (ADR-0042, ADR-0043 (e) and (j)). Each query reads in one
//! `RepeatableRead` transaction and returns the `row_version` of the version with its data.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use recipe_domain::plant::{
    Machine, MachineCode, PlatformTypes, PlcTypeId, TypeNode, TypeTree, UnitId, UnitPath,
};
use recipe_domain::recipe::{
    Parameter, ParameterId, ParameterKey, ParameterTag, ParameterValue, Recipe, RecipeVersionId,
    ReferenceNumber, RuleFault, StoredLimit, StoredParameter, StoredValue, Translation, ValueFault,
    VersionState, check_stored_parameter,
};
use recipe_domain::text::{LongText, Text};

use super::{cut_page, page_window, read_machine, read_recipe, require_visible};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing, StorageError};
use crate::plant::type_text;
use crate::ports::{Isolation, ParameterRow, Transaction, UnitCount, VersionDetail};

/// The recipe, its machine, and the header of one version, as a parameter query reads them.
///
/// The type solves one problem: every parameter query starts with the same three reads and the
/// same two checks. One struct carries their result to the rest of the query.
pub(super) struct VersionScope {
    /// The recipe with the headers of all its versions. It can be deleted.
    pub(super) recipe: Recipe,
    /// The machine of the recipe. It gives the platform and the units.
    pub(super) machine: Machine,
    /// The header of the version with its `row_version`.
    pub(super) detail: VersionDetail,
}

/// Read the recipe, run the deleted-recipe check `visible`, and read the machine and the version.
///
/// # Errors
///
/// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
/// Returns the error of `visible` for a deleted recipe. Returns `AppError::Storage` on a failure.
pub(super) async fn read_scope(
    tx: &mut dyn Transaction,
    (reference, number): (&ReferenceNumber, u32),
    visible: impl FnOnce(&Recipe) -> Result<(), AppError>,
) -> Result<VersionScope, AppError> {
    let recipe = read_recipe(tx, reference).await?;
    visible(&recipe)?;
    let not_found = || AppError::NotFound {
        entity: Missing::version(reference, number),
    };
    let version = recipe.version(number).ok_or_else(not_found)?.id();
    let machine = read_machine(tx.plant(), &recipe).await?;
    let detail = tx
        .recipe_reads()
        .version_detail(version)
        .await?
        .ok_or_else(not_found)?;
    Ok(VersionScope {
        recipe,
        machine,
        detail,
    })
}

/// The parsed request of `ListParameters`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListParametersInput {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The number of the version.
    pub number: u32,
    /// The unit whose parameters the page shows, or `None` for the default of ADR-0043 (j).
    pub unit: Option<UnitPath>,
    /// The page number. The first page is page 0.
    pub page: u32,
}

/// The result of `ListParameters`.
///
/// The type derives no `Clone` and no `PartialEq`, because `RuleFault` has none.
#[derive(Debug)]
pub struct ParameterList {
    /// The recipe with the headers of all its versions. It can be deleted.
    pub recipe: Recipe,
    /// The machine of the recipe. It gives the PLC platform of the version.
    pub machine: Machine,
    /// The header of the version with its `row_version` (ADR-0042).
    pub detail: VersionDetail,
    /// The count of the parameters of each unit that holds one, in the numeric order of the path.
    pub units: Vec<UnitCount>,
    /// The unit whose parameters the rows hold, or `None` when the rows cover every unit.
    pub unit: Option<UnitPath>,
    /// The rows of the page in the order unit path, ordinal, and tag. At most `page.parameters` rows.
    pub rows: Vec<ParameterRow>,
    /// The type text of each type that a row of the page uses, for example `ARRAY[1..4] OF INT`.
    pub type_texts: BTreeMap<PlcTypeId, String>,
    /// The leaf text of the value of each primitive parameter of the page, as the leaf grid
    /// writes it (A43). An array parameter and a struct parameter have no entry.
    pub value_texts: BTreeMap<ParameterKey, String>,
    /// The rule faults of the stored parts of each parameter of the page, whatever its type
    /// (ADR-0118). A parameter without a fault has no entry.
    pub rule_faults: BTreeMap<ParameterKey, Vec<RuleFault>>,
    /// The page number. The first page is page 0.
    pub page: u32,
    /// Whether a page after this page holds a row.
    pub has_next: bool,
}

/// Choose the unit whose rows the page shows (ADR-0043 (j), `api.md` section 2.6).
///
/// A wanted unit must hold a parameter of the version. Without a wanted unit, a live draft with
/// more than `size` parameters shows its first unit. A smaller draft and a version in another
/// state show every unit.
///
/// # Errors
///
/// Returns `NotFound` (`E-WEB-005`) when the version holds no parameter in the wanted unit.
fn shown_unit<'a>(
    units: &'a [UnitCount],
    wanted: Option<&UnitPath>,
    (state, size): (VersionState, u32),
    code: MachineCode,
) -> Result<Option<&'a UnitCount>, AppError> {
    if let Some(path) = wanted {
        let found = units.iter().find(|count| count.path == *path);
        return found.map(Some).ok_or_else(|| AppError::NotFound {
            entity: Missing::Unit {
                code,
                path: path.clone(),
            },
        });
    }
    if state != VersionState::LiveDraft {
        return Ok(None);
    }
    let total = units
        .iter()
        .try_fold(0_u64, |sum, count| sum.checked_add(u64::from(count.count)))
        .ok_or_else(|| StorageError::invariant("the parameter counts have no sum"))?;
    Ok((total > u64::from(size)).then(|| units.first()).flatten())
}

/// Write the type text of the row `id` with `plant/queries/type_text.rs`.
///
/// A parameter type is a global row or a row of the machine of the recipe. So a missing text is
/// a defect of the storage.
pub(super) fn text_of(types: &PlatformTypes, id: PlcTypeId) -> Result<String, AppError> {
    type_text(types, id)
        .ok_or_else(|| StorageError::invariant("a parameter type has no type text").into())
}

/// Write the type text of each type that `rows` use, over the type rows of `types`.
fn texts_of(
    types: &PlatformTypes,
    rows: &[ParameterRow],
) -> Result<BTreeMap<PlcTypeId, String>, AppError> {
    let used: BTreeSet<PlcTypeId> = rows.iter().map(|row| row.plc_type).collect();
    used.into_iter()
        .map(|id| Ok((id, text_of(types, id)?)))
        .collect()
}

/// Write the leaf text of a stored value, as the leaf grid writes it (A43, ADR-0039).
///
/// The function solves one problem: the parameter table and the answer of `AddParameter` show
/// the value of a primitive parameter, and they must show the text of the leaf grid. It uses the
/// domain check `ParameterValue::try_from` and the domain writer `ParameterValue::texts`, so no
/// second formatter exists. A `TIME` value shows its IEC literal, and a `REAL` value shows its
/// canonical decimal text.
///
/// Returns `Some(text)` when the root of `tree` is a primitive leaf. Returns `None` for an array
/// and for a struct, because they have more than one leaf.
///
/// # Errors
///
/// Returns the `ValueFault` of a stored value that does not match `tree`, a defect of the storage.
pub(crate) fn value_text(
    tree: &TypeTree,
    value: &StoredValue,
) -> Result<Option<String>, ValueFault> {
    if !matches!(tree.root(), TypeNode::Leaf { .. }) {
        return Ok(None);
    }
    leaf_text(tree, value).map(Some)
}

/// Write the leaf text of a stored value whose `tree` has a primitive leaf as its root.
///
/// The caller guarantees the leaf root. A tree with more than one leaf gives `WrongTree`.
fn leaf_text(tree: &TypeTree, value: &StoredValue) -> Result<String, ValueFault> {
    let checked = ParameterValue::try_from((value, tree))?;
    let mut leaves = checked.texts(tree)?.into_iter();
    match (leaves.next(), leaves.next()) {
        (Some(leaf), None) => Ok(leaf.text),
        _ => Err(ValueFault::WrongTree),
    }
}

/// Resolve the tree of each type that `rows` use.
///
/// Every row needs its tree, because the read check also runs the address rules (ADR-0118). So an
/// array, a struct, and a primitive type each get an entry.
fn row_trees(
    types: &PlatformTypes,
    rows: &[ParameterRow],
) -> Result<BTreeMap<PlcTypeId, TypeTree>, AppError> {
    let used: BTreeSet<PlcTypeId> = rows.iter().map(|row| row.plc_type).collect();
    used.into_iter()
        .map(|id| Ok((id, TypeTree::resolve(id, types)?)))
        .collect()
}

/// The value texts and the rule faults of the rows of one page.
#[derive(Debug, Default)]
struct ValueReads {
    /// The leaf text of the stored value of each primitive row. A composite row has no entry.
    texts: BTreeMap<ParameterKey, String>,
    /// The rule faults of each row that has one. A row without a fault has no entry.
    faults: BTreeMap<ParameterKey, Vec<RuleFault>>,
}

/// Build the parameter that the read check needs from a row of the page and its stored value.
///
/// The page reads no limit row, no translation, and no description. So the parameter holds none
/// of them, and the read check reports no limit fault.
///
/// # Errors
///
/// Returns `StorageError` (`E-DB-008`) when the name of the row is not a valid description.
fn checked_parameter(row: &ParameterRow, value: &StoredValue) -> Result<Parameter, StorageError> {
    let description = LongText::parse(row.name_en.as_str())
        .map_err(|_| StorageError::invariant("the name of a parameter is not a description"))?;
    Ok(Parameter::from(StoredParameter {
        id: ParameterId::from(*row.key.as_uuid()),
        key: row.key,
        unit: row.unit,
        tag: row.tag.clone(),
        name_en: row.name_en.clone(),
        description_en: description,
        translations: Vec::new(),
        plc_type: row.plc_type,
        s7_address: row.s7_address,
        limits: Vec::new(),
        value: value.clone(),
        ordinal: row.ordinal,
    }))
}

/// Write the value text and find the rule faults of each row of the page.
///
/// The text comes from `value_text` and exists for a primitive row only. The faults come from the
/// read check `check_stored_parameter` (ADR-0118), so no second rule exists here. The check runs
/// for every row, because the address rules apply to an array and to a struct too.
///
/// # Errors
///
/// Returns `StorageError` (`E-DB-008`) when a row has no value or no tree, when a value does not
/// match its tree, or when a part breaks the earlier rule set. All three cases are defects of
/// the storage. The second and third case name the parameter key.
fn value_reads_of(
    rows: &[ParameterRow],
    trees: &BTreeMap<PlcTypeId, TypeTree>,
    values: &BTreeMap<ParameterKey, StoredValue>,
) -> Result<ValueReads, AppError> {
    let mut reads = ValueReads::default();
    for row in rows {
        let tree = trees
            .get(&row.plc_type)
            .ok_or_else(|| StorageError::invariant("a parameter type has no type tree"))?;
        let value = values
            .get(&row.key)
            .ok_or_else(|| StorageError::invariant("a parameter has no stored value"))?;
        let text =
            value_text(tree, value).map_err(|fault| StorageError::broken_value(row.key, fault))?;
        let parameter = checked_parameter(row, value)?;
        let faults = check_stored_parameter(&parameter, tree)
            .map_err(|fault| StorageError::broken_stored_parameter(row.key, fault))?;
        if let Some(text) = text {
            reads.texts.insert(row.key, text);
        }
        if !faults.is_empty() {
            reads.faults.insert(row.key, faults);
        }
    }
    Ok(reads)
}

/// Read the stored values of the rows of the page in the open transaction.
///
/// A composite row is included, because the address rules need its value and its tree.
async fn read_row_values(
    tx: &mut dyn Transaction,
    version: RecipeVersionId,
    rows: &[ParameterRow],
) -> Result<BTreeMap<ParameterKey, StoredValue>, AppError> {
    let keys: Vec<ParameterKey> = rows.iter().map(|row| row.key).collect();
    Ok(tx.recipe_reads().parameter_values(version, &keys).await?)
}

/// Lists the parameters of one version, one page at a time.
///
/// The query solves one problem: a version can hold 4 000 parameters. The table must show at
/// most `page.parameters` rows from a fixed number of statements. The query reads the counts of
/// the units, one page of rows, and the type rows in one `RepeatableRead` transaction. It also
/// reads the values of the rows of the page in that transaction (ADR-0108 (b), ADR-0118). So the
/// `row_version`, the type texts, and the values of the result belong to the rows (ADR-0042).
/// After the transaction ends, the value texts and the read check run in `Blocking::validate`,
/// because one page can hold `page.parameters` values of `budget.value_bytes` each (ADR-0121 (f)).
#[derive(Clone)]
pub struct ListParameters {
    ctx: AppContext,
}

impl UseCase for ListParameters {
    const OPERATION: Operation = Operation::ListParameters;
}

impl ListParameters {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return one page of the parameters with the counts of the units.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe, the version, or the unit does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted and the role cannot restore it.
    /// Returns `AppError::Storage` when the database fails.
    /// Returns `AppError::Timeout` (`E-WEB-009`) with the stage `Validation` when the value check
    /// passes `timeout.validation`.
    /// Returns `AppError::TaskFailed` (`E-WEB-011`) with the stage `Validation` when it fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ListParametersInput,
    ) -> Result<ParameterList, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let mut tx = self.ctx.uow.begin(Isolation::RepeatableRead).await?;
        let target = (&input.reference, input.number);
        let scope = read_scope(tx.as_mut(), target, |recipe| require_visible(user, recipe)).await?;
        let units = tx.recipe_reads().unit_counts(scope.detail.id).await?;
        let size = self.ctx.settings.pages.parameters;
        let page_rule = (scope.detail.state, size);
        let shown =
            shown_unit(&units, input.unit.as_ref(), page_rule, scope.machine.code())?.cloned();
        let (offset, limit) = page_window(input.page, size)?;
        let filter = shown.as_ref().map(|count| count.unit);
        let rows = tx
            .recipe_reads()
            .parameter_page(scope.detail.id, filter, offset, limit)
            .await?;
        let type_rows = tx.types().type_rows(scope.machine.id()).await?;
        let (rows, has_next) = cut_page(rows, size);
        let machine = &scope.machine;
        let types = PlatformTypes::collect(machine.id(), machine.platform(), type_rows)?;
        let trees = row_trees(&types, &rows)?;
        let values = read_row_values(tx.as_mut(), scope.detail.id, &rows).await?;
        drop(tx);
        let type_texts = texts_of(&types, &rows)?;
        let (rows, reads) = self
            .ctx
            .settings
            .blocking
            .validate(move || {
                let reads = value_reads_of(&rows, &trees, &values);
                (rows, reads)
            })
            .await?;
        let reads = reads?;
        Ok(ParameterList {
            recipe: scope.recipe,
            machine: scope.machine,
            detail: scope.detail,
            units,
            unit: shown.map(|count| count.path),
            rows,
            type_texts,
            value_texts: reads.texts,
            rule_faults: reads.faults,
            page: input.page,
            has_next,
        })
    }
}

/// One parameter of one version, as a URL names it.
///
/// The type solves one problem: `GetParameter`, `GetLimits`, and `GetTranslations` read the same
/// three values from a URL, and one struct keeps them together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterRef {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The number of the version.
    pub number: u32,
    /// The key of the parameter.
    pub key: ParameterKey,
}

/// The version scope with the parameter of one key.
struct ParameterScope {
    scope: VersionScope,
    parameter: StoredParameter,
}

/// Read the version scope with the deleted-recipe rule of ADR-0043 (e) and the parameter of `target`.
///
/// # Errors
///
/// Returns `NotFound` (`E-WEB-005`) with `Missing::Parameter` when the version does not hold the key.
async fn read_parameter(
    tx: &mut dyn Transaction,
    user: &CurrentUser,
    target: &ParameterRef,
) -> Result<ParameterScope, AppError> {
    let scope = read_scope(tx, (&target.reference, target.number), |recipe| {
        require_visible(user, recipe)
    })
    .await?;
    let parameter = tx
        .recipe_reads()
        .parameter_by_key(scope.detail.id, target.key)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::Parameter {
                reference: target.reference.clone(),
                number: target.number,
                key: target.key,
            },
        })?;
    Ok(ParameterScope { scope, parameter })
}

/// Return the path of the unit `unit` of `machine`.
///
/// A parameter always names a unit of the machine of its recipe (I15), so a missing unit is a
/// defect of the storage.
fn unit_path_of(machine: &Machine, unit: UnitId) -> Result<UnitPath, AppError> {
    machine
        .units()
        .find(|found| found.id() == unit)
        .map(|found| found.path().clone())
        .ok_or_else(|| {
            StorageError::invariant("the unit of a parameter is not a unit of its machine").into()
        })
}

/// The result of `GetParameter`.
///
/// The type derives no `PartialEq`, because `StoredParameter` has none.
#[derive(Clone, Debug)]
pub struct ParameterView {
    /// The recipe with the headers of all its versions. It can be deleted.
    pub recipe: Recipe,
    /// The machine of the recipe. It gives the PLC platform of the version.
    pub machine: Machine,
    /// The header of the version with its `row_version` (ADR-0042).
    pub detail: VersionDetail,
    /// The path of the unit that holds the parameter.
    pub unit_path: UnitPath,
    /// The parameter with its value, its limits, and its translations.
    pub parameter: StoredParameter,
    /// The resolved tree of the type of the parameter.
    pub tree: TypeTree,
    /// The type text of the parameter type, for example `ST_Zone@2` or `STRING(20)`.
    pub type_text: String,
    /// The rule faults of the stored parameter, each with its field (ADR-0118, design D7).
    ///
    /// The list is empty for a parameter that meets the current rules.
    pub rule_faults: Arc<[RuleFault]>,
}

/// Shows one parameter with its value, its limits, its translations, and its type tree.
///
/// The query solves one problem: the parameter page shows the leaf grid of one parameter. The
/// grid needs the tree, the value, and the limits of one moment. The query reads them in one
/// `RepeatableRead` transaction and resolves the tree after the transaction ends.
#[derive(Clone)]
pub struct GetParameter {
    ctx: AppContext,
}

impl UseCase for GetParameter {
    const OPERATION: Operation = Operation::GetParameter;
}

impl GetParameter {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the parameter, its tree, its unit path, and the machine.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe, the version, or the key does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted and the role cannot restore it.
    /// Returns `AppError::Plant` when the type rows break a rule of the domain.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ParameterRef,
    ) -> Result<ParameterView, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let mut tx = self.ctx.uow.begin(Isolation::RepeatableRead).await?;
        let found = read_parameter(tx.as_mut(), user, &input).await?;
        let machine = found.scope.machine;
        let rows = tx.types().type_rows(machine.id()).await?;
        drop(tx);
        let types = PlatformTypes::collect(machine.id(), machine.platform(), rows)?;
        let tree = TypeTree::resolve(found.parameter.plc_type, &types)?;
        let type_text = text_of(&types, found.parameter.plc_type)?;
        let unit_path = unit_path_of(&machine, found.parameter.unit)?;
        let key = found.parameter.key;
        let rule_faults = check_stored_parameter(&Parameter::from(found.parameter.clone()), &tree)
            .map_err(|fault| StorageError::broken_stored_parameter(key, fault))?;
        Ok(ParameterView {
            recipe: found.scope.recipe,
            machine,
            detail: found.scope.detail,
            unit_path,
            parameter: found.parameter,
            tree,
            type_text,
            rule_faults: rule_faults.into(),
        })
    }
}

/// The result of `GetLimits`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitsView {
    /// The header of the version with its `row_version` (ADR-0042).
    pub detail: VersionDetail,
    /// The key of the parameter.
    pub key: ParameterKey,
    /// The tag of the parameter.
    pub tag: ParameterTag,
    /// The limit rows of the parameter in their stored order.
    pub limits: Vec<StoredLimit>,
    /// The resolved tree of the type of the parameter. The limits grid names each row by it.
    pub tree: TypeTree,
}

/// Shows the limit rows of one parameter with the type tree of the parameter.
///
/// The query solves one problem: the limits fragment of `api.md` section 2.6 shows the rows of
/// one parameter without its value and its translations. The grid needs the tree to name the
/// leaf of each row (ADR-0046 (d)).
#[derive(Clone)]
pub struct GetLimits {
    ctx: AppContext,
}

impl UseCase for GetLimits {
    const OPERATION: Operation = Operation::GetLimits;
}

impl GetLimits {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the limit rows and the type tree of the parameter with the `row_version` of the version.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe, the version, or the key does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted and the role cannot restore it.
    /// Returns `AppError::Plant` when the type rows break a rule of the domain.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ParameterRef,
    ) -> Result<LimitsView, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let mut tx = self.ctx.uow.begin(Isolation::RepeatableRead).await?;
        let found = read_parameter(tx.as_mut(), user, &input).await?;
        let machine = found.scope.machine;
        let rows = tx.types().type_rows(machine.id()).await?;
        drop(tx);
        let types = PlatformTypes::collect(machine.id(), machine.platform(), rows)?;
        let tree = TypeTree::resolve(found.parameter.plc_type, &types)?;
        Ok(LimitsView {
            detail: found.scope.detail,
            key: found.parameter.key,
            tag: found.parameter.tag,
            limits: found.parameter.limits,
            tree,
        })
    }
}

/// The result of `GetTranslations`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslationsView {
    /// The header of the version with its `row_version` (ADR-0042).
    pub detail: VersionDetail,
    /// The key of the parameter.
    pub key: ParameterKey,
    /// The tag of the parameter.
    pub tag: ParameterTag,
    /// The English name, which is the base text of each row (ADR-0043 (f)).
    pub name_en: Text,
    /// The English description, which is the base text of each row.
    pub description_en: LongText,
    /// The translation rows of the parameter in their stored order.
    pub translations: Vec<Translation>,
}

/// Shows the translation rows of one parameter.
///
/// The query solves one problem: the translation fragment shows one row per language next to
/// the English base text, without the value and the limits of the parameter.
#[derive(Clone)]
pub struct GetTranslations {
    ctx: AppContext,
}

impl UseCase for GetTranslations {
    const OPERATION: Operation = Operation::GetTranslations;
}

impl GetTranslations {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the translation rows with the English base text and the `row_version`.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe, the version, or the key does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted and the role cannot restore it.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ParameterRef,
    ) -> Result<TranslationsView, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let mut tx = self.ctx.uow.begin(Isolation::RepeatableRead).await?;
        let found = read_parameter(tx.as_mut(), user, &input).await?;
        drop(tx);
        Ok(TranslationsView {
            detail: found.scope.detail,
            key: found.parameter.key,
            tag: found.parameter.tag,
            name_en: found.parameter.name_en,
            description_en: found.parameter.description_en,
            translations: found.parameter.translations,
        })
    }
}
