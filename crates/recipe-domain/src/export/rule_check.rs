//! The rule check of the parameters of one version (ADR-0119, design D8).
//!
//! The export of an approved version must tell a reader whether a PLC can load the version. A
//! stored part that breaks only the current rules makes the version not loadable. The function of
//! this module runs the read check of one parameter on every parameter of the version.

use crate::error_codes::ErrorCode;
use crate::plant::TypeTrees;
use crate::recipe::{
    DraftError, Parameter, ParameterKey, RuleFault, StructureFault, check_stored_parameter,
    not_on_platform, overlap_partners,
};

/// The parameters of a version that have a rule fault, with their faults, in parameter order.
pub type VersionRuleFaults = Vec<(ParameterKey, Vec<RuleFault>)>;

/// The key of a parameter and the structure fault of its stored part (ADR-0119 (k)).
///
/// The type solves one problem: the caller must know which parameter is damaged and must give one
/// stable code for the damage. The code is the code of the [`StructureFault`]. The type holds no
/// secret.
///
/// The fault is in a Box, because a structure fault is large and the check returns this type in
/// Err.
#[derive(Debug)]
pub struct BrokenParameter {
    key: ParameterKey,
    fault: Box<StructureFault>,
}

impl BrokenParameter {
    fn new(key: ParameterKey, fault: StructureFault) -> Self {
        Self {
            key,
            fault: Box::new(fault),
        }
    }

    /// Return the key of the damaged parameter.
    #[must_use]
    pub const fn key(&self) -> ParameterKey {
        self.key
    }

    /// Return the structure fault of the stored part.
    #[must_use]
    pub fn fault(&self) -> &StructureFault {
        &self.fault
    }

    /// Return the stable code of the structure fault.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        self.fault.code()
    }

    /// Return the key and the structure fault, and drop the Box.
    #[must_use]
    pub fn into_parts(self) -> (ParameterKey, StructureFault) {
        (self.key, *self.fault)
    }
}

/// Check every parameter of a version against the current rule set (ADR-0119 (j), design D8).
///
/// `trees` holds the type trees of the parameters. They come from the platform of the machine of
/// the version, so the address rules of that platform apply. The check calls
/// [`check_stored_parameter`] once for each parameter.
///
/// The result lists, in the order of `parameters`, each parameter that has at least one rule
/// fault. A parameter without a rule fault does not appear. An empty result means that the
/// version has no rule fault.
///
/// After the loop over the parameters, the check also looks for stored S7 addresses that share
/// bytes (I20, ADR-0149 (e) to (j), design D10). It covers every parameter of the version. Each
/// parameter of an overlapping pair gets a rule fault `E-RCP-022` on its address, after its other
/// faults. The check runs only when no parameter has a structure fault. The hashed body does not
/// change (ADR-0149 (j)).
///
/// A parameter whose type has no usable tree cannot be checked. The check treats it as a
/// structure fault, so the version page and the export share one rule (ADR-0118 (d)).
///
/// # Errors
///
/// Returns the key and the [`StructureFault`] of the first parameter with a structure fault. A
/// parameter without a tree is one. A structure fault is a storage defect, and the caller gives
/// `500 E-DB-008` (ADR-0119 (k)). An overlap is never a structure fault (ADR-0149 (h)).
pub fn check_version_parameters(
    parameters: &[Parameter],
    trees: &TypeTrees,
) -> Result<VersionRuleFaults, BrokenParameter> {
    let mut checked = Vec::new();
    for parameter in parameters {
        let broken = |fault: StructureFault| BrokenParameter::new(parameter.key(), fault);
        let tree = match trees.get(parameter.plc_type()) {
            Some(Ok(tree)) => tree,
            Some(Err(error)) => return Err(broken(DraftError::from(error.clone()).into())),
            None => {
                let error = not_on_platform(trees, parameter.plc_type());
                return Err(broken(DraftError::from(error).into()));
            }
        };
        let faults = check_stored_parameter(parameter, tree).map_err(broken)?;
        checked.push((parameter, tree, faults));
    }
    let partners = overlap_partners(
        checked
            .iter()
            .map(|(parameter, tree, _)| (*parameter, *tree)),
    )
    .map_err(|(key, error)| BrokenParameter::new(key, DraftError::from(*error).into()))?;
    let mut found = Vec::new();
    for (parameter, _, mut faults) in checked {
        let overlap = partners.get(&parameter.key()).zip(parameter.s7_address());
        if let Some((other_tag, address)) = overlap {
            faults.push(RuleFault::overlap(
                parameter.tag().clone(),
                other_tag.clone(),
                address,
            ));
        }
        if !faults.is_empty() {
            found.push((parameter.key(), faults));
        }
    }
    Ok(found)
}
#[cfg(test)]
mod tests;
