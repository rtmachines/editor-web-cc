//! The paths of the recipe, version, and parameter pages (`api.md` section 2).
//!
//! The module solves one problem: the usage page, the parameter page, and the diff sections all
//! link to the same pages. One builder owns each path, so the three places cannot drift apart.

use recipe_domain::recipe::{ParameterKey, ReferenceNumber};

/// Return the path of the recipe page, for example `/recipes/R-100`.
pub(crate) fn recipe_href(reference: &ReferenceNumber) -> String {
    format!("/recipes/{reference}")
}

/// Return the path of the version page, for example `/recipes/R-100/versions/3`.
pub(crate) fn version_href(reference: &ReferenceNumber, number: u32) -> String {
    format!("{}/versions/{number}", recipe_href(reference))
}

/// Return the path of the parameter page, for example `/recipes/R-100/versions/3/parameters/{key}`.
pub(crate) fn parameter_href(
    reference: &ReferenceNumber,
    number: u32,
    key: &ParameterKey,
) -> String {
    format!("{}/parameters/{key}", version_href(reference, number))
}
