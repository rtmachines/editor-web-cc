//! The hashed body of an export document (`api.md` section 6.1, ADR-0019).

use serde::Serialize;

use super::parameters::ParameterEntry;
use super::types::{TypeEntry, collect_types};
use crate::plant::{LiveUnits, MachineCode, PlcPlatform, TypeNode, TypeTrees, UnitPath};
use crate::recipe::{
    Parameter, ParameterTag, RecipeError, ReferenceNumber, SnapshotItem, not_on_platform,
};

/// The identifying values of the version that the hashed body holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportHeader {
    /// The code of the machine.
    pub machine: MachineCode,
    /// The PLC platform of the machine.
    pub platform: PlcPlatform,
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The number of the version.
    pub version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct MachineBody {
    #[serde(serialize_with = "super::display_text")]
    code: MachineCode,
    #[serde(serialize_with = "super::display_text")]
    plc_platform: PlcPlatform,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct RecipeBody {
    #[serde(serialize_with = "super::display_text")]
    reference_number: ReferenceNumber,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct VersionBody {
    number: u32,
}

/// The JSON object that `content_sha256` covers.
///
/// The type solves one problem: the hash covers the fields marked "In hash" in `api.md`
/// section 6.1 and nothing else. A name, a status, or an export time cannot enter it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HashedBody {
    machine: MachineBody,
    recipe: RecipeBody,
    version: VersionBody,
    types: Vec<TypeEntry>,
    parameters: Vec<ParameterEntry>,
}

/// What the build needs of one parameter that is not corrupt.
struct Resolved<'a> {
    parameter: &'a Parameter,
    unit: &'a UnitPath,
    root: &'a TypeNode,
}

/// Look up the unit and the type tree of a parameter.
///
/// A unit or a type that the inputs do not hold is a corrupt input. The items name the problem.
fn resolve<'a>(
    parameter: &'a Parameter,
    units: &'a LiveUnits,
    trees: &'a TypeTrees,
) -> Result<Resolved<'a>, Vec<SnapshotItem>> {
    let tag: &ParameterTag = parameter.tag();
    let mut items = Vec::new();
    let unit = units.path_of(parameter.unit());
    if unit.is_none() {
        items.push(SnapshotItem::UnknownUnit {
            tag: tag.clone(),
            unit: parameter.unit(),
        });
    }
    let root = match trees.get(parameter.plc_type()) {
        Some(Ok(tree)) => Some(tree.root()),
        Some(Err(error)) => {
            items.push(SnapshotItem::Parameter {
                tag: tag.clone(),
                error: error.clone().into(),
            });
            None
        }
        None => {
            items.push(SnapshotItem::Parameter {
                tag: tag.clone(),
                error: not_on_platform(trees, parameter.plc_type()).into(),
            });
            None
        }
    };
    match (unit, root) {
        (Some(unit), Some(root)) if items.is_empty() => Ok(Resolved {
            parameter,
            unit,
            root,
        }),
        _ => Err(items),
    }
}

impl HashedBody {
    /// Build the hashed body of a version.
    ///
    /// The parameters come from a `Draft` or a `VersionSnapshot`. `units` gives the unit paths,
    /// also of archived units. The arrays follow the orders of `api.md` section 6.3.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-020` when a parameter uses a unit that `units` does not know
    /// (item `E-PLT-020`) or a type that `trees` does not hold (item `E-PLT-013`).
    pub fn build<'a>(
        header: &ExportHeader,
        parameters: impl IntoIterator<Item = &'a Parameter>,
        units: &LiveUnits,
        trees: &TypeTrees,
    ) -> Result<Self, RecipeError> {
        let mut items = Vec::new();
        let mut resolved = Vec::new();
        for parameter in parameters {
            match resolve(parameter, units, trees) {
                Ok(found) => resolved.push(found),
                Err(problems) => items.extend(problems),
            }
        }
        if !items.is_empty() {
            return Err(RecipeError::ValidationFailed { items });
        }
        let types = collect_types(resolved.iter().map(|found| found.root));
        resolved.sort_by(|a, b| {
            a.unit
                .cmp(b.unit)
                .then_with(|| a.parameter.tag().as_str().cmp(b.parameter.tag().as_str()))
        });
        let parameters = resolved
            .iter()
            .map(|found| ParameterEntry::of(found.parameter, found.unit, found.root))
            .collect();
        Ok(Self {
            machine: MachineBody {
                code: header.machine,
                plc_platform: header.platform,
            },
            recipe: RecipeBody {
                reference_number: header.reference.clone(),
            },
            version: VersionBody {
                number: header.version,
            },
            types,
            parameters,
        })
    }

    /// Return the code of the machine, the value of `machine.code`.
    #[must_use]
    pub const fn machine_code(&self) -> MachineCode {
        self.machine.code
    }

    /// Return the PLC platform of the machine, the value of `machine.plc_platform`.
    #[must_use]
    pub const fn platform(&self) -> PlcPlatform {
        self.machine.plc_platform
    }

    /// Return the reference number of the recipe, the value of `recipe.reference_number`.
    #[must_use]
    pub const fn reference_number(&self) -> &ReferenceNumber {
        &self.recipe.reference_number
    }

    /// Return the number of the version, the value of `version.number`.
    #[must_use]
    pub const fn version_number(&self) -> u32 {
        self.version.number
    }

    /// Return the entries of `types[]`, ordered by `id`.
    #[must_use]
    pub fn types(&self) -> &[TypeEntry] {
        &self.types
    }

    /// Return the entries of `parameters[]`, ordered by `unit_path` and then by `tag`.
    #[must_use]
    pub fn parameters(&self) -> &[ParameterEntry] {
        &self.parameters
    }
}

#[cfg(test)]
mod tests;
