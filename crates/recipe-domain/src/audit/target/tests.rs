use super::*;
use crate::setting::SettingId;

const CONSTRAINT_VALUES: [&str; 5] = ["machine", "recipe", "app_user", "app_setting", "plc_type"];

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
fn a_connection_record_belongs_to_the_machine_aggregate() {
    let target = AuditTarget::plc_connection(PlcConnectionId::from(id(9)), MachineId::from(id(1)));
    assert_eq!(target.entity_type(), "plc_connection");
    assert_eq!(target.aggregate_type(), AggregateType::Machine);
    assert_eq!(target.entity_id(), &id(9));
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
    let target = AuditTarget::recipe_version(RecipeId::from(id(4)), RecipeVersionId::from(id(5)));
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
fn a_setting_is_its_own_aggregate() {
    let target = AuditTarget::app_setting(SettingId::from(id(8)));
    assert_eq!(target.entity_type(), "app_setting");
    assert_eq!(target.aggregate_type(), AggregateType::AppSetting);
    assert_eq!(target.entity_id(), &id(8));
    assert_eq!(target.aggregate_id(), &id(8));
}

#[test]
fn every_entity_type_text_matches_the_format_constraint() {
    let targets = [
        AuditTarget::app_setting(SettingId::from(id(8))),
        AuditTarget::machine(MachineId::from(id(1))),
        AuditTarget::unit(MachineId::from(id(1)), UnitId::from(id(2))),
        AuditTarget::plc_type(MachineId::from(id(1)), PlcTypeId::from(id(3))),
        AuditTarget::plc_connection(PlcConnectionId::from(id(9)), MachineId::from(id(1))),
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
