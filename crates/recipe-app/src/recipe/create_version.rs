//! The use case `CreateVersion` (`domain-model.md` section 5.2, `data-model.md` section 6, design D13).

use std::collections::BTreeSet;

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{
    LiveUnits, Machine, MachineId, PlantError, PlatformTypes, PlcPlatform, PlcType, TypeTrees,
    Unit, UnitId,
};
use recipe_domain::recipe::{
    BudgetCounts, Parameter, ParameterTag, Recipe, RecipeError, RecipeId, RecipeVersionId,
    ReferenceNumber, SnapshotItem, StoredParameter, VersionSnapshot, check_budgets,
};
use recipe_domain::text::LongText;
use uuid::Uuid;

use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, Missing, RECIPE_VERSION_ONE_LIVE_DRAFT, map_storage};
use crate::ports::{Isolation, Locked, NewVersion, NonEmpty, Transaction};

/// The parsed request of `CreateVersion`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateVersionInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the base version, or `None` for an empty draft (ADR-0028).
    pub based_on: Option<u32>,
    /// The change note of the new version.
    pub change_note: LongText,
}

/// The result of `CreateVersion`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateVersionOutput {
    /// The identifier of the new version.
    pub version: RecipeVersionId,
    /// The number of the new version.
    pub number: u32,
    /// The tags of the parameters that use an outdated type revision.
    pub outdated: Vec<ParameterTag>,
}

/// Creates a live draft as a copy of a base version.
///
/// The use case solves one problem: a new draft must not copy a parameter that references an
/// archived unit, an archived machine, or a type that left the platform. It checks the base
/// version before the transaction with `Blocking`, and it checks the units again under their
/// `FOR SHARE` locks (design D13, ADR-0015).
#[derive(Clone)]
pub struct CreateVersion {
    ctx: AppContext,
}

impl UseCase for CreateVersion {
    const OPERATION: Operation = Operation::CreateVersion;
}

/// What step 2 of design D10 reads about the base version.
struct BaseRead {
    version: RecipeVersionId,
    parameters: Vec<StoredParameter>,
    units: LiveUnits,
    types: Vec<PlcType>,
}

/// What step 2 of design D10 reads without a lock.
struct Read {
    recipe: Recipe,
    machine: Machine,
    base: Option<BaseRead>,
}

/// Read the parameters, the live units, and the type rows for the base version.
async fn read_base(
    tx: &mut dyn Transaction,
    version: RecipeVersionId,
    machine: MachineId,
) -> Result<BaseRead, AppError> {
    let parameters = tx.recipes().version_parameters(version).await?;
    let units = tx
        .units()
        .live_units(machine)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::machine_id(machine),
        })?;
    let types = tx.types().type_rows(machine).await?;
    Ok(BaseRead {
        version,
        parameters,
        units,
        types,
    })
}

/// Read the recipe, the machine, and the base version in a `ReadOnly` transaction.
///
/// The function solves one problem: step 2 of design D10 needs several reads in one snapshot.
/// The transaction takes no lock and ends before the function returns.
async fn read_inputs(ctx: &AppContext, input: &CreateVersionInput) -> Result<Read, AppError> {
    let mut tx = ctx.uow.begin(Isolation::ReadOnly).await?;
    let recipe = tx
        .recipes()
        .recipe_by_reference(&input.reference)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::Recipe(input.reference.clone()),
        })?;
    let version = input
        .based_on
        .map(|number| {
            recipe
                .version(number)
                .map(recipe_domain::recipe::VersionHeader::id)
                .ok_or_else(|| AppError::NotFound {
                    entity: Missing::Version {
                        reference: input.reference.clone(),
                        number,
                    },
                })
        })
        .transpose()?;
    let machine_id = recipe.machine();
    let machine =
        tx.plant()
            .machine_by_id(machine_id)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::machine_id(machine_id),
            })?;
    let base = match version {
        Some(version) => Some(read_base(tx.as_mut(), version, machine_id).await?),
        None => None,
    };
    Ok(Read {
        recipe,
        machine,
        base,
    })
}

/// Check the root on a copy and require a live machine (design D13 step 1).
///
/// The function solves one problem: a deleted recipe must give `E-RCP-012`, not an item of
/// `E-RCP-020`. The copy keeps the state of step 2 away from the command (design D10).
fn check_root_and_machine(recipe: &Recipe, machine: &Machine) -> Result<(), AppError> {
    recipe
        .clone()
        .create_version(RecipeVersionId::from(Uuid::new_v4()))?;
    if machine.is_live() {
        Ok(())
    } else {
        Err(PlantError::MachineArchived {
            code: machine.code(),
        }
        .into())
    }
}

fn count(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
}

fn total(mut values: impl Iterator<Item = u64>) -> u64 {
    values.try_fold(0_u64, u64::checked_add).unwrap_or(u64::MAX)
}

/// Count the parameters, the leaves, and the limit rows of a version for the size budgets.
fn budget_counts(parameters: &[Parameter], trees: &TypeTrees) -> BudgetCounts {
    BudgetCounts {
        parameters: count(parameters.len()),
        leaves: total(
            parameters
                .iter()
                .filter_map(|parameter| trees.tree(parameter.plc_type()))
                .map(|tree| u64::from(tree.leaf_count())),
        ),
        limit_rows: total(
            parameters
                .iter()
                .map(|parameter| count(parameter.limits().len())),
        ),
    }
}

/// Run the snapshot check and the size budgets on a blocking thread (design D13 step 2).
///
/// # Errors
///
/// Returns `E-RCP-020` with the items, or `E-RCP-021` for a budget above its limit.
fn validate_base(
    recipe: &Recipe,
    (machine, platform): (MachineId, PlcPlatform),
    base: BaseRead,
) -> Result<Vec<ParameterTag>, AppError> {
    let types = PlatformTypes::collect(machine, platform, base.types)?;
    let snapshot = VersionSnapshot::from(base.parameters);
    let parameters = snapshot.parameters();
    let trees = TypeTrees::resolve(parameters.iter().map(|p| p.plc_type()), &types);
    let outdated = snapshot.check(recipe, &base.units, &trees)?;
    check_budgets(budget_counts(parameters, &trees))?;
    Ok(outdated)
}

/// The base version after the check before the transaction.
struct BaseChecked {
    version: RecipeVersionId,
    /// The tag and the unit of each parameter, in tag order.
    unit_uses: Vec<(ParameterTag, UnitId)>,
    outdated: Vec<ParameterTag>,
}

/// Check the base version with `Blocking` (design D13 step 2).
async fn check_base(
    ctx: &AppContext,
    (recipe, machine): (&Recipe, &Machine),
    base: BaseRead,
) -> Result<BaseChecked, AppError> {
    let version = base.version;
    let mut unit_uses: Vec<(ParameterTag, UnitId)> = base
        .parameters
        .iter()
        .map(|parameter| (parameter.tag.clone(), parameter.unit))
        .collect();
    unit_uses.sort();
    let target = (machine.id(), machine.platform());
    let copy = recipe.clone();
    let outdated = ctx
        .settings
        .blocking
        .validate(move || validate_base(&copy, target, base))
        .await??;
    Ok(BaseChecked {
        version,
        unit_uses,
        outdated,
    })
}

/// Take `machine FOR SHARE` and require a live machine (`data-model.md` section 6).
async fn lock_live_machine(tx: &mut dyn Transaction, id: MachineId) -> Result<(), AppError> {
    let Locked { state: machine, .. } =
        tx.plant()
            .lock_machine_shared(id)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::machine_id(id),
            })?;
    if machine.is_live() {
        Ok(())
    } else {
        Err(PlantError::MachineArchived {
            code: machine.code(),
        }
        .into())
    }
}

/// Report the item of a parameter whose unit is archived or unknown under the lock.
fn locked_unit_item(tag: &ParameterTag, unit: UnitId, locked: &[Unit]) -> Option<SnapshotItem> {
    match locked.iter().find(|candidate| candidate.id() == unit) {
        Some(found) if found.is_live() => None,
        Some(found) => Some(SnapshotItem::Parameter {
            tag: tag.clone(),
            error: PlantError::UnitArchived {
                path: found.path().clone(),
            }
            .into(),
        }),
        None => Some(SnapshotItem::UnknownUnit {
            tag: tag.clone(),
            unit,
        }),
    }
}

/// Take `unit FOR SHARE` on the units of the base version and check `archived_at` again.
///
/// The function solves one problem: `ArchiveUnit` can archive a unit between the snapshot
/// check and the locks (I19). The locks stop it, and this check sees its result.
///
/// # Errors
///
/// Returns `E-RCP-020` with one item for each parameter on an archived unit.
async fn recheck_units(
    tx: &mut dyn Transaction,
    unit_uses: &[(ParameterTag, UnitId)],
) -> Result<(), AppError> {
    let ids: Vec<UnitId> = unit_uses
        .iter()
        .map(|(_, unit)| *unit)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let locked = tx.units().lock_units_shared(&ids).await?;
    let items: Vec<SnapshotItem> = unit_uses
        .iter()
        .filter_map(|(tag, unit)| locked_unit_item(tag, *unit, &locked))
        .collect();
    if items.is_empty() {
        Ok(())
    } else {
        Err(RecipeError::ValidationFailed { items }.into())
    }
}

/// The identifiers that step 2 of design D10 gives to the transaction.
#[derive(Clone, Copy)]
struct Target {
    recipe: RecipeId,
    machine: MachineId,
}

impl CreateVersion {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Create the version.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `VersionCreate`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the base version does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::DraftAlreadyExists` (`E-RCP-011`) when a live draft exists.
    /// Returns `PlantError::MachineArchived` (`E-PLT-010`) when the machine is archived.
    /// Returns `RecipeError::ValidationFailed` (`E-RCP-020`) when the base version is not valid.
    /// Returns `RecipeError::SizeBudgetExceeded` (`E-RCP-021`) when a budget is exceeded.
    /// Returns `AppError::Timeout` (`E-WEB-009`) or `AppError::TaskFailed` (`E-WEB-011`)
    /// when the snapshot check does not complete.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: CreateVersionInput,
    ) -> Result<CreateVersionOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let Read {
            recipe,
            machine,
            base,
        } = read_inputs(&self.ctx, &input).await?;
        check_root_and_machine(&recipe, &machine)?;
        let checked = match base {
            Some(base) => Some(check_base(&self.ctx, (&recipe, &machine), base).await?),
            None => None,
        };
        let target = Target {
            recipe: recipe.id(),
            machine: machine.id(),
        };
        self.write_version((user, meta), input, target, checked)
            .await
    }

    /// Run the transaction of design D13 step 3 and commit the new version.
    async fn write_version(
        &self,
        (user, meta): (&CurrentUser, &RequestMeta),
        input: CreateVersionInput,
        target: Target,
        checked: Option<BaseChecked>,
    ) -> Result<CreateVersionOutput, AppError> {
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let version = RecipeVersionId::from(Uuid::new_v4());
        let unit_uses = checked.as_ref().map(|base| base.unit_uses.as_slice());
        let number =
            lock_and_number(tx.as_mut(), target, &input.reference, version, unit_uses).await?;
        let new = NewVersion {
            recipe: target.recipe,
            version,
            number,
            based_on: checked.as_ref().map(|base| base.version),
            change_note: input.change_note,
            creator: user.user(),
        };
        let entry = insert_version_entry(tx.as_mut(), new, &input.reference, (user, meta)).await?;
        self.ctx.uow.commit(tx, NonEmpty::new(entry)).await?;
        Ok(CreateVersionOutput {
            version,
            number,
            outdated: checked.map(|base| base.outdated).unwrap_or_default(),
        })
    }
}

/// Take the machine and recipe locks, number the version, and check the units again.
///
/// The domain call comes before the unit check: a deleted recipe gives `E-RCP-012` before any
/// unit item.
async fn lock_and_number(
    tx: &mut dyn Transaction,
    target: Target,
    reference: &ReferenceNumber,
    version: RecipeVersionId,
    unit_uses: Option<&[(ParameterTag, UnitId)]>,
) -> Result<u32, AppError> {
    lock_live_machine(tx, target.machine).await?;
    let Locked {
        state: mut recipe, ..
    } = tx
        .recipes()
        .lock_recipe(target.recipe)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::Recipe(reference.clone()),
        })?;
    let number = recipe.create_version(version)?.number();
    if let Some(unit_uses) = unit_uses {
        recheck_units(tx, unit_uses).await?;
    }
    Ok(number)
}

/// Insert the version row and build its audit entry.
///
/// A second live draft gives `E-RCP-011`.
async fn insert_version_entry(
    tx: &mut dyn Transaction,
    new: NewVersion,
    reference: &ReferenceNumber,
    (user, meta): (&CurrentUser, &RequestMeta),
) -> Result<AuditEntry, AppError> {
    let (recipe, version, number) = (new.recipe, new.version, new.number);
    let image = tx.recipes().insert_version(new).await.map_err(|error| {
        map_storage(error, |constraint| {
            (constraint == RECIPE_VERSION_ONE_LIVE_DRAFT).then(|| {
                RecipeError::DraftAlreadyExists {
                    reference: reference.clone(),
                    number,
                }
                .into()
            })
        })
    })?;
    Ok(AuditEntry::row(
        RowAction::VersionCreated,
        AuditTarget::recipe_version(recipe, version),
        RowChange::Create {
            after: image.into_value(),
        },
        audit_context(user, meta),
    ))
}

#[cfg(test)]
mod tests;
