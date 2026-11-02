//! The list of exportable recipes of one machine (`api.md` section 2.7).
//!
//! The types solve one problem: the use case, the read port, and the list writer of
//! `recipe-export` share one row type, and `recipe-export` must not depend on `recipe-app`
//! (`overview.md` section 5).

use crate::plant::{MachineCode, PlcPlatform};
use crate::recipe::ReferenceNumber;
use crate::text::Text;
use crate::timestamp::Timestamp;

use super::Sha256;

/// One recipe row of the list of exportable recipes.
///
/// The type solves one problem: the transfer client polls the list and needs, for each recipe,
/// the approved version number and the stored hash, but no parameter (`api.md` §2.7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportableRecipe {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The name of the recipe.
    pub name: Text,
    /// The number of the `Approved` version.
    pub version_number: u32,
    /// The instant of the approval of that version.
    pub approved_at: Timestamp,
    /// The hash that approval stored for that version.
    pub content_sha256: Sha256,
}

/// The machine and the recipes with an `Approved` version that `exportable_recipes` reads.
///
/// The type solves one problem: the list route of the transfer client answers with the machine
/// that the code resolved to and its recipes, and the use case must not join them itself
/// (`api.md` §2.7, ADR-0109 (j)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportableRecipes {
    /// The code of the resolved machine.
    pub code: MachineCode,
    /// The PLC platform of the resolved machine.
    pub platform: PlcPlatform,
    /// Whether the resolved machine is archived.
    pub archived: bool,
    /// The recipes in the byte order of the reference number.
    pub recipes: Vec<ExportableRecipe>,
}
