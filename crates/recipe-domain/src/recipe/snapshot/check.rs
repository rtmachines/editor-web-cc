//! The one check of the stored parameters of a version (design D12).
//!
//! `VersionSnapshot::check` and `Draft::check_for_approval` call this function, so the two
//! checks cannot differ.

use super::{SnapshotError, SnapshotItem};
use crate::plant::{DerivedTypeReason, LiveUnits, PlantError, TypeTree, TypeTrees};
use crate::recipe::draft::{check_value, stored_limits, stored_value};
use crate::recipe::{Parameter, ParameterTag, Recipe, RecipeError};

/// Check the parameters, the machine, and the recipe of a version.
///
/// `parameters` must come in tag order. The function collects every problem. It returns the
/// tags of the outdated parameters in the same order when it finds none.
///
/// # Errors
///
/// Returns `E-RCP-020` with every item when at least one problem exists.
pub(in crate::recipe) fn check_snapshot<'a>(
    parameters: impl IntoIterator<Item = &'a Parameter>,
    recipe: Option<&Recipe>,
    units: &LiveUnits,
    trees: &TypeTrees,
) -> Result<Vec<ParameterTag>, RecipeError> {
    let parameters: Vec<&Parameter> = parameters.into_iter().collect();
    if let (Some(recipe), Some(first)) = (recipe, parameters.first())
        && trees.machine() != recipe.machine()
    {
        return Err(RecipeError::ValidationFailed {
            items: vec![SnapshotItem::Version {
                error: PlantError::DerivedTypeInvalid {
                    type_expr: trees.label(first.plc_type()),
                    reason: DerivedTypeReason::NotOnPlatform,
                }
                .into(),
            }],
        });
    }
    let mut items = Vec::new();
    let mut outdated = Vec::new();
    for parameter in parameters {
        check_parameter(parameter, units, trees, &mut items);
        if trees.outdated(parameter.plc_type()).is_some() {
            outdated.push(parameter.tag().clone());
        }
    }
    check_version(recipe, units, &mut items);
    if items.is_empty() {
        Ok(outdated)
    } else {
        Err(RecipeError::ValidationFailed { items })
    }
}

/// Check the machine and the recipe. They give the version items.
fn check_version(recipe: Option<&Recipe>, units: &LiveUnits, items: &mut Vec<SnapshotItem>) {
    if units.is_machine_archived() {
        items.push(SnapshotItem::Version {
            error: PlantError::MachineArchived {
                code: units.machine_code(),
            }
            .into(),
        });
    }
    if let Some(recipe) = recipe.filter(|recipe| recipe.is_deleted()) {
        items.push(SnapshotItem::Version {
            error: RecipeError::RecipeDeleted {
                reference: recipe.reference().clone(),
            }
            .into(),
        });
    }
}

/// Check the unit, the type, the limits, and the value of one parameter.
fn check_parameter(
    parameter: &Parameter,
    units: &LiveUnits,
    trees: &TypeTrees,
    items: &mut Vec<SnapshotItem>,
) {
    let tag = parameter.tag();
    let in_parameter = |error: SnapshotError| SnapshotItem::Parameter {
        tag: tag.clone(),
        error,
    };
    items.extend(unit_item(parameter, units));
    match trees.get(parameter.plc_type()) {
        Some(Ok(tree)) => items.extend(content_errors(parameter, tree).map(in_parameter)),
        Some(Err(PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::NotOnPlatform,
            ..
        }))
        | None => items.push(in_parameter(
            type_not_on_platform_item(parameter, trees).into(),
        )),
        Some(Err(error)) => items.push(in_parameter(error.clone().into())),
    }
}

/// Report an archived or unknown unit of a parameter.
fn unit_item(parameter: &Parameter, units: &LiveUnits) -> Option<SnapshotItem> {
    let Some(path) = units.path_of(parameter.unit()) else {
        return Some(SnapshotItem::UnknownUnit {
            tag: parameter.tag().clone(),
            unit: parameter.unit(),
        });
    };
    (!units.is_live(parameter.unit())).then(|| SnapshotItem::Parameter {
        tag: parameter.tag().clone(),
        error: PlantError::UnitArchived { path: path.clone() }.into(),
    })
}

/// Build `E-RCP-013` with the label of the type.
fn type_not_on_platform_item(parameter: &Parameter, trees: &TypeTrees) -> RecipeError {
    RecipeError::TypeNotOnPlatform {
        tag: parameter.tag().clone(),
        type_name: trees.label(parameter.plc_type()),
        platform: trees.platform(),
    }
}

/// Parse the stored limits and the stored value. Check the value against the limits.
///
/// The limits and the value are independent, so both can give an error. The check against the
/// limits needs both.
fn content_errors(parameter: &Parameter, tree: &TypeTree) -> impl Iterator<Item = SnapshotError> {
    let tag = parameter.tag();
    let limits = stored_limits(tag, parameter.limits(), tree);
    let value = stored_value(tag, parameter.value(), tree);
    let fits = match (&limits, &value) {
        (Ok(limits), Ok(value)) => check_value(tag, value, limits, tree).err(),
        _ => None,
    };
    limits
        .err()
        .into_iter()
        .chain(value.err())
        .chain(fits)
        .map(SnapshotError::from)
}
