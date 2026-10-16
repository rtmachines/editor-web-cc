//! The query `GetParameterForm` (design D4, ADR-0039 (h), ADR-0043 (e)).
//!
//! The spec is "Permission first in each recipe query". The form of a new parameter and the edit
//! form of a parameter read through this query. With a `TypeExpr`, it returns the preview tree of
//! `preview.rs` and writes nothing.

use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::plant::{
    Machine, PlatformTypes, PlcType, Primitive, RecordShape, StringKind, StringLength, TypeExpr,
    TypeName, TypeTree, Unit, UnitPath,
};
use recipe_domain::recipe::{
    ParameterKey, Recipe, RecipeError, ReferenceNumber, StoredParameter, VersionState,
};

use super::parameters::{VersionScope, read_scope, text_of};
use super::preview::preview_tree;
use crate::auth::{
    CurrentUser, FormPurpose, Operation, ParameterFormPurpose, RequestMeta, authorize,
};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, VersionDetail};

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

/// Refuse a version that is not a live draft, because only a live draft gets the forms (ADR-0043 (k)).
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

/// Read the version, the type rows, and the parameter of the key in one `RepeatableRead` transaction.
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

/// Return the proposals of the type input, as `GetUdtForm` writes them (`plant/queries/types.rs`).
///
/// First come the primitives of the platform without `STRING` and `WSTRING`. Then come `STRING`,
/// and `WSTRING` when the platform has it. The latest revision of each struct follows as
/// `<name>@<revision>` in the byte order of the name.
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
    let mut latest: BTreeMap<&TypeName, u32> = BTreeMap::new();
    for row in types.rows() {
        if let RecordShape::Struct { name, revision, .. } = row.shape() {
            let entry = latest.entry(name).or_insert(*revision);
            *entry = (*entry).max(*revision);
        }
    }
    primitives
        .into_iter()
        .chain(strings)
        .map(|primitive| primitive.name().to_owned())
        .chain(
            latest
                .iter()
                .map(|(name, revision)| format!("{name}@{revision}")),
        )
        .collect()
}

/// Prepares the form of a new parameter and the edit form of a parameter.
///
/// The query solves one problem. The form needs the live units, the type proposals, and the
/// parameter that it edits. It also needs the leaf rows of a type that the catalog may not hold yet.
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
    /// Returns `NotFound` (`E-WEB-005`) when the recipe, the version, the key, or the unit does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotEditable` (`E-RCP-002`) when the version is not a live draft.
    /// Returns `PlantError::DerivedTypeInvalid` (`E-PLT-013`) or `TypeTooLarge` (`E-PLT-018`) for the type.
    /// Returns `AppError::Storage` when the database fails.
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
        let types =
            PlatformTypes::collect(machine.id(), machine.platform(), reads.rows.iter().cloned())?;
        let choices = proposals(&types);
        let type_text = reads
            .parameter
            .as_ref()
            .map(|parameter| text_of(&types, parameter.plc_type))
            .transpose()?;
        let preview = input
            .type_expr
            .as_ref()
            .map(|expr| preview_tree(machine.id(), machine.platform(), reads.rows, expr))
            .transpose()?;
        let units = machine
            .units()
            .filter(|unit| unit.is_live())
            .cloned()
            .collect();
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
        })
    }
}
