//! The use of a type revision in the in-memory `TypeCatalog` (design D4).
//!
//! The walk goes from the type to every type that holds it through a member or an array element.
//! A set of visited types ends the walk, so a cycle of rows cannot loop. The rows come in the
//! same order as the `ORDER BY` of the statement of `recipe-db`.

use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::plant::{MachineId, PlcTypeId, RecordShape};

use crate::memory::MemoryState;
use crate::memory::recipe::ParameterRow;
use crate::ports::TypeUsageRow;

/// Return one page of the parameters of `machine` that use the type `type_id`.
pub(super) fn usage(
    state: &MemoryState,
    machine: MachineId,
    type_id: PlcTypeId,
    offset: u64,
    limit: u32,
) -> Vec<TypeUsageRow> {
    let holders = holders_of(state, type_id);
    let mut rows: Vec<TypeUsageRow> = state
        .parameters
        .values()
        .filter(|parameter| holders.contains(&parameter.plc_type))
        .filter_map(|parameter| row_of(state, machine, parameter))
        .collect();
    rows.sort_by(|left, right| {
        left.reference
            .cmp(&right.reference)
            .then_with(|| right.number.cmp(&left.number))
            .then_with(|| left.tag.cmp(&right.tag))
    });
    // A count that does not fit `usize` is larger than any list, so it keeps its meaning.
    let skip = usize::try_from(offset).unwrap_or(usize::MAX);
    let take = usize::try_from(limit).unwrap_or(usize::MAX);
    rows.into_iter().skip(skip).take(take).collect()
}

/// Return `type_id` and every type that holds it, on any level.
fn holders_of(state: &MemoryState, type_id: PlcTypeId) -> BTreeSet<PlcTypeId> {
    let mut holders_by_inner: BTreeMap<PlcTypeId, Vec<PlcTypeId>> = BTreeMap::new();
    for row in state.types.values() {
        let holder = row.plc_type.id();
        for inner in inner_types(row.plc_type.shape()) {
            holders_by_inner.entry(inner).or_default().push(holder);
        }
    }
    let mut seen = BTreeSet::from([type_id]);
    let mut pending = vec![type_id];
    while let Some(current) = pending.pop() {
        for holder in holders_by_inner.get(&current).into_iter().flatten() {
            if seen.insert(*holder) {
                pending.push(*holder);
            }
        }
    }
    seen
}

/// Return the types that a row holds directly: the types of its members or its element type.
fn inner_types(shape: &RecordShape) -> Vec<PlcTypeId> {
    match shape {
        RecordShape::Primitive { .. } => Vec::new(),
        RecordShape::Array { element, .. } => vec![*element],
        RecordShape::Struct { members, .. } => {
            members.iter().map(|member| member.type_id).collect()
        }
    }
}

/// Build the row of a parameter, or `None` when its recipe is deleted, its version is discarded,
/// or its recipe belongs to another machine.
fn row_of(
    state: &MemoryState,
    machine: MachineId,
    parameter: &ParameterRow,
) -> Option<TypeUsageRow> {
    let version = state.versions.get(&parameter.version)?;
    let recipe = state.recipes.get(&version.recipe)?;
    let counts =
        recipe.machine == machine && recipe.deleted_at.is_none() && version.deleted_at.is_none();
    counts.then(|| TypeUsageRow {
        reference: recipe.reference.clone(),
        number: version.number,
        status: version.status,
        tag: parameter.tag.clone(),
        parameter_type: parameter.plc_type,
    })
}

#[cfg(test)]
mod tests;
