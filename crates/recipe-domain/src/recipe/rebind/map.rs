//! The mapping of every affected parameter into one result (design D13, step 7, ADR-0027).

use std::collections::BTreeMap;

use super::RebindRequest;
use super::limit::{LimitMapping, map_limits};
use super::value::{ValueMapping, map_value};
use crate::plant::{TypeTree, TypeTrees};
use crate::recipe::{Draft, ParameterKey, RecipeErrors, StoredLimit, StoredValue};

/// The new values and limits of the affected parameters.
pub(super) struct Mapped {
    pub(super) values: BTreeMap<ParameterKey, StoredValue>,
    pub(super) limits: BTreeMap<ParameterKey, Vec<StoredLimit>>,
}

/// Map the value and the limits of each affected parameter to its planned tree.
///
/// The function visits the parameters in tag order and collects the entries of all of them.
/// The entries of the value of a parameter come before the entries of its limits. The function
/// stops for no parameter before the end.
///
/// # Errors
///
/// Returns the list of every entry of every parameter. The draft stays unchanged.
pub(super) fn map_parameters(
    draft: &Draft,
    request: &RebindRequest,
    trees: &TypeTrees,
    planned: &BTreeMap<ParameterKey, TypeTree>,
) -> Result<Mapped, RecipeErrors> {
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
        let value = map_value(&ValueMapping {
            key: parameter.key(),
            tag: parameter.tag(),
            request,
            old_tree,
            new_tree,
            old_value: parameter.value(),
        });
        let limits = map_limits(&LimitMapping {
            tag: parameter.tag(),
            old_tree,
            new_tree,
            rows: parameter.limits(),
        });
        match (value, limits) {
            (Ok(value), Ok(limits)) => {
                mapped.values.insert(parameter.key(), value);
                mapped.limits.insert(parameter.key(), limits);
            }
            (value, limits) => {
                entries.extend(value.err().into_iter().flatten());
                entries.extend(limits.err().into_iter().flatten());
            }
        }
    }
    match RecipeErrors::from_vec(entries) {
        Some(list) => Err(list),
        None => Ok(mapped),
    }
}
