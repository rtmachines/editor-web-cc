//! The queries `GetUnitTree`, `GetUnitForm`, and `GetUnit` (design D3, D19).
//!
//! The specs are "Unit tree of a machine" and "Proposal of a new unit".

use std::collections::BTreeSet;

use recipe_domain::plant::{Machine, MachineCode, PlantError, Unit, UnitPath};

use super::read_live_machine;
use crate::auth::{CurrentUser, FormPurpose, Operation, RequestMeta, UnitFormPurpose, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing};

/// The deepest level of a unit path (A30, `domain-model.md` section 2.1).
const MAX_DEPTH: u8 = 5;
/// The lowest segment of a unit path (A30).
const FIRST_SEGMENT: u8 = 1;
/// The highest segment of a unit path (A30).
const LAST_SEGMENT: u8 = 64;

/// The parsed request of `GetUnitTree`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitTreeInput {
    /// The code of the live machine.
    pub code: MachineCode,
}

/// Shows the unit tree of a live machine.
///
/// The query solves one problem: the tree page and the tree fragment need the machine with all its
/// units. The result keeps every unit as the port returns it, live or archived. The web layer
/// chooses the live units or every unit.
#[derive(Clone)]
pub struct GetUnitTree {
    ctx: AppContext,
}

impl UseCase for GetUnitTree {
    const OPERATION: Operation = Operation::GetUnitTree;
}

impl GetUnitTree {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the live machine with its units.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: UnitTreeInput,
    ) -> Result<Machine, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        read_live_machine(&self.ctx, input.code).await
    }
}

/// The parsed request of `GetUnitForm`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitFormInput {
    /// The command that the form serves. The query checks its permission (ADR-0046 (b)).
    pub purpose: UnitFormPurpose,
    /// The code of the live machine.
    pub code: MachineCode,
    /// The parent unit that the URL names. `None` means a main unit.
    pub parent: Option<UnitPath>,
}

/// The data of the form `New unit`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitForm {
    /// The live machine.
    pub machine: Machine,
    /// The live units of depth 1 to 4 in the order of the path. Each one can have a child.
    pub parents: Vec<Unit>,
    /// The chosen parent, or `None` for a main unit.
    pub parent: Option<UnitPath>,
    /// The lowest segment that no unit under the parent uses, or `None` when all are in use.
    pub segment: Option<u8>,
}

/// Prepares the form that adds a unit.
///
/// The query solves one problem: the form must offer only the parents that can have a child and
/// must propose a free segment. A segment of an archived unit stays in use, because the path of a
/// unit never returns to the pool.
#[derive(Clone)]
pub struct GetUnitForm {
    ctx: AppContext,
}

impl UseCase for GetUnitForm {
    const OPERATION: Operation = Operation::GetUnitForm;
}

impl GetUnitForm {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the parents and the proposed segment.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) with the command of the purpose when the role lacks its
    /// permission.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code, or when the parent is
    /// not a live unit of the machine.
    /// Returns `PlantError::UnitDepthExceeded` (`E-PLT-002`) when the parent has depth 5.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: UnitFormInput,
    ) -> Result<UnitForm, AppError> {
        authorize(
            input.purpose.operation(),
            user,
            meta,
            self.ctx.security.as_ref(),
        )
        .await?;
        let machine = read_live_machine(&self.ctx, input.code).await?;
        let segment = proposed_segment(&machine, input.parent.as_ref())?;
        let parents = machine
            .units()
            .filter(|unit| unit.is_live() && unit.path().depth() < MAX_DEPTH)
            .cloned()
            .collect();
        Ok(UnitForm {
            machine,
            parents,
            parent: input.parent,
            segment,
        })
    }
}

/// Propose the segment of a new unit under `parent`, or of a new main unit without a parent.
fn proposed_segment(machine: &Machine, parent: Option<&UnitPath>) -> Result<Option<u8>, AppError> {
    let Some(parent) = parent else {
        return Ok(lowest_free_segment(machine, None));
    };
    let live = machine
        .unit(parent)
        .filter(|unit| unit.is_live())
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::Unit {
                code: machine.code(),
                path: parent.clone(),
            },
        })?;
    if live.path().depth() >= MAX_DEPTH {
        return Err(PlantError::UnitDepthExceeded {
            path: format!("{parent}.{FIRST_SEGMENT}"),
        }
        .into());
    }
    Ok(lowest_free_segment(machine, Some(parent)))
}

/// Return the lowest segment that no direct child of `parent` uses, live or archived.
fn lowest_free_segment(machine: &Machine, parent: Option<&UnitPath>) -> Option<u8> {
    let used: BTreeSet<u8> = machine
        .units()
        .filter(|unit| is_child_of(unit.path(), parent))
        .filter_map(|unit| unit.path().segments().last())
        .collect();
    (FIRST_SEGMENT..=LAST_SEGMENT).find(|segment| !used.contains(segment))
}

/// Tell whether `path` is a direct child of `parent`. Without a parent, the children are the main units.
fn is_child_of(path: &UnitPath, parent: Option<&UnitPath>) -> bool {
    match parent {
        Some(parent) => path.parent().as_ref() == Some(parent),
        None => path.depth() == 1,
    }
}

/// The parsed request of `GetUnit`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitRef {
    /// The code of the live machine.
    pub code: MachineCode,
    /// The path of the unit.
    pub path: UnitPath,
}

/// The result of `GetUnit`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitView {
    /// The live machine.
    pub machine: Machine,
    /// The unit, live or archived.
    pub unit: Unit,
    /// The parent of the unit, or `None` for a main unit.
    pub parent: Option<Unit>,
    /// The live units directly under the unit, in the order of the path.
    pub children: Vec<Unit>,
    /// The number of live units at any level under the unit.
    pub descendants: usize,
}

/// Shows one unit of a live machine.
///
/// The query solves one problem: the unit page and the archive warning need the unit, its parent,
/// its children, and the number of units that an archive also archives.
#[derive(Clone)]
pub struct GetUnit {
    ctx: AppContext,
}

impl UseCase for GetUnit {
    const OPERATION: Operation = Operation::GetUnit;
}

impl GetUnit {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the unit with its parent, its live children, and its live descendants.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code or the machine has no such unit.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: UnitRef,
    ) -> Result<UnitView, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let machine = read_live_machine(&self.ctx, input.code).await?;
        let unit = machine
            .unit(&input.path)
            .cloned()
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::Unit {
                    code: input.code,
                    path: input.path.clone(),
                },
            })?;
        let parent = unit
            .path()
            .parent()
            .and_then(|path| machine.unit(&path))
            .cloned();
        let children = machine
            .units()
            .filter(|child| child.is_live() && is_child_of(child.path(), Some(unit.path())))
            .cloned()
            .collect();
        let descendants = machine
            .units()
            .filter(|other| other.is_live() && unit.path().is_ancestor_of(other.path()))
            .count();
        Ok(UnitView {
            machine,
            unit,
            parent,
            children,
            descendants,
        })
    }
}
