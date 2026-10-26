//! The use case `CreateRecipe` (`domain-model.md` section 5.2, `data-model.md` section 6).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{MachineCode, MachineId, PlantError};
use recipe_domain::recipe::{Recipe, RecipeError, RecipeId, RecipeVersionId, ReferenceNumber};
use recipe_domain::text::{LongText, Text};
use uuid::Uuid;

use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, RECIPE_REFERENCE_UNIQUE, map_storage};
use crate::ports::{InsertedRecipe, Isolation, Locked, NewVersion, NonEmpty, Transaction};

/// The parsed request of `CreateRecipe`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateRecipeInput {
    /// The code of the machine in the request field `machine_code`.
    pub machine_code: MachineCode,
    /// The reference number of the new recipe.
    pub reference: ReferenceNumber,
    /// The name of the new recipe.
    pub name: Text,
    /// The change note of version 1.
    pub change_note: LongText,
}

/// The result of `CreateRecipe`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreateRecipeOutput {
    /// The identifier of the new recipe.
    pub recipe: RecipeId,
    /// The identifier of version 1.
    pub version: RecipeVersionId,
}

/// Creates a recipe with an empty first version.
///
/// The use case solves one problem: a recipe must not appear on a machine that a concurrent
/// command archives. It locks the machine `FOR SHARE` and requires it live under that lock
/// (`data-model.md` section 6). The unique constraint `recipe_reference_unique` guards the
/// reference number.
#[derive(Clone)]
pub struct CreateRecipe {
    ctx: AppContext,
}

impl UseCase for CreateRecipe {
    const OPERATION: Operation = Operation::CreateRecipe;
}

/// Resolve the live machine of the request field `machine_code` in a `ReadOnly` transaction.
///
/// The function solves one problem: the field names a machine, so a missing machine gives
/// `E-PLT-019` and not `E-WEB-005` (`api.md` section 1). A machine that exists only as an
/// archived machine gives `E-PLT-010`.
async fn live_machine_of_field(ctx: &AppContext, code: MachineCode) -> Result<MachineId, AppError> {
    let mut tx = ctx.uow.begin(Isolation::ReadOnly).await?;
    if let Some(machine) = tx.plant().machine_by_code(code).await? {
        return Ok(machine.id());
    }
    let archived = tx.plant().archived_machine_by_code(code).await?;
    Err(if archived.is_some() {
        PlantError::MachineArchived { code }
    } else {
        PlantError::MachineNotFound { code }
    }
    .into())
}

impl CreateRecipe {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Create the recipe.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `RecipeCreate`.
    /// Returns `PlantError::MachineNotFound` (`E-PLT-019`) when no machine has the code.
    /// Returns `PlantError::MachineArchived` (`E-PLT-010`) when the machine is archived.
    /// Returns `RecipeError::DuplicateReference` (`E-RCP-001`) when the reference number is in use.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: CreateRecipeInput,
    ) -> Result<CreateRecipeOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let code = input.machine_code;
        let machine_id = live_machine_of_field(&self.ctx, code).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        lock_live_machine(tx.as_mut(), machine_id, code).await?;
        let (output, inserted) = insert_new_recipe(tx.as_mut(), user, machine_id, input).await?;
        let entries = creation_entries(user, meta, output, inserted);
        self.ctx.uow.commit(tx, entries).await?;
        Ok(output)
    }
}

/// Lock the machine `FOR SHARE` and require it live under that lock.
async fn lock_live_machine(
    tx: &mut dyn Transaction,
    machine_id: MachineId,
    code: MachineCode,
) -> Result<(), AppError> {
    let Locked { state: machine, .. } = tx
        .plant()
        .lock_machine_shared(machine_id)
        .await?
        .ok_or(PlantError::MachineNotFound { code })?;
    if !machine.is_live() {
        return Err(PlantError::MachineArchived { code }.into());
    }
    Ok(())
}

/// Insert the recipe with its version 1 and map a taken reference number to `E-RCP-001`.
async fn insert_new_recipe(
    tx: &mut dyn Transaction,
    user: &CurrentUser,
    machine_id: MachineId,
    input: CreateRecipeInput,
) -> Result<(CreateRecipeOutput, InsertedRecipe), AppError> {
    let output = CreateRecipeOutput {
        recipe: RecipeId::from(Uuid::new_v4()),
        version: RecipeVersionId::from(Uuid::new_v4()),
    };
    let recipe = Recipe::create(
        output.recipe,
        machine_id,
        input.reference.clone(),
        input.name,
        output.version,
    );
    let first = NewVersion {
        recipe: output.recipe,
        version: output.version,
        number: 1,
        based_on: None,
        change_note: input.change_note,
        creator: user.user(),
    };
    let inserted = tx
        .recipes()
        .insert_recipe(&recipe, first)
        .await
        .map_err(|error| {
            map_storage(error, |constraint| {
                (constraint == RECIPE_REFERENCE_UNIQUE).then(|| {
                    RecipeError::DuplicateReference {
                        reference: input.reference.clone(),
                    }
                    .into()
                })
            })
        })?;
    Ok((output, inserted))
}

/// Build the audit entries `RecipeCreated` and `VersionCreated` of one creation.
fn creation_entries(
    user: &CurrentUser,
    meta: &RequestMeta,
    output: CreateRecipeOutput,
    inserted: InsertedRecipe,
) -> NonEmpty<AuditEntry> {
    let context = audit_context(user, meta);
    let mut entries = NonEmpty::new(AuditEntry::row(
        RowAction::RecipeCreated,
        AuditTarget::recipe(output.recipe),
        RowChange::Create {
            after: inserted.recipe.into_value(),
        },
        context.clone(),
    ));
    entries.push(AuditEntry::row(
        RowAction::VersionCreated,
        AuditTarget::recipe_version(output.recipe, output.version),
        RowChange::Create {
            after: inserted.version.into_value(),
        },
        context,
    ));
    entries
}

#[cfg(test)]
mod tests;
