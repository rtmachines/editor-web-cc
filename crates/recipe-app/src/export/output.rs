//! The result of the export use cases.

use recipe_domain::export::{ExportStatus, Sha256};
use recipe_domain::recipe::ReferenceNumber;

/// The export document of one version and the values that the web layer needs (design D6 (f)).
///
/// The type solves one problem: `recipe-web` sends the export document, but it must not rebuild
/// the body or read the database. The port `DocumentWriter` wrote the bytes. The web layer builds
/// the file name from the reference number, the version number, and the status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedVersion {
    /// The bytes of the export document (`api.md` section 6.1).
    pub document: Vec<u8>,
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The number of the version in its recipe.
    pub number: u32,
    /// The status of the version: `Approved` or `Retired`.
    pub status: ExportStatus,
    /// The hash of the hashed body. It equals the stored hash of the version.
    pub content_sha256: Sha256,
    /// `true` when a stored part of the version breaks the current rules (ADR-0119).
    pub has_rule_fault: bool,
    /// The value of `loadable` in the bytes of `document` (ADR-0202 (l)).
    pub(super) loadable: bool,
}

impl ExportedVersion {
    /// Return the value of `loadable` in the bytes of the document (`api.md` section 6).
    ///
    /// The flag is true only for an approved version without a rule fault. A document v1 of a
    /// `Utf8` machine is not loadable (ADR-0202 (l)).
    #[must_use]
    pub const fn loadable(&self) -> bool {
        self.loadable
    }
}
