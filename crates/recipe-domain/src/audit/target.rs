//! Target of an audit entry: the changed row and the root of its aggregate (`data-model.md` section 10).

use uuid::Uuid;

use crate::identity::UserId;
use crate::plant::{MachineId, PlcConnectionId, PlcTypeId, UnitId};
use crate::recipe::{ParameterId, RecipeId, RecipeVersionId};
use crate::setting::SettingId;

/// Root of an aggregate. The column `audit_log.aggregate_type` stores its SQL text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AggregateType {
    /// A machine, with its units and types.
    Machine,
    /// A recipe, with its versions and parameters.
    Recipe,
    /// A user.
    AppUser,
    /// A startup setting.
    AppSetting,
    /// A global primitive type row that a data migration changes.
    PlcType,
}

impl AggregateType {
    /// Every aggregate type, in the order of the constraint `audit_log_aggregate_type_value`.
    pub const ALL: [Self; 5] = [
        Self::Machine,
        Self::Recipe,
        Self::AppUser,
        Self::AppSetting,
        Self::PlcType,
    ];

    /// Return the text that the column `aggregate_type` stores.
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Machine => "machine",
            Self::Recipe => "recipe",
            Self::AppUser => "app_user",
            Self::AppSetting => "app_setting",
            Self::PlcType => "plc_type",
        }
    }
}

/// Table name of the changed row. The column `audit_log.entity_type` stores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum EntityType {
    Machine,
    Unit,
    PlcType,
    PlcConnection,
    Recipe,
    RecipeVersion,
    Parameter,
    AppUser,
    AppSetting,
}

impl EntityType {
    const fn as_sql(self) -> &'static str {
        match self {
            Self::Machine => "machine",
            Self::Unit => "unit",
            Self::PlcType => "plc_type",
            Self::PlcConnection => "plc_connection",
            Self::Recipe => "recipe",
            Self::RecipeVersion => "recipe_version",
            Self::Parameter => "parameter",
            Self::AppUser => "app_user",
            Self::AppSetting => "app_setting",
        }
    }
}

/// The changed row and the root of its aggregate.
///
/// Each constructor takes the identifiers that `data-model.md` section 10 pairs for
/// one entity type. A wrong pair of aggregate and entity therefore cannot be built.
/// A change of a limit row or a translation row uses [`AuditTarget::parameter`].
/// The global `plc_type` row of a data migration is not built here.
/// No use case of the application layer writes it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AuditTarget {
    entity: EntityType,
    entity_id: Uuid,
    aggregate: AggregateType,
    aggregate_id: Uuid,
}

impl AuditTarget {
    fn new(
        entity: EntityType,
        entity_id: Uuid,
        aggregate: AggregateType,
        aggregate_id: Uuid,
    ) -> Self {
        Self {
            entity,
            entity_id,
            aggregate,
            aggregate_id,
        }
    }

    /// Target a machine row. The machine is its own aggregate.
    #[must_use]
    pub fn machine(machine: MachineId) -> Self {
        let id = *machine.as_uuid();
        Self::new(EntityType::Machine, id, AggregateType::Machine, id)
    }

    /// Target a unit row. The aggregate is the machine of the unit.
    #[must_use]
    pub fn unit(machine: MachineId, unit: UnitId) -> Self {
        Self::new(
            EntityType::Unit,
            *unit.as_uuid(),
            AggregateType::Machine,
            *machine.as_uuid(),
        )
    }

    /// Target a type row that the application writes. The aggregate is the machine of the type.
    #[must_use]
    pub fn plc_type(machine: MachineId, plc_type: PlcTypeId) -> Self {
        Self::new(
            EntityType::PlcType,
            *plc_type.as_uuid(),
            AggregateType::Machine,
            *machine.as_uuid(),
        )
    }

    /// Target the connection record of a machine. The aggregate is the machine.
    ///
    /// `entity_id` is the id of the record row. `aggregate_id` is the id of the machine
    /// (`data-model.md` section 10, ADR-0154).
    #[must_use]
    pub fn plc_connection(connection: PlcConnectionId, machine: MachineId) -> Self {
        Self::new(
            EntityType::PlcConnection,
            *connection.as_uuid(),
            AggregateType::Machine,
            *machine.as_uuid(),
        )
    }

    /// Target a recipe row. The recipe is its own aggregate.
    #[must_use]
    pub fn recipe(recipe: RecipeId) -> Self {
        let id = *recipe.as_uuid();
        Self::new(EntityType::Recipe, id, AggregateType::Recipe, id)
    }

    /// Target a version row. The aggregate is the recipe of the version.
    #[must_use]
    pub fn recipe_version(recipe: RecipeId, version: RecipeVersionId) -> Self {
        Self::new(
            EntityType::RecipeVersion,
            *version.as_uuid(),
            AggregateType::Recipe,
            *recipe.as_uuid(),
        )
    }

    /// Target a parameter row, with its limit rows and translation rows.
    /// The aggregate is the recipe of the parameter.
    #[must_use]
    pub fn parameter(recipe: RecipeId, parameter: ParameterId) -> Self {
        Self::new(
            EntityType::Parameter,
            *parameter.as_uuid(),
            AggregateType::Recipe,
            *recipe.as_uuid(),
        )
    }

    /// Target a user row. The user is its own aggregate.
    #[must_use]
    pub fn app_user(user: UserId) -> Self {
        let id = *user.as_uuid();
        Self::new(EntityType::AppUser, id, AggregateType::AppUser, id)
    }

    /// Target a setting row. The setting is its own aggregate.
    #[must_use]
    pub fn app_setting(setting: SettingId) -> Self {
        let id = *setting.as_uuid();
        Self::new(EntityType::AppSetting, id, AggregateType::AppSetting, id)
    }

    /// Return the text of the column `entity_type`, the table name of the changed row.
    #[must_use]
    pub const fn entity_type(&self) -> &'static str {
        self.entity.as_sql()
    }

    /// Return the id of the changed row.
    #[must_use]
    pub const fn entity_id(&self) -> &Uuid {
        &self.entity_id
    }

    /// Return the root of the aggregate.
    #[must_use]
    pub const fn aggregate_type(&self) -> AggregateType {
        self.aggregate
    }

    /// Return the id of the root of the aggregate.
    #[must_use]
    pub const fn aggregate_id(&self) -> &Uuid {
        &self.aggregate_id
    }
}

#[cfg(test)]
mod tests;
