//! The outdated parameters of a draft (`E-RCP-024`, ADR-0018, ADR-0043 (l), design D4).

use std::collections::BTreeMap;
use std::fmt;

use recipe_domain::plant::{
    Machine, PlantError, PlatformTypes, PlcType, PlcTypeId, RecordShape, TypeName, TypeNode,
    TypeTree, TypeTrees, UnitPath,
};
use recipe_domain::recipe::{ParameterKey, ParameterTag, RebindLeaves, rebind_new_leaves};

use crate::error::{AppError, StorageError};
use crate::ports::ParameterRow;

/// A parameter whose type uses an older revision of a struct.
///
/// The type solves one problem: the version page lists the outdated parameters, and the rebind
/// form needs the two type identifiers (`from_type_id` and `to_type_id` of `api.md` section 2.5)
/// and one input per new member leaf. The parameter can hold a second outdated struct inside the
/// first. After the rebind, the page shows the next one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutdatedParameter {
    /// The key of the parameter.
    pub key: ParameterKey,
    /// The path of the unit that holds the parameter.
    pub unit_path: UnitPath,
    /// The tag of the parameter.
    pub tag: ParameterTag,
    /// The name of the outdated struct.
    pub type_name: TypeName,
    /// The revision that the parameter uses.
    pub current: u32,
    /// The newest revision of the struct.
    pub newest: u32,
    /// The type row of the revision that the parameter uses.
    pub from: PlcTypeId,
    /// The type row of the newest revision.
    pub to: PlcTypeId,
    /// The planned tree and the new member leaves of the rebind from `from` to `to`.
    ///
    /// `None` when that rebind does not move the parameter, for example a struct member of type
    /// `from`. It is also `None` when the planned tree is above a size limit. The command then
    /// gives `E-PLT-018`. A caller without `ParameterEdit` gets `None`, and a page over the bound
    /// `form.rebind_leaves` gets `None` for every parameter (ADR-0044).
    pub rebind_leaves: Option<RebindLeaves>,
    /// The struct revision that holds the outdated struct as a member, or `None` at the root.
    ///
    /// An array between the two structs does not count. A rebind does not move a parameter with
    /// a holder. The page tells the user to make a new revision of the holder first. Every reader
    /// gets this value.
    pub holder: Option<StructRevision>,
}

/// One revision of a struct, for example `ST_Line@1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructRevision {
    /// The name of the struct.
    pub name: TypeName,
    /// The revision number.
    pub revision: u32,
}

impl fmt::Display for StructRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.revision)
    }
}

/// Return the struct revision that holds the first struct node `name` in `revision`.
///
/// The walk has the order of `TypeTrees::outdated`: depth first, in declaration order. So it finds
/// the node of that `Outdated`. An earlier node with the same name and revision would be outdated
/// too. An array between the two structs does not count. A node at the root has no holder.
fn holder_of(tree: &TypeTree, name: &TypeName, revision: u32) -> Option<StructRevision> {
    let mut stack: Vec<(&TypeNode, Option<(&TypeName, u32)>)> = vec![(tree.root(), None)];
    while let Some((node, holder)) = stack.pop() {
        match node {
            TypeNode::Leaf { .. } => {}
            TypeNode::Array { element, .. } => stack.push((element.as_ref(), holder)),
            TypeNode::Struct {
                name: found,
                revision: number,
                members,
                ..
            } => {
                if found == name && *number == revision {
                    return holder.map(|(name, revision)| StructRevision {
                        name: name.clone(),
                        revision,
                    });
                }
                let parent = Some((found, *number));
                stack.extend(members.iter().rev().map(|(_, child)| (child, parent)));
            }
        }
    }
    None
}

/// Find the type row of the struct `name` in `revision`.
fn struct_row(
    types: &PlatformTypes,
    name: &TypeName,
    revision: u32,
) -> Result<PlcTypeId, AppError> {
    types
        .rows()
        .find_map(|row| match row.shape() {
            RecordShape::Struct {
                name: found,
                revision: number,
                ..
            } if found == name && *number == revision => Some(row.id()),
            _ => None,
        })
        .ok_or_else(|| {
            StorageError::invariant("a struct revision of a tree has no type row").into()
        })
}

/// The type rows and the resolved trees that the new member leaves need.
struct Catalog<'a> {
    types: &'a PlatformTypes,
    trees: &'a TypeTrees,
    /// The resolved tree of each newest revision, read once per revision.
    targets: BTreeMap<PlcTypeId, TypeTree>,
}

impl Catalog<'_> {
    /// Return the new member leaves of a parameter of type `plc_type` for a rebind `from` to `to`.
    ///
    /// # Errors
    ///
    /// Returns the `PlantError` of the type rows when the newest revision does not resolve.
    fn rebind_leaves(
        &mut self,
        plc_type: PlcTypeId,
        (from, to): (PlcTypeId, PlcTypeId),
    ) -> Result<Option<RebindLeaves>, AppError> {
        let Some(tree) = self.trees.tree(plc_type) else {
            return Ok(None);
        };
        let target = match self.targets.get(&to) {
            Some(found) => found,
            None => {
                let resolved = TypeTree::resolve(to, self.types)?;
                self.targets.entry(to).or_insert(resolved)
            }
        };
        match rebind_new_leaves(tree, from, target) {
            Ok(found) => Ok(found),
            Err(PlantError::TypeTooLarge { .. }) => Ok(None),
            Err(other) => Err(other.into()),
        }
    }
}

/// The rebind work that one call of `GetVersion` may do (ADR-0044 (a) and (b)).
#[derive(Clone, Copy, Debug)]
pub(super) struct RebindBudget {
    /// True when the caller holds `ParameterEdit` and the version is a live draft.
    pub(super) computes: bool,
    /// The bound `form.rebind_leaves` on the sum of the new member leaves.
    pub(super) bound: usize,
}

/// The outdated parameters of a draft and the sum of their new member leaves.
#[derive(Debug, Default)]
pub(super) struct Outdated {
    /// The outdated parameters, in the order of the rows.
    pub(super) parameters: Vec<OutdatedParameter>,
    /// The sum of the new member leaves that the function computed.
    pub(super) sum: usize,
    /// True when `sum` passed the bound. No parameter then holds leaves.
    pub(super) over_bound: bool,
}

/// The running sum of the new member leaves (ADR-0044 (b)).
struct Tally {
    budget: RebindBudget,
    sum: usize,
    over_bound: bool,
}

impl Tally {
    /// Start the sum at 0 under `budget`.
    const fn new(budget: RebindBudget) -> Self {
        Self {
            budget,
            sum: 0,
            over_bound: false,
        }
    }

    /// Return the result. Over the bound, no parameter keeps its leaves (ADR-0044 (c)).
    fn finish(self, found: Vec<OutdatedParameter>) -> Outdated {
        let parameters = if self.over_bound {
            found
                .into_iter()
                .map(|item| OutdatedParameter {
                    rebind_leaves: None,
                    ..item
                })
                .collect()
        } else {
            found
        };
        Outdated {
            parameters,
            sum: self.sum,
            over_bound: self.over_bound,
        }
    }

    /// Return the new member leaves of one parameter while the budget permits the work.
    ///
    /// The sum is a count that the page shows, so it saturates (`.claude/rules/rust.md`).
    fn leaves(
        &mut self,
        catalog: &mut Catalog<'_>,
        plc_type: PlcTypeId,
        pair: (PlcTypeId, PlcTypeId),
    ) -> Result<Option<RebindLeaves>, AppError> {
        if !self.budget.computes || self.over_bound {
            return Ok(None);
        }
        let found = catalog.rebind_leaves(plc_type, pair)?;
        if let Some(leaves) = &found {
            self.sum = self.sum.saturating_add(leaves.leaves().len());
            self.over_bound = self.sum > self.budget.bound;
        }
        Ok(found)
    }
}

/// Return the parameters of `rows` whose type uses an older revision of a struct.
///
/// The function resolves the type trees of the rows against the type rows of the machine, so the
/// caller runs it after it ends the transaction (`reliability.md` section 7.4). When `budget`
/// permits it, the function also gives each parameter the new member leaves of its rebind. It
/// stops that work at the first parameter that passes the bound, and then removes all leaves.
///
/// # Errors
///
/// Returns the `PlantError` of the type rows when they break a rule of the domain.
pub(super) fn outdated_parameters(
    machine: &Machine,
    rows: &[ParameterRow],
    types: Vec<PlcType>,
    budget: RebindBudget,
) -> Result<Outdated, AppError> {
    if rows.is_empty() {
        return Ok(Outdated::default());
    }
    let types = PlatformTypes::collect(machine.id(), machine.platform(), types)?;
    let trees = TypeTrees::resolve(rows.iter().map(|row| row.plc_type), &types);
    outdated_with_trees(rows, (&types, &trees), budget)
}

/// Return the outdated parameters of `rows` with the type trees that the caller resolved.
///
/// The function solves one problem: a caller that already resolved the trees of the rows does
/// not resolve them a second time. `trees` can hold more trees than the rows use. A row whose
/// type has no tree in `trees` is not outdated, as in [`outdated_parameters`].
///
/// # Errors
///
/// Returns the `PlantError` of the type rows when they break a rule of the domain.
pub(super) fn outdated_with_trees(
    rows: &[ParameterRow],
    (types, trees): (&PlatformTypes, &TypeTrees),
    budget: RebindBudget,
) -> Result<Outdated, AppError> {
    let mut catalog = Catalog {
        types,
        trees,
        targets: BTreeMap::new(),
    };
    let mut tally = Tally::new(budget);
    let mut found = Vec::new();
    for row in rows {
        let Some(outdated) = trees.outdated(row.plc_type) else {
            continue;
        };
        let from = struct_row(types, outdated.type_name(), outdated.current())?;
        let to = struct_row(types, outdated.type_name(), outdated.newest())?;
        let holder = trees
            .tree(row.plc_type)
            .and_then(|tree| holder_of(tree, outdated.type_name(), outdated.current()));
        found.push(OutdatedParameter {
            key: row.key,
            unit_path: row.unit_path.clone(),
            tag: row.tag.clone(),
            type_name: outdated.type_name().clone(),
            current: outdated.current(),
            newest: outdated.newest(),
            from,
            to,
            rebind_leaves: tally.leaves(&mut catalog, row.plc_type, (from, to))?,
            holder,
        });
    }
    Ok(tally.finish(found))
}

#[cfg(test)]
mod tests;
