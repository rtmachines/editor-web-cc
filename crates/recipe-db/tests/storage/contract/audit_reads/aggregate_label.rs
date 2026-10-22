//! Contract cases of the label of one aggregate (ADR-0051 (a), (d), design D5).
//!
//! The label comes from the current row of the root. A root that does not exist gives `Missing`.

use recipe_app::ports::{AggregateId, EntityLabel};
use recipe_domain::audit::AggregateType;
use recipe_domain::identity::Login;
use recipe_domain::plant::{MachineCode, PlcType, PlcTypeId, TypeName};
use recipe_domain::recipe::ReferenceNumber;
use recipe_domain::text::Text;
use uuid::Uuid;

use super::fixtures::{begin_read, delete_recipe, free_id, second_user};
use crate::contract::PgHarness;
use crate::contract::harness::{Harness, begin, commit};
use crate::contract::recipe_seed::{create_recipe, plant};
use crate::contract::seed::{archive, machine, struct_record, type_id};
use crate::support::Role;

/// Read the label of the aggregate `id` of `kind` in a transaction that nothing commits.
async fn aggregate_label(h: &dyn Harness, kind: AggregateType, id: Uuid) -> EntityLabel {
    let mut tx = begin_read(h).await;
    tx.audit_reads()
        .aggregate_label(kind, AggregateId::from(id))
        .await
        .unwrap()
}

/// Insert the struct type `Valve` revision 2 of the machine 52 and return its id.
async fn seed_valve(h: &dyn Harness, dint: PlcTypeId) -> PlcTypeId {
    let built = machine(52, "M52");
    let record = struct_record(&built, type_id(3), "Valve", 2, None, &[("open", dint)]);
    let mut tx = begin(h).await;
    tx.types()
        .insert_struct(&PlcType::try_from(record).unwrap())
        .await
        .unwrap();
    commit(h, tx, built.id()).await;
    type_id(3)
}

async fn each_aggregate_type_gives_the_label_of_its_current_root(h: &dyn Harness) {
    let plant = plant(h, 52, "M52").await;
    let valve = seed_valve(h, plant.dint).await;
    let recipe = create_recipe(h, &plant, 52, "R-520").await;
    let user = second_user(h, "anowak").await;
    let cases = [
        (
            AggregateType::Machine,
            *plant.machine.as_uuid(),
            EntityLabel::Machine(MachineCode::parse("M52").unwrap()),
        ),
        (
            AggregateType::Recipe,
            *recipe.id().as_uuid(),
            EntityLabel::Recipe(ReferenceNumber::parse("R-520").unwrap()),
        ),
        (
            AggregateType::AppUser,
            *user.as_uuid(),
            EntityLabel::User(Login::parse("anowak").unwrap()),
        ),
        (
            AggregateType::PlcType,
            *valve.as_uuid(),
            EntityLabel::Type {
                name: TypeName::parse("Valve").unwrap(),
                revision: 2,
            },
        ),
        (
            AggregateType::PlcType,
            *plant.dint.as_uuid(),
            EntityLabel::Missing,
        ),
    ];

    for (kind, id, expected) in cases {
        assert_eq!(aggregate_label(h, kind, id).await, expected, "{kind:?}");
    }
}

contract_case!(
    each_aggregate_type_gives_the_label_of_its_current_root,
    memory_each_aggregate_type_gives_the_label_of_its_current_root,
    postgres_each_aggregate_type_gives_the_label_of_its_current_root
);

async fn a_deleted_recipe_keeps_its_reference_number_as_label(h: &dyn Harness) {
    let plant = plant(h, 53, "M53").await;
    let recipe = create_recipe(h, &plant, 53, "R-530").await;
    delete_recipe(h, &recipe).await;

    let label = aggregate_label(h, AggregateType::Recipe, *recipe.id().as_uuid()).await;

    assert_eq!(
        label,
        EntityLabel::Recipe(ReferenceNumber::parse("R-530").unwrap())
    );
}

contract_case!(
    a_deleted_recipe_keeps_its_reference_number_as_label,
    memory_a_deleted_recipe_keeps_its_reference_number_as_label,
    postgres_a_deleted_recipe_keeps_its_reference_number_as_label
);

async fn an_archived_machine_keeps_its_code_as_label(h: &dyn Harness) {
    let plant = plant(h, 54, "M54").await;
    archive(h, plant.machine).await;

    let label = aggregate_label(h, AggregateType::Machine, *plant.machine.as_uuid()).await;

    assert_eq!(
        label,
        EntityLabel::Machine(MachineCode::parse("M54").unwrap())
    );
}

contract_case!(
    an_archived_machine_keeps_its_code_as_label,
    memory_an_archived_machine_keeps_its_code_as_label,
    postgres_an_archived_machine_keeps_its_code_as_label
);

async fn a_root_that_does_not_exist_gives_missing(h: &dyn Harness) {
    for kind in AggregateType::ALL {
        assert_eq!(
            aggregate_label(h, kind, free_id(0x5200)).await,
            EntityLabel::Missing,
            "{kind:?}"
        );
    }
}

contract_case!(
    a_root_that_does_not_exist_gives_missing,
    memory_a_root_that_does_not_exist_gives_missing,
    postgres_a_root_that_does_not_exist_gives_missing
);

/// Insert a setting row as `recipe_app` and return its id.
///
/// The memory adapter has no setting rows. The case therefore runs on PostgreSQL only.
const SEED_SETTING: &str =
    "INSERT INTO app_setting (key, value) VALUES ('test.aggregate_label', 'on') RETURNING id";

#[tokio::test]
async fn postgres_only_a_setting_aggregate_gives_the_key_of_its_row() {
    let h = PgHarness::create().await;
    let mut app = h.database.connect(Role::App).await;
    let id: Uuid = sqlx::query_scalar(SEED_SETTING)
        .fetch_one(&mut app)
        .await
        .unwrap();

    let label = aggregate_label(&h, AggregateType::AppSetting, id).await;

    assert_eq!(
        label,
        EntityLabel::Setting(Text::parse("test.aggregate_label").unwrap())
    );
}
