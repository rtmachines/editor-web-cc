//! The mapping of every affected parameter into one result (design D13, step 7, ADR-0027).

use std::collections::BTreeMap;

use super::RebindRequest;
use super::limit::{LimitMapping, kept_limit_defect, map_limits};
use super::value::{ValueMapping, map_value};
use crate::plant::{TypeTree, TypeTrees};
use crate::recipe::draft::stored_value;
use crate::recipe::{
    Draft, DraftError, Parameter, ParameterKey, RecipeError, RecipeErrors, StoredDefect,
    StoredLimit, StoredValue,
};

/// The failure of the mapping of one value or one limit list.
///
/// The type solves one problem: an entry is a fault of the request, and a defect is a fault of
/// the stored data. The two must not share one list, because a defect stops the plan with
/// `E-DB-008` and no list (ADR-0195 (n)).
pub(super) enum MapFault {
    /// The stored data of the parameter is defective. It carries the first fault.
    Defect(RecipeError),
    /// The request or a dropped limit gives these entries.
    Entries(Vec<RecipeError>),
}

/// The new values and limits of the affected parameters.
pub(super) struct Mapped {
    pub(super) values: BTreeMap<ParameterKey, StoredValue>,
    pub(super) limits: BTreeMap<ParameterKey, Vec<StoredLimit>>,
}

/// Map the value and the limits of each affected parameter to its planned tree.
///
/// The function visits the parameters in tag order. A stored defect of a parameter stops it
/// at once. Otherwise it collects the entries of all parameters. The entries of the value of a
/// parameter come before the entries of its limits. The I7 check of the kept value against the
/// kept limits runs also when the parameter has entries (ADR-0195 (l), (n)).
///
/// # Errors
///
/// Returns `Stored` with the key of the first parameter with a stored defect, in tag order. It
/// drops every entry. Without a defect it returns the list of every entry of every parameter.
/// The draft stays unchanged.
pub(super) fn map_parameters(
    draft: &Draft,
    request: &RebindRequest,
    trees: &TypeTrees,
    planned: &BTreeMap<ParameterKey, TypeTree>,
) -> Result<Mapped, DraftError> {
    let mut mapped = Mapped {
        values: BTreeMap::new(),
        limits: BTreeMap::new(),
    };
    let mut entries = Vec::new();
    for parameter in draft.parameters() {
        let (Some(new_tree), Some(old_tree)) = (
            planned.get(&parameter.key()),
            trees.tree(parameter.plc_type()),
        ) else {
            continue;
        };
        match map_one_parameter(parameter, request, old_tree, new_tree)? {
            ParameterMapping::Kept { value, limits } => {
                mapped.values.insert(parameter.key(), value);
                mapped.limits.insert(parameter.key(), limits);
            }
            ParameterMapping::Entries(found) => entries.extend(found),
        }
    }
    match RecipeErrors::from_vec(entries) {
        Some(list) => Err(DraftError::from(list)),
        None => Ok(mapped),
    }
}

/// The result of the mapping of one parameter that has no stored defect.
///
/// The type solves one problem: the loop of `map_parameters` must tell a kept pair from a list
/// of entries without a nested `Result`.
enum ParameterMapping {
    /// The value and the limits map without an entry.
    Kept {
        value: StoredValue,
        limits: Vec<StoredLimit>,
    },
    /// The value or the limits give these entries. The entries of the value come first.
    Entries(Vec<RecipeError>),
}

/// Map the value and the limits of one parameter to its planned tree.
///
/// The function checks the stored defects in this order. It checks the old value first.
/// Then it checks the mapping of the value and the mapping of the limits.
/// Last, it runs the I7 check of the kept value against the kept limits.
/// The I7 check runs also when the parameter has entries.
///
/// # Errors
///
/// Returns `Stored` with the first stored defect of the parameter. Entries are not errors
/// here. The function returns them in `ParameterMapping::Entries`.
fn map_one_parameter(
    parameter: &Parameter,
    request: &RebindRequest,
    old_tree: &TypeTree,
    new_tree: &TypeTree,
) -> Result<ParameterMapping, DraftError> {
    let key = parameter.key();
    let stored = |error| DraftError::Stored(Box::new(StoredDefect::of_parameter(key, error)));
    let old_value = match stored_value(parameter.tag(), parameter.value(), old_tree) {
        Ok(old_value) => old_value,
        Err(error) => return Err(stored(error)),
    };
    let value = map_value(&ValueMapping {
        key,
        tag: parameter.tag(),
        request,
        old_tree,
        new_tree,
        old_value: &old_value,
    });
    let limit_input = LimitMapping {
        tag: parameter.tag(),
        old_tree,
        new_tree,
        rows: parameter.limits(),
    };
    let limits = map_limits(&limit_input);
    if let Err(MapFault::Defect(error)) = value {
        return Err(stored(error));
    }
    if let Err(MapFault::Defect(error)) = limits {
        return Err(stored(error));
    }
    if let Some(error) = kept_limit_defect(&limit_input, &old_value) {
        return Err(stored(error));
    }
    Ok(match (value, limits) {
        (Ok(value), Ok(limits)) => ParameterMapping::Kept { value, limits },
        (value, limits) => {
            let mut entries = entries_of(value);
            entries.extend(entries_of(limits));
            ParameterMapping::Entries(entries)
        }
    })
}

/// Return the entries of a failed mapping. A successful mapping has none.
fn entries_of<T>(result: Result<T, MapFault>) -> Vec<RecipeError> {
    match result {
        Err(MapFault::Entries(entries)) => entries,
        Ok(_) | Err(MapFault::Defect(_)) => Vec::new(),
    }
}
