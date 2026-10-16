//! Contract cases of the labels of the audit rows (task 5.1, ADR-0051 (e)).

use recipe_app::ports::{AuditEntityType, AuditFilter, AuditScope, EntityFilter, EntityLabel};
use recipe_domain::audit::{AuditTarget, RowAction};
use recipe_domain::identity::{Login, UserId};
use recipe_domain::plant::{
    MachineCode, MachineId, PlcType, PlcTypeId, StringKind, TypeName, UnitId, UnitPath,
};
use recipe_domain::recipe::{
    ParameterId, ParameterTag, RecipeId, RecipeVersionId, ReferenceNumber,
};

use recipe_app::identity::new_request_id;
use recipe_domain::text::Text;

use super::fixtures::{WHOLE, entry, free_id, list, second_user, write};
use crate::contract::PgHarness;
use crate::contract::harness::{Harness, begin, commit};
use crate::contract::recipe_seed::{create_recipe, parameter, plant, seed, version_id};
use crate::contract::seed::{array_record, ensure, machine, string_record, struct_record, type_id};
use crate::support::Role;

/// Read the label of the newest entry of `kind`.
async fn newest_label(h: &dyn Harness, kind: AuditEntityType) -> EntityLabel {
    let filter = AuditFilter {
        entity: Some(EntityFilter {
            entity_type: kind,
            entity_id: None,
        }),
        ..AuditFilter::default()
    };
    let page = list(h, &filter, AuditScope::All, None, WHOLE).await;
    page.rows[0].label.clone()
}

/// Insert the struct type `Pump` revision 1 of the machine 50 and return its id.
async fn seed_struct(h: &dyn Harness, dint: PlcTypeId) -> PlcTypeId {
    let built = machine(50, "M50");
    let record = struct_record(&built, type_id(1), "Pump", 1, None, &[("speed", dint)]);
    let mut tx = begin(h).await;
    tx.types()
        .insert_struct(&PlcType::try_from(record).unwrap())
        .await
        .unwrap();
    commit(h, tx, built.id()).await;
    type_id(1)
}

async fn each_label_comes_from_the_current_row(h: &dyn Harness) {
    let plant = plant(h, 50, "M50").await;
    let pump = seed_struct(h, plant.dint).await;
    let recipe = create_recipe(h, &plant, 50, "R-500").await;
    let speed = parameter(&plant, 50, "SPEED");
    seed(h, version_id(50), &speed).await;
    let user = second_user(h, "jsmith").await;
    let (machine, recipe_id) = (plant.machine, recipe.id());
    let targets = [
        (RowAction::MachineRenamed, AuditTarget::machine(machine)),
        (
            RowAction::UnitRenamed,
            AuditTarget::unit(machine, plant.unit),
        ),
        (RowAction::UdtDefined, AuditTarget::plc_type(machine, pump)),
        (RowAction::RecipeRenamed, AuditTarget::recipe(recipe_id)),
        (
            RowAction::VersionCreated,
            AuditTarget::recipe_version(recipe_id, version_id(50)),
        ),
        (
            RowAction::ParameterAdded,
            AuditTarget::parameter(recipe_id, speed.id),
        ),
        (RowAction::UserRoleSet, AuditTarget::app_user(user)),
    ];
    write(h, entries(targets, Some(h.user()))).await;

    for (kind, label) in expected_labels() {
        assert_eq!(newest_label(h, kind).await, label, "{kind:?}");
    }
}

/// The label of each entity type that `each_label_comes_from_the_current_row` seeds.
fn expected_labels() -> [(AuditEntityType, EntityLabel); 7] {
    [
        (
            AuditEntityType::Machine,
            EntityLabel::Machine(MachineCode::parse("M50").unwrap()),
        ),
        (
            AuditEntityType::Unit,
            EntityLabel::Unit(UnitPath::parse("U1").unwrap()),
        ),
        (
            AuditEntityType::PlcType,
            EntityLabel::Type {
                name: TypeName::parse("Pump").unwrap(),
                revision: 1,
            },
        ),
        (
            AuditEntityType::Recipe,
            EntityLabel::Recipe(ReferenceNumber::parse("R-500").unwrap()),
        ),
        (AuditEntityType::RecipeVersion, EntityLabel::Version(1)),
        (
            AuditEntityType::Parameter,
            EntityLabel::Parameter {
                tag: ParameterTag::parse("SPEED").unwrap(),
                version: 1,
            },
        ),
        (
            AuditEntityType::AppUser,
            EntityLabel::User(Login::parse("jsmith").unwrap()),
        ),
    ]
}

contract_case!(
    each_label_comes_from_the_current_row,
    memory_each_label_comes_from_the_current_row,
    postgres_each_label_comes_from_the_current_row
);

async fn a_row_that_no_longer_exists_and_a_type_without_a_name_give_missing(h: &dyn Harness) {
    let plant = plant(h, 51, "M51").await;
    let built = machine(51, "M51");
    let array = ensure(
        h,
        built.id(),
        array_record(&built, type_id(2), plant.dint, &[(1, 3)]),
    )
    .await;
    let string = ensure(
        h,
        built.id(),
        string_record(&built, type_id(4), StringKind::String, 20),
    )
    .await;
    let targets = gone_targets(plant.machine, array.id, string.id);
    write(h, entries(targets, None)).await;

    for kind in AuditEntityType::ALL
        .into_iter()
        .filter(|kind| *kind != AuditEntityType::AppSetting)
    {
        assert_eq!(
            newest_label(h, kind).await,
            EntityLabel::Missing,
            "{kind:?}"
        );
    }
    let rows = list(h, &AuditFilter::default(), AuditScope::All, None, WHOLE)
        .await
        .rows;
    let type_labels: Vec<&EntityLabel> = rows
        .iter()
        .filter(|row| row.entity_type == AuditEntityType::PlcType)
        .map(|row| &row.label)
        .collect();
    assert_eq!(type_labels, [&EntityLabel::Missing, &EntityLabel::Missing]);
    let machine_row = rows
        .iter()
        .find(|row| row.entity_type == AuditEntityType::Machine)
        .unwrap();
    let full = gone(1).simple().to_string();
    assert_eq!(machine_row.entity_id.short().as_str(), &full[..8]);
}

/// Return the id number `n` of a row that does not exist.
fn gone(n: u128) -> uuid::Uuid {
    free_id(0x5100 | n)
}

/// The targets of rows that do not exist, and of the types `array` and `string` of `machine`.
fn gone_targets(
    machine: MachineId,
    array: PlcTypeId,
    string: PlcTypeId,
) -> [(RowAction, AuditTarget); 8] {
    let (recipe, machine_gone) = (RecipeId::from(gone(4)), MachineId::from(gone(1)));
    [
        (
            RowAction::MachineRenamed,
            AuditTarget::machine(machine_gone),
        ),
        (
            RowAction::UnitRenamed,
            AuditTarget::unit(machine_gone, UnitId::from(gone(2))),
        ),
        (
            RowAction::DerivedTypeCreated,
            AuditTarget::plc_type(machine, array),
        ),
        (
            RowAction::DerivedTypeCreated,
            AuditTarget::plc_type(machine, string),
        ),
        (RowAction::RecipeRenamed, AuditTarget::recipe(recipe)),
        (
            RowAction::VersionCreated,
            AuditTarget::recipe_version(recipe, RecipeVersionId::from(gone(5))),
        ),
        (
            RowAction::ParameterRemoved,
            AuditTarget::parameter(recipe, ParameterId::from(gone(6))),
        ),
        (
            RowAction::UserRoleSet,
            AuditTarget::app_user(UserId::from(gone(7))),
        ),
    ]
}

/// Build one entry of `actor` for each pair of action and target, in order.
fn entries<const N: usize>(
    targets: [(RowAction, AuditTarget); N],
    actor: Option<UserId>,
) -> Vec<recipe_domain::audit::AuditEntry> {
    targets
        .into_iter()
        .zip(1_u64..)
        .map(|((action, target), n)| entry(action, target, actor, n))
        .collect()
}

/// Insert a setting row and its `SettingChanged` entry as `recipe_app`.
///
/// No port writes a setting entry, and the memory adapter has no setting rows. The case therefore
/// runs on PostgreSQL only.
const SEED_SETTING_ENTRY: &str = "\
WITH s AS (INSERT INTO app_setting (key, value) VALUES ('test.audit_label', 'on') RETURNING id) \
INSERT INTO audit_log (action, entity_type, entity_id, aggregate_type, aggregate_id, \
    after_state, request_id) \
SELECT 'SettingChanged', 'app_setting', s.id, 'app_setting', s.id, '{}'::jsonb, $1 FROM s";

#[tokio::test]
async fn postgres_only_a_setting_entry_shows_the_key_of_its_row() {
    let h = PgHarness::create().await;
    let mut app = h.database.connect(Role::App).await;
    sqlx::query(SEED_SETTING_ENTRY)
        .bind(*new_request_id().unwrap().as_uuid())
        .execute(&mut app)
        .await
        .unwrap();

    let label = newest_label(&h, AuditEntityType::AppSetting).await;

    assert_eq!(
        label,
        EntityLabel::Setting(Text::parse("test.audit_label").unwrap())
    );
}

contract_case!(
    a_row_that_no_longer_exists_and_a_type_without_a_name_give_missing,
    memory_a_row_that_no_longer_exists_and_a_type_without_a_name_give_missing,
    postgres_a_row_that_no_longer_exists_and_a_type_without_a_name_give_missing
);
