//! The query `GetParameterForm` (design D4, ADR-0039 (h), ADR-0043 (e)).
//!
//! The spec is "Permission first in each recipe query". The form of a new parameter and the edit
//! form of a parameter read through this query. With a `TypeExpr`, it returns the preview tree of
//! `preview.rs` and writes nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use recipe_domain::plant::{
    Machine, PlatformTypes, PlcType, Primitive, RecordShape, StringKind, StringLength, TypeExpr,
    TypeName, TypeTree, Unit, UnitPath,
};
use recipe_domain::recipe::{
    DraftError, Parameter, ParameterKey, Recipe, RecipeError, ReferenceNumber, RuleFault,
    StoredDefect, StoredParameter, VersionState, check_stored_parameter,
    is_stored_resolution_defect,
};

use super::parameters::{VersionScope, read_scope, text_of};
use super::preview::preview_tree;
use crate::auth::{
    CurrentUser, FormPurpose, Operation, ParameterFormPurpose, RequestMeta, authorize,
};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing, StorageError};
use crate::plant::{check_iec_struct, expr_names_row, is_refused_struct, require_current_type};
use crate::ports::{Isolation, VersionDetail};
use crate::recipe::draft_session::collect_stored_types;

/// The parsed request of `GetParameterForm`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetParameterFormInput {
    /// The command that the form or the route serves. The query checks its permission
    /// (ADR-0046 (b)).
    pub purpose: ParameterFormPurpose,
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The number of the version.
    pub number: u32,
    /// The key of the parameter that the edit form shows, or `None` for a new parameter.
    pub key: Option<ParameterKey>,
    /// The type whose leaf rows the form shows, or `None` for a form without value fields.
    pub type_expr: Option<TypeExpr>,
    /// The unit that the form selects at first, or `None` for no selection.
    pub unit: Option<UnitPath>,
}

/// The data of the form of a parameter.
///
/// The type derives no `PartialEq`, because `StoredParameter` has none.
#[derive(Clone, Debug)]
pub struct ParameterForm {
    /// The recipe with the headers of all its versions.
    pub recipe: Recipe,
    /// The machine of the recipe. It gives the PLC platform of the version.
    pub machine: Machine,
    /// The header of the version with its `row_version` (ADR-0042).
    pub detail: VersionDetail,
    /// The live units of the machine in the order of the path.
    pub units: Vec<Unit>,
    /// The unit that the form selects at first. It is a live unit of the machine.
    pub unit: Option<UnitPath>,
    /// The proposals of the `datalist` of the type input, as `GetUdtForm` gives them.
    pub choices: Vec<String>,
    /// The parameter of the key, or `None` for a new parameter.
    pub parameter: Option<StoredParameter>,
    /// The type text of the parameter of the key, or `None` for a new parameter.
    pub type_text: Option<String>,
    /// The resolved tree of `type_expr`, or `None` without a type.
    pub preview: Option<TypeTree>,
    /// The rule faults of the stored parameter, each with its field (ADR-0118, design D7).
    ///
    /// The list is empty for a new parameter and for a parameter that meets the current rules.
    pub rule_faults: Arc<[RuleFault]>,
}

/// What `GetParameterForm` reads in its transaction.
struct FormReads {
    scope: VersionScope,
    rows: Vec<PlcType>,
    parameter: Option<StoredParameter>,
}

/// Refuse a deleted recipe to every role, because a deleted recipe holds no form.
fn refuse_deleted(recipe: &Recipe) -> Result<(), AppError> {
    if recipe.is_deleted() {
        return Err(RecipeError::RecipeDeleted {
            reference: recipe.reference().clone(),
        }
        .into());
    }
    Ok(())
}

/// Refuse a version that is not a live draft, because only a live draft gets the forms
/// (ADR-0043 (k)).
fn require_live_draft(reference: &ReferenceNumber, detail: &VersionDetail) -> Result<(), AppError> {
    if detail.state == VersionState::LiveDraft {
        return Ok(());
    }
    Err(RecipeError::VersionNotEditable {
        reference: reference.clone(),
        number: detail.number,
        status: detail.state.status(),
    }
    .into())
}

/// Require a live unit of `machine` at `path` (`api.md` section 1, the query value `unit`).
fn require_live_unit(machine: &Machine, path: &UnitPath) -> Result<(), AppError> {
    let is_live = machine.unit(path).is_some_and(Unit::is_live);
    if is_live {
        return Ok(());
    }
    Err(AppError::NotFound {
        entity: Missing::Unit {
            code: machine.code(),
            path: path.clone(),
        },
    })
}

/// Read the version, the type rows, and the parameter of the key in one
/// `RepeatableRead` transaction.
///
/// The transaction ends when the function returns.
async fn read_form(ctx: &AppContext, input: &GetParameterFormInput) -> Result<FormReads, AppError> {
    let mut tx = ctx.uow.begin(Isolation::RepeatableRead).await?;
    let target = (&input.reference, input.number);
    let scope = read_scope(tx.as_mut(), target, refuse_deleted).await?;
    require_live_draft(&input.reference, &scope.detail)?;
    let rows = tx.types().type_rows(scope.machine.id()).await?;
    let parameter = match input.key {
        Some(key) => {
            let found = tx
                .recipe_reads()
                .parameter_by_key(scope.detail.id, key)
                .await?;
            Some(found.ok_or_else(|| AppError::NotFound {
                entity: Missing::Parameter {
                    reference: input.reference.clone(),
                    number: input.number,
                    key,
                },
            })?)
        }
        None => None,
    };
    Ok(FormReads {
        scope,
        rows,
        parameter,
    })
}

/// Check the stored `parameter` against the rules in force now and list its rule faults.
///
/// The function runs `check_stored_parameter` on the type tree of the parameter. A new parameter
/// has no faults. With `repairs` set, a stored type that does not resolve gives no faults.
/// The form then opens, so the user can repair the type (ADR-0195 (ab)).
///
/// # Errors
///
/// Returns `AppError::Storage` (`E-DB-008`) when the parameter breaks the earlier rule set.
/// It also returns it when its stored type fails to resolve through its stored rows, because
/// those are defects of the storage.
/// Returns the error of the type (`E-PLT-013`) when the platform does not hold a member type.
fn rule_faults_of(
    parameter: Option<&StoredParameter>,
    types: &PlatformTypes,
    repairs: bool,
) -> Result<Arc<[RuleFault]>, AppError> {
    let Some(stored) = parameter else {
        return Ok(Arc::from(Vec::new()));
    };
    let tree = match TypeTree::resolve(stored.plc_type, types) {
        Ok(tree) => tree,
        Err(_) if repairs => return Ok(Arc::from(Vec::new())),
        Err(error) if is_stored_resolution_defect(&error) => {
            return Err(DraftError::Stored(Box::new(StoredDefect::of_parameter(
                stored.key, error,
            )))
            .into());
        }
        Err(error) => return Err(error.into()),
    };
    let faults = check_stored_parameter(&Parameter::from(stored.clone()), &tree)
        .map_err(|fault| StorageError::broken_stored_parameter(stored.key, fault))?;
    Ok(faults.into())
}

/// Return the type text of the stored parameter, or `None` when no parameter exists.
///
/// With `repairs`, a type that does not resolve gives `None`, so the form opens and the user can
/// repair the type.
///
/// # Errors
///
/// Returns the error of `text_of` when the stored type has no type text and `repairs` is false.
fn stored_type_text(
    types: &PlatformTypes,
    parameter: Option<&StoredParameter>,
    repairs: bool,
) -> Result<Option<String>, AppError> {
    match parameter
        .map(|stored| text_of(types, stored.plc_type))
        .transpose()
    {
        Ok(text) => Ok(text),
        Err(_) if repairs => Ok(None),
        Err(error) => Err(error),
    }
}

/// Check the type text of the preview with the current rule set (ADR-0206 (d3)).
///
/// The function solves one problem: the preview of the form must show the reason that the command
/// gives, before it resolves the text. It calls [`require_current_type`] and then
/// [`check_iec_struct`] (ADR-0203 (j1), ADR-0204 (d1)). It skips both for an unchanged type text of
/// an edit form. The purpose of such a form is `UpdateParameterDefinition`, and it edits `stored`.
/// The canonical text of `expr` names the stored type, as `UpdateParameterDefinition` compares it.
///
/// # Errors
///
/// Returns `PlantError::DerivedTypeInvalid` (`E-PLT-013`) with the reason of the failed check.
fn require_form_type(
    types: &PlatformTypes,
    expr: &TypeExpr,
    (purpose, stored): (ParameterFormPurpose, Option<&StoredParameter>),
) -> Result<(), AppError> {
    let is_unchanged = purpose == ParameterFormPurpose::UpdateParameterDefinition
        && stored.is_some_and(|parameter| expr_names_row(types, parameter.plc_type, expr));
    if is_unchanged {
        return Ok(());
    }
    require_current_type(expr, types.platform())?;
    check_iec_struct(types, expr)?;
    Ok(())
}

/// Return the proposals of the type input, as `GetUdtForm` writes them (`plant/queries/types.rs`).
///
/// First come the primitives of the platform without `STRING` and `WSTRING`. Then come `STRING`,
/// and `WSTRING` when the platform has it. The latest revision of each struct follows as
/// `<name>@<revision>` in the byte order of the name. A name whose latest revision is refused is no
/// proposal, as in `GetUdtForm`. A struct with a reserved name is no proposal,
/// because every command refuses it (ADR-0143 (i2)). On an S7 platform, a struct with a Siemens
/// reserved name is no proposal either (ADR-0186 (h)).
fn proposals(types: &PlatformTypes) -> Vec<String> {
    let primitives: BTreeSet<Primitive> = types
        .rows()
        .filter_map(|row| match row.shape() {
            RecordShape::Primitive { primitive, .. }
                if !matches!(primitive, Primitive::String | Primitive::Wstring) =>
            {
                Some(*primitive)
            }
            _ => None,
        })
        .collect();
    let has_wstring = StringLength::default_length(StringKind::Wstring, types.platform()).is_some();
    let strings = [Primitive::String]
        .into_iter()
        .chain(has_wstring.then_some(Primitive::Wstring));
    let mut latest: BTreeMap<&TypeName, (u32, &PlcType)> = BTreeMap::new();
    for row in types.rows() {
        if let RecordShape::Struct { name, revision, .. } = row.shape() {
            let entry = latest.entry(name).or_insert((*revision, row));
            if *revision > entry.0 {
                *entry = (*revision, row);
            }
        }
    }
    primitives
        .into_iter()
        .chain(strings)
        .map(|primitive| primitive.name().to_owned())
        .chain(
            latest
                .iter()
                .filter(|(_, (_, row))| !is_refused_struct(types.platform(), row))
                .map(|(name, (revision, _))| format!("{name}@{revision}")),
        )
        .collect()
}

/// Builds the preview tree of the requested type text, or `None` without one.
///
/// The step solves one problem. `execute` stays short, and the preview stays a pure in-memory
/// step after the read transaction (ADR-0039 (h)).
fn preview_of(
    machine: &Machine,
    rows: Vec<PlcType>,
    type_expr: Option<&TypeExpr>,
) -> Result<Option<TypeTree>, AppError> {
    let kind = (machine.platform(), machine.string_encoding());
    type_expr
        .map(|expr| preview_tree(machine.id(), kind, rows, expr))
        .transpose()
}

/// Returns the live units of the machine in the order of the path.
fn live_units(machine: &Machine) -> Vec<Unit> {
    machine
        .units()
        .filter(|unit| unit.is_live())
        .cloned()
        .collect()
}

/// Prepares the form of a new parameter and the edit form of a parameter.
///
/// The query solves one problem. The form needs the live units, the type proposals, and the
/// parameter that it edits. It also needs the leaf rows of a type that the catalog may not
/// hold yet.
/// The query reads in one `RepeatableRead` transaction and builds the preview in memory after it.
/// So a request with `fields=1` writes nothing (ADR-0039 (h)). A deleted recipe has no form for
/// any role. Only a live draft has a form (ADR-0043 (k)).
#[derive(Clone)]
pub struct GetParameterForm {
    ctx: AppContext,
}

impl UseCase for GetParameterForm {
    const OPERATION: Operation = Operation::GetParameterForm;
}

impl GetParameterForm {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the data of the form, with the preview tree when the request names a type.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) with the command of the purpose when the role lacks its
    /// permission.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe, the version, the key, or the unit
    /// does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotEditable` (`E-RCP-002`) when the version is not a live
    /// draft.
    /// Returns `PlantError::DerivedTypeInvalid` (`E-PLT-013`) or `TypeTooLarge` (`E-PLT-018`)
    /// for the preview type. The type text of the add form and a changed type text of the edit
    /// form first meet the current rule set, so a reserved struct name gives `E-PLT-013` with the
    /// reason "reserved name" (ADR-0206 (d3)). Returns `TypeCycle` (`E-PLT-005`) when the stored rows of the
    /// preview type form a cycle.
    /// Returns `DerivedTypeInvalid` for a stored member type that the platform does not hold.
    /// Returns `AppError::Storage` when the database fails.
    /// Returns `AppError::Storage` (`E-DB-008`) when the stored parameter breaks the earlier rule
    /// set.
    /// It also returns `E-DB-008` for two type rows with one identifier.
    /// For a stored type that does not resolve, it returns `E-DB-008`. Examples are a cycle, a size
    /// above a limit, and a bad row shape.
    /// For a stored type without a type text, it returns `E-DB-008`.
    /// With the purpose `UpdateParameterDefinition`, a stored type that does not resolve gives the
    /// form without rule faults. The form has no type text when none exists. Two type rows with one
    /// identifier still give `E-DB-008`.
    /// A part that breaks only the current rule set gives a rule fault in `rule_faults`.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: GetParameterFormInput,
    ) -> Result<ParameterForm, AppError> {
        authorize(
            input.purpose.operation(),
            user,
            meta,
            self.ctx.security.as_ref(),
        )
        .await?;
        let reads = read_form(&self.ctx, &input).await?;
        let machine = reads.scope.machine;
        if let Some(path) = &input.unit {
            require_live_unit(&machine, path)?;
        }
        let types = collect_stored_types(
            machine.id(),
            machine.platform(),
            machine.string_encoding(),
            reads.rows.clone(),
        )?;
        let repairs = input.purpose == ParameterFormPurpose::UpdateParameterDefinition;
        let choices = proposals(&types);
        let type_text = stored_type_text(&types, reads.parameter.as_ref(), repairs)?;
        let rule_faults = rule_faults_of(reads.parameter.as_ref(), &types, repairs)?;
        if let Some(expr) = &input.type_expr {
            require_form_type(&types, expr, (input.purpose, reads.parameter.as_ref()))?;
        }
        let preview = preview_of(&machine, reads.rows, input.type_expr.as_ref())?;
        let units = live_units(&machine);
        Ok(ParameterForm {
            recipe: reads.scope.recipe,
            machine,
            detail: reads.scope.detail,
            units,
            unit: input.unit,
            choices,
            parameter: reads.parameter,
            type_text,
            preview,
            rule_faults,
        })
    }
}
