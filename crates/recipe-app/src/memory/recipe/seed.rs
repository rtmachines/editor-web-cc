//! Seed helpers for the recipe tables. They need no transaction.

use recipe_domain::export::Sha256;
use recipe_domain::identity::UserId;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::{
    Parameter, Recipe, RecipeVersionId, StoredParameter, VersionHeader, VersionState, VersionStatus,
};
use recipe_domain::text::LongText;
use recipe_domain::timestamp::Timestamp;

use super::build::parameter_row;
use super::rows::{RecipeRow, VersionRow};
use crate::memory::MemoryStore;

/// The change note of a seeded version.
const SEED_NOTE: &str = "Seeded version";

/// Build the row of a seeded version header.
///
/// An approved or retired version gets `creator` as approver, the time `now` as approval
/// time, and a hash of 32 zero bytes, so the row satisfies `recipe_version_status_fields`.
fn version_row(
    header: &VersionHeader,
    recipe: &Recipe,
    (platform, creator): (PlcPlatform, UserId),
    (note, now): (LongText, Timestamp),
) -> VersionRow {
    let state = header.state();
    let is_signed = matches!(state, VersionState::Approved | VersionState::Retired);
    VersionRow {
        id: header.id(),
        recipe: recipe.id(),
        platform,
        number: header.number(),
        status: state.status(),
        row_version: header.row_version(),
        based_on: None,
        change_note: note,
        content_sha256: is_signed.then(|| Sha256::from([0_u8; 32])),
        created_by: creator,
        created_at: now,
        updated_at: now,
        approved_by: is_signed.then_some(creator),
        approved_at: is_signed.then_some(now),
        retired_at: (state.status() == VersionStatus::Retired).then_some(now),
        deleted_at: (state == VersionState::Discarded).then_some(now),
    }
}

impl MemoryStore {
    /// Insert a recipe and the rows of its version headers into the committed state.
    ///
    /// The insert checks no constraint. The time of the clock is the time of every column
    /// with a time. An approved or retired version gets `creator` as approver and a hash of
    /// 32 zero bytes. A version has no parameters until `seed_parameter` adds them.
    pub fn seed_recipe(&self, recipe: &Recipe, platform: PlcPlatform, creator: UserId) {
        let Ok(note) = LongText::parse(SEED_NOTE) else {
            return;
        };
        self.update_state(|state, now| {
            let row = RecipeRow {
                id: recipe.id(),
                machine: recipe.machine(),
                platform,
                reference: recipe.reference().clone(),
                name: recipe.name().clone(),
                deleted_at: recipe.deleted_at(),
                created_by: creator,
                created_at: now,
                updated_at: now,
            };
            state.recipes.insert(row.id, row);
            for header in recipe.versions() {
                let version = version_row(header, recipe, (platform, creator), (note.clone(), now));
                state.versions.insert(version.id, version);
            }
        });
    }

    /// Insert a parameter with its limits and translations into the committed state.
    ///
    /// The platform of the parameter is the platform of the seeded version.
    /// A version that is not seeded gives no row.
    /// A limit that breaks a check of `parameter_limit` gives no row.
    pub fn seed_parameter(&self, version: RecipeVersionId, parameter: &StoredParameter) {
        self.update_state(|state, now| {
            let Some(platform) = state.versions.get(&version).map(|row| row.platform) else {
                return;
            };
            let parameter = Parameter::from(parameter.clone());
            if let Ok(row) = parameter_row((version, platform), &parameter, now) {
                state.parameters.insert(row.id, row);
            }
        });
    }

    /// Record that `user` changed the version, as a draft command does.
    pub fn seed_editor(&self, version: RecipeVersionId, user: UserId) {
        self.update_state(|state, now| {
            state.editors.insert((version, user), now);
        });
    }
}
