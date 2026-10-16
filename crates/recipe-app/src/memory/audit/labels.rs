//! Labels of the audit rows from the current rows of the store (ADR-0051 (e)).

use recipe_domain::identity::UserId;
use recipe_domain::plant::{MachineId, PlcType, PlcTypeId, RecordShape, UnitId};
use recipe_domain::recipe::{ParameterId, RecipeId, RecipeVersionId};

use crate::memory::MemoryState;
use crate::ports::{AuditEntityType, EntityId, EntityLabel};

/// Return the label of the entity `id` of `kind` from its current row.
///
/// A row that no longer exists gives `EntityLabel::Missing`. No label comes from a stored state.
pub(super) fn label(state: &MemoryState, kind: AuditEntityType, id: EntityId) -> EntityLabel {
    let uuid = *id.as_uuid();
    let found = match kind {
        AuditEntityType::Machine => state
            .machines
            .get(&MachineId::from(uuid))
            .map(|row| EntityLabel::Machine(row.code)),
        AuditEntityType::Unit => state
            .units
            .get(&UnitId::from(uuid))
            .map(|row| EntityLabel::Unit(row.path.clone())),
        AuditEntityType::PlcType => state
            .types
            .get(&PlcTypeId::from(uuid))
            .and_then(|row| type_label(&row.plc_type)),
        AuditEntityType::Recipe => state
            .recipes
            .get(&RecipeId::from(uuid))
            .map(|row| EntityLabel::Recipe(row.reference.clone())),
        AuditEntityType::RecipeVersion => state
            .versions
            .get(&RecipeVersionId::from(uuid))
            .map(|row| EntityLabel::Version(row.number)),
        AuditEntityType::Parameter => parameter_label(state, ParameterId::from(uuid)),
        AuditEntityType::AppUser => state
            .users
            .get(&UserId::from(uuid))
            .map(|row| EntityLabel::User(row.login.clone())),
        AuditEntityType::AppSetting => None,
    };
    found.unwrap_or(EntityLabel::Missing)
}

/// Return the tag of the parameter and the number of its version.
fn parameter_label(state: &MemoryState, id: ParameterId) -> Option<EntityLabel> {
    let parameter = state.parameters.get(&id)?;
    let version = state.versions.get(&parameter.version)?;
    Some(EntityLabel::Parameter {
        tag: parameter.tag.clone(),
        version: version.number,
    })
}

/// Return the name and the revision of a struct type. Another type has no name.
fn type_label(plc_type: &PlcType) -> Option<EntityLabel> {
    match plc_type.shape() {
        RecordShape::Struct { name, revision, .. } => Some(EntityLabel::Type {
            name: name.clone(),
            revision: *revision,
        }),
        RecordShape::Primitive { .. } | RecordShape::Array { .. } => None,
    }
}
