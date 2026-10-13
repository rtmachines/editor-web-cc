//! The result of the export use cases.

use recipe_domain::export::{HashedBody, Sha256};
use recipe_domain::identity::{Login, UserId};
use recipe_domain::recipe::VersionStatus;
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;

/// The content of an export document (`api.md` section 6.1).
///
/// The type solves one problem: `recipe-web` writes the export document, but it must not
/// rebuild the body or read the database. The type holds the hashed body and every field
/// outside the hash. The fields outside the hash change after a rename or a retirement. The
/// body and the hash do not.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportedVersion {
    /// The body that `content_sha256` covers.
    pub body: HashedBody,
    /// The hash of the body. It equals the stored hash of the version.
    pub content_sha256: Sha256,
    /// The status of the version: `Approved` or `Retired`.
    pub status: VersionStatus,
    /// The name of the machine at the time of the export. It is not in the hash.
    pub machine_name: Text,
    /// The name of the recipe at the time of the export. It is not in the hash.
    pub recipe_name: Text,
    /// The user who approved the version.
    ///
    /// The document holds the `Login` of this user. The user port comes with the identity use
    /// cases in P2.5, so the web layer resolves the login until then. An approved or retired
    /// version always has an approver, so `None` means corrupt rows.
    pub approved_by: Option<UserId>,
    /// The instant of the approval.
    pub approved_at: Option<Timestamp>,
    /// The instant of the export.
    pub exported_at: Timestamp,
    /// The login of the user who exports.
    pub exported_by: Login,
}

impl ExportedVersion {
    /// Return true only when the version is approved at the time of the export (`api.md` section 6).
    #[must_use]
    pub fn loadable(&self) -> bool {
        self.status == VersionStatus::Approved
    }
}
