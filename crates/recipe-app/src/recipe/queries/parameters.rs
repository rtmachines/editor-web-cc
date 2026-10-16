//! The queries `ListParameters`, `GetParameter`, `GetLimits`, and `GetTranslations` (design D4).
//!
//! The specs are "Permission first in each recipe query", "Read of a deleted recipe", and "One
//! read state for a draft query" (ADR-0042, ADR-0043 (e) and (j)). Each query reads in one
//! `RepeatableRead` transaction and returns the `row_version` of the version with its data.

use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::plant::{
    Machine, MachineCode, PlatformTypes, PlcType, PlcTypeId, TypeTree, UnitId, UnitPath,
};
use recipe_domain::recipe::{
    ParameterKey, ParameterTag, Recipe, ReferenceNumber, StoredLimit, StoredParameter, Translation,
    VersionState,
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
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Write the type text of each type that `rows` use, over the type rows of `machine`.
fn texts_of(
    machine: &Machine,
    type_rows: Vec<PlcType>,
    rows: &[ParameterRow],
) -> Result<BTreeMap<PlcTypeId, String>, AppError> {
    let types = PlatformTypes::collect(machine.id(), machine.platform(), type_rows)?;
    let used: BTreeSet<PlcTypeId> = rows.iter().map(|row| row.plc_type).collect();
    used.into_iter()
        .map(|id| Ok((id, text_of(&types, id)?)))
        .collect()
}

/// Lists the parameters of one version, one page at a time.
///
/// The query solves one problem: a version can hold 4 000 parameters. The table must show at
/// most `page.parameters` rows from a fixed number of statements. The query reads the counts of
/// the units, one page of rows, and the type rows in one `RepeatableRead` transaction. So the
/// `row_version` and the type texts of the result belong to the rows (ADR-0042).
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
        drop(tx);
        let (rows, has_next) = cut_page(rows, size);
        let type_texts = texts_of(&scope.machine, type_rows, &rows)?;
        Ok(ParameterList {
            recipe: scope.recipe,
            machine: scope.machine,
            detail: scope.detail,
            units,
            unit: shown.map(|count| count.path),
            rows,
            type_texts,
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
        Ok(ParameterView {
            recipe: found.scope.recipe,
            machine,
            detail: found.scope.detail,
            unit_path,
            parameter: found.parameter,
            tree,
            type_text,
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
