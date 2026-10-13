//! Target of an audit entry: the changed row and the root of its aggregate (`data-model.md` section 10).

use uuid::Uuid;

use crate::identity::UserId;
use crate::plant::{MachineId, PlcTypeId, UnitId};
use crate::recipe::{ParameterId, RecipeId, RecipeVersionId};

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
    Recipe,
    RecipeVersion,
    Parameter,
    AppUser,
}

impl EntityType {
    const fn as_sql(self) -> &'static str {
        match self {
            Self::Machine => "machine",
            Self::Unit => "unit",
            Self::PlcType => "plc_type",
            Self::Recipe => "recipe",
            Self::RecipeVersion => "recipe_version",
            Self::Parameter => "parameter",
            Self::AppUser => "app_user",
        }
    }
}

/// The changed row and the root of its aggregate.
///
/// Each constructor takes the identifiers that `data-model.md` section 10 pairs for
/// one entity type. A wrong pair of aggregate and entity therefore cannot be built.
/// A change of a limit row or a translation row uses [`AuditTarget::parameter`].
/// The targets `app_setting` and the global `plc_type` row of a data migration are
/// not built here. No use case of the application layer writes them.
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
mod tests {
    use super::*;

    const CONSTRAINT_VALUES: [&str; 5] =
        ["machine", "recipe", "app_user", "app_setting", "plc_type"];

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn every_aggregate_type_text_is_in_the_constraint_list() {
        for aggregate in AggregateType::ALL {
            assert!(
                CONSTRAINT_VALUES.contains(&aggregate.as_sql()),
                "{aggregate:?}"
            );
        }
        assert_eq!(AggregateType::ALL.len(), 5);
    }

    #[test]
    fn aggregate_type_texts_match_data_model_section_10() {
        assert_eq!(AggregateType::Machine.as_sql(), "machine");
        assert_eq!(AggregateType::Recipe.as_sql(), "recipe");
        assert_eq!(AggregateType::AppUser.as_sql(), "app_user");
        assert_eq!(AggregateType::AppSetting.as_sql(), "app_setting");
        assert_eq!(AggregateType::PlcType.as_sql(), "plc_type");
    }

    #[test]
    fn a_machine_is_its_own_aggregate() {
        let target = AuditTarget::machine(MachineId::from(id(1)));
        assert_eq!(target.entity_type(), "machine");
        assert_eq!(target.aggregate_type(), AggregateType::Machine);
        assert_eq!(target.entity_id(), &id(1));
        assert_eq!(target.aggregate_id(), &id(1));
    }

    #[test]
    fn a_unit_belongs_to_the_machine_aggregate() {
        let target = AuditTarget::unit(MachineId::from(id(1)), UnitId::from(id(2)));
        assert_eq!(target.entity_type(), "unit");
        assert_eq!(target.aggregate_type(), AggregateType::Machine);
        assert_eq!(target.entity_id(), &id(2));
        assert_eq!(target.aggregate_id(), &id(1));
    }

    #[test]
    fn a_type_row_of_the_application_belongs_to_a_machine() {
        let target = AuditTarget::plc_type(MachineId::from(id(1)), PlcTypeId::from(id(3)));
        assert_eq!(target.entity_type(), "plc_type");
        assert_eq!(target.aggregate_type(), AggregateType::Machine);
        assert_eq!(target.entity_id(), &id(3));
        assert_eq!(target.aggregate_id(), &id(1));
    }

    #[test]
    fn a_recipe_is_its_own_aggregate() {
        let target = AuditTarget::recipe(RecipeId::from(id(4)));
        assert_eq!(target.entity_type(), "recipe");
        assert_eq!(target.aggregate_type(), AggregateType::Recipe);
        assert_eq!(target.entity_id(), &id(4));
        assert_eq!(target.aggregate_id(), &id(4));
    }

    #[test]
    fn a_version_belongs_to_the_recipe_aggregate() {
        let target =
            AuditTarget::recipe_version(RecipeId::from(id(4)), RecipeVersionId::from(id(5)));
        assert_eq!(target.entity_type(), "recipe_version");
        assert_eq!(target.aggregate_type(), AggregateType::Recipe);
        assert_eq!(target.entity_id(), &id(5));
        assert_eq!(target.aggregate_id(), &id(4));
    }

    #[test]
    fn a_parameter_belongs_to_the_recipe_aggregate() {
        let target = AuditTarget::parameter(RecipeId::from(id(4)), ParameterId::from(id(6)));
        assert_eq!(target.entity_type(), "parameter");
        assert_eq!(target.aggregate_type(), AggregateType::Recipe);
        assert_eq!(target.entity_id(), &id(6));
        assert_eq!(target.aggregate_id(), &id(4));
    }

    #[test]
    fn a_user_is_its_own_aggregate() {
        let target = AuditTarget::app_user(UserId::from(id(7)));
        assert_eq!(target.entity_type(), "app_user");
        assert_eq!(target.aggregate_type(), AggregateType::AppUser);
        assert_eq!(target.entity_id(), &id(7));
        assert_eq!(target.aggregate_id(), &id(7));
    }

    #[test]
    fn every_entity_type_text_matches_the_format_constraint() {
        let targets = [
            AuditTarget::machine(MachineId::from(id(1))),
            AuditTarget::unit(MachineId::from(id(1)), UnitId::from(id(2))),
            AuditTarget::plc_type(MachineId::from(id(1)), PlcTypeId::from(id(3))),
            AuditTarget::recipe(RecipeId::from(id(4))),
            AuditTarget::recipe_version(RecipeId::from(id(4)), RecipeVersionId::from(id(5))),
            AuditTarget::parameter(RecipeId::from(id(4)), ParameterId::from(id(6))),
            AuditTarget::app_user(UserId::from(id(7))),
        ];
        for target in targets {
            let text = target.entity_type();
            assert!((2..=64).contains(&text.len()), "{text}");
            assert!(
                text.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{text}"
            );
        }
    }
}
