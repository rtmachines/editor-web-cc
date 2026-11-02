//! The scope of an export: the whole version or one unit (design D5, ADR-0109 (w)).

use recipe_domain::export::Sha256;
use recipe_domain::plant::UnitPath;

/// The part of a version that an export returns.
///
/// The type solves one problem: the route of a transfer client can ask for the parameters of one
/// unit and its descendants, and the other routes ask for the whole version. One flow serves both,
/// so the input names the scope. [`ExportScope::Version`] is the default.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ExportScope {
    /// The whole version. The document is the export document v1.
    #[default]
    Version,
    /// The unit at the path and every unit below it. The document is the export document v2.
    Unit(UnitPath),
}

/// What the audit entry of a unit export records about the scope (ADR-0109 (ab)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ScopeAudit {
    /// The unit of the request, the value of `scope_unit_path`.
    pub(super) unit_path: UnitPath,
    /// The hash of the scope body, the value of `scope_sha256`.
    pub(super) sha256: Sha256,
}
