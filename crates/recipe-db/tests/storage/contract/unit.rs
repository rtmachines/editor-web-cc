//! Contract cases of the unit methods of `PlantRepository` (task 9.3).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The cases that
//! need PostgreSQL carry the prefix `postgres_only`.

use recipe_app::error::{StorageError, UNIT_PARENT_EXISTS, UNIT_PATH_UNIQUE};
use recipe_app::ports::RowImage;
use recipe_domain::plant::{MachineId, UnitId, UnitUsage};
use recipe_domain::recipe::ReferenceNumber;
use sqlx::PgConnection;
use uuid::Uuid;

use super::harness::{Harness, begin, commit};
use super::images::{assert_shape, text, unit_shape};
use super::locks::{LOCK_NOT_AVAILABLE, try_row_lock};
use super::postgres_harness::PgHarness;
use super::seed::{
    U1_1, U1_1_1, U1_1_2, U1_2, U1_10, U2, insert, machine, machine_with_tree, name, path, read,
    tree_unit_id,
};
use crate::support::Role;
use crate::support::fixtures::{self, Draft, TWIN_CAT};

/// The identifiers of units as the text of an image.
fn image_ids(images: &[RowImage]) -> Vec<String> {
    images
        .iter()
        .map(|image| text(image, "id").to_owned())
        .collect()
}

async fn insert_unit_gives_the_image_of_the_new_row(h: &dyn Harness) {
    let target = machine(60, "M60");
    insert(h, &target).await;
    let donor = machine_with_tree(61, "M61");
    let child = donor.unit(&path("U1.1")).unwrap().clone();
    let mut tx = begin(h).await;

    let image = tx.plant().insert_unit(target.id(), &child).await.unwrap();
    commit(h, tx, target.id()).await;

    assert_shape(&image, &unit_shape(true, false));
    assert_eq!(text(&image, "path"), "U1.1");
    assert_eq!(text(&image, "parent_path"), "U1");
    assert_eq!(text(&image, "machine_id"), target.id().to_string());
    assert_eq!(text(&image, "id"), child.id().to_string());
    assert_eq!(image.as_value()["depth"], 2);
    let reloaded = read(h, target.id()).await.unwrap();
    assert_eq!(reloaded.unit(&path("U1.1")), Some(&child));
}

async fn insert_unit_with_a_known_path_gives_duplicate(h: &dyn Harness) {
    let target = machine(62, "M62");
    insert(h, &target).await;
    let donor = machine(63, "M63");
    let same_path = donor.unit(&path("U1")).unwrap().clone();
    let mut tx = begin(h).await;

    let error = tx
        .plant()
        .insert_unit(target.id(), &same_path)
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == UNIT_PATH_UNIQUE),
        "{error:?}"
    );
}

async fn insert_unit_with_a_missing_parent_gives_missing_reference(h: &dyn Harness) {
    let target = machine(64, "M64");
    insert(h, &target).await;
    let donor = machine_with_tree(65, "M65");
    let grandchild = donor.unit(&path("U1.1.1")).unwrap().clone();
    let mut tx = begin(h).await;

    let error = tx
        .plant()
        .insert_unit(target.id(), &grandchild)
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::MissingReference { constraint }
            if constraint == UNIT_PARENT_EXISTS),
        "{error:?}"
    );
}

async fn lock_unit_for_update_gives_state_and_before_image(h: &dyn Harness) {
    let built = machine_with_tree(66, "M66");
    let inserted = insert(h, &built).await;
    let mut tx = begin(h).await;

    let locked = tx
        .plant()
        .lock_unit_for_update(built.id(), &path("U1.1"))
        .await
        .unwrap()
        .expect("the unit exists");
    let missing = tx
        .plant()
        .lock_unit_for_update(built.id(), &path("U3"))
        .await
        .unwrap();

    assert_eq!(&locked.state, built.unit(&path("U1.1")).unwrap());
    assert_shape(&locked.before, &unit_shape(true, false));
    assert_eq!(text(&locked.before, "path"), "U1.1");
    let inserted_ids = image_ids(&inserted.units);
    assert!(inserted_ids.contains(&text(&locked.before, "id").to_owned()));
    assert!(missing.is_none());
}

async fn lock_unit_for_update_ignores_the_units_of_other_machines(h: &dyn Harness) {
    let first = machine_with_tree(67, "M67");
    let second = machine(68, "M68");
    insert(h, &first).await;
    insert(h, &second).await;
    let mut tx = begin(h).await;

    let found = tx
        .plant()
        .lock_unit_for_update(second.id(), &path("U1.1"))
        .await
        .unwrap();

    assert!(found.is_none());
}

async fn rename_unit_gives_the_image_after_the_change(h: &dyn Harness) {
    let built = machine_with_tree(69, "M69");
    insert(h, &built).await;
    let id = tree_unit_id(69, U1_2);
    let mut tx = begin(h).await;
    let locked = tx
        .plant()
        .lock_unit_for_update(built.id(), &path("U1.2"))
        .await
        .unwrap()
        .expect("the unit exists");

    let image = tx.plant().rename_unit(id, &name("Renamed")).await.unwrap();
    commit(h, tx, built.id()).await;

    assert_shape(&image, &unit_shape(true, false));
    assert_eq!(text(&image, "name"), "Renamed");
    assert_eq!(text(&image, "path"), "U1.2");
    assert_eq!(text(&locked.before, "name"), "Unit");
    let reloaded = read(h, built.id()).await.unwrap();
    assert_eq!(
        reloaded.unit(&path("U1.2")).unwrap().name().as_str(),
        "Renamed"
    );
    assert_eq!(
        reloaded.unit(&path("U1.1")).unwrap().name().as_str(),
        "Unit"
    );
}

async fn lock_unit_tree_returns_the_unit_and_its_descendants_ordered_by_id(h: &dyn Harness) {
    let built = machine_with_tree(70, "M70");
    let other = machine_with_tree(71, "M71");
    insert(h, &built).await;
    insert(h, &other).await;
    let mut tx = begin(h).await;

    let locked = tx
        .plant()
        .lock_unit_tree_for_update(built.id(), &path("U1.1"))
        .await
        .unwrap();

    // `U1.10` and `U1.2` are outside the tree. The ids do not follow the path order.
    let paths: Vec<String> = locked
        .iter()
        .map(|entry| entry.state.path().to_string())
        .collect();
    assert_eq!(paths, vec!["U1.1.2", "U1.1.1", "U1.1"]);
    let ids: Vec<String> = locked
        .iter()
        .map(|entry| entry.state.id().to_string())
        .collect();
    let expected: Vec<String> = [U1_1_2, U1_1_1, U1_1]
        .into_iter()
        .map(|offset| tree_unit_id(70, offset).to_string())
        .collect();
    assert_eq!(ids, expected);
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
    for entry in &locked {
        assert_eq!(
            text(&entry.before, "id"),
            entry.state.id().to_string(),
            "the before image names the unit"
        );
        assert_eq!(text(&entry.before, "machine_id"), built.id().to_string());
    }
    assert_shape(&locked[2].before, &unit_shape(true, false));
}

async fn lock_unit_tree_of_a_main_unit_holds_every_unit_below_it(h: &dyn Harness) {
    let built = machine_with_tree(72, "M72");
    insert(h, &built).await;
    let mut tx = begin(h).await;

    let locked = tx
        .plant()
        .lock_unit_tree_for_update(built.id(), &path("U1"))
        .await
        .unwrap();

    let mut paths: Vec<String> = locked
        .iter()
        .map(|entry| entry.state.path().to_string())
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        vec!["U1", "U1.1", "U1.1.1", "U1.1.2", "U1.10", "U1.2"]
    );
}

async fn lock_unit_tree_of_a_missing_path_is_empty(h: &dyn Harness) {
    let built = machine(73, "M73");
    insert(h, &built).await;
    let mut tx = begin(h).await;

    let locked = tx
        .plant()
        .lock_unit_tree_for_update(built.id(), &path("U7"))
        .await
        .unwrap();

    assert!(locked.is_empty());
}

async fn archive_units_keeps_the_order_and_the_count_of_the_input(h: &dyn Harness) {
    let built = machine_with_tree(74, "M74");
    insert(h, &built).await;
    // The input order is not the order by id: `U1.1` has the largest id.
    let input: Vec<UnitId> = [U1_1, U1_1_2, U1_1_1]
        .into_iter()
        .map(|offset| tree_unit_id(74, offset))
        .collect();
    let mut tx = begin(h).await;

    let images = tx.plant().archive_units(&input).await.unwrap();
    commit(h, tx, built.id()).await;

    assert_eq!(images.len(), input.len());
    let expected: Vec<String> = input.iter().map(ToString::to_string).collect();
    assert_eq!(image_ids(&images), expected);
    for image in &images {
        assert_shape(image, &unit_shape(true, true));
    }
    let reloaded = read(h, built.id()).await.unwrap();
    for archived in ["U1.1", "U1.1.1", "U1.1.2"] {
        assert!(
            !reloaded.unit(&path(archived)).unwrap().is_live(),
            "{archived}"
        );
    }
    for live in ["U1", "U1.2", "U1.10", "U2"] {
        assert!(reloaded.unit(&path(live)).unwrap().is_live(), "{live}");
    }
}

async fn archive_units_of_an_empty_list_gives_an_empty_list(h: &dyn Harness) {
    let mut tx = begin(h).await;

    let images = tx.plant().archive_units(&[]).await.unwrap();

    assert!(images.is_empty());
}

async fn unit_usage_without_parameters_is_empty(h: &dyn Harness) {
    let built = machine_with_tree(75, "M75");
    insert(h, &built).await;
    let units: Vec<UnitId> = [U1_1, U2]
        .into_iter()
        .map(|offset| tree_unit_id(75, offset))
        .collect();
    let mut tx = begin(h).await;

    let usage = tx.plant().unit_usage(built.id(), &units).await.unwrap();

    assert_eq!(usage, UnitUsage::default());
}

contract_case!(
    insert_unit_gives_the_image_of_the_new_row,
    insert_unit_gives_the_image_of_the_new_row_memory,
    insert_unit_gives_the_image_of_the_new_row_postgres
);
contract_case!(
    insert_unit_with_a_known_path_gives_duplicate,
    insert_unit_with_a_known_path_gives_duplicate_memory,
    insert_unit_with_a_known_path_gives_duplicate_postgres
);
contract_case!(
    insert_unit_with_a_missing_parent_gives_missing_reference,
    insert_unit_with_a_missing_parent_gives_missing_reference_memory,
    insert_unit_with_a_missing_parent_gives_missing_reference_postgres
);
contract_case!(
    lock_unit_for_update_gives_state_and_before_image,
    lock_unit_for_update_gives_state_and_before_image_memory,
    lock_unit_for_update_gives_state_and_before_image_postgres
);
contract_case!(
    lock_unit_for_update_ignores_the_units_of_other_machines,
    lock_unit_for_update_ignores_the_units_of_other_machines_memory,
    lock_unit_for_update_ignores_the_units_of_other_machines_postgres
);
contract_case!(
    rename_unit_gives_the_image_after_the_change,
    rename_unit_gives_the_image_after_the_change_memory,
    rename_unit_gives_the_image_after_the_change_postgres
);
contract_case!(
    lock_unit_tree_returns_the_unit_and_its_descendants_ordered_by_id,
    lock_unit_tree_returns_the_unit_and_its_descendants_ordered_by_id_memory,
    lock_unit_tree_returns_the_unit_and_its_descendants_ordered_by_id_postgres
);
contract_case!(
    lock_unit_tree_of_a_main_unit_holds_every_unit_below_it,
    lock_unit_tree_of_a_main_unit_holds_every_unit_below_it_memory,
    lock_unit_tree_of_a_main_unit_holds_every_unit_below_it_postgres
);
contract_case!(
    lock_unit_tree_of_a_missing_path_is_empty,
    lock_unit_tree_of_a_missing_path_is_empty_memory,
    lock_unit_tree_of_a_missing_path_is_empty_postgres
);
contract_case!(
    archive_units_keeps_the_order_and_the_count_of_the_input,
    archive_units_keeps_the_order_and_the_count_of_the_input_memory,
    archive_units_keeps_the_order_and_the_count_of_the_input_postgres
);
contract_case!(
    archive_units_of_an_empty_list_gives_an_empty_list,
    archive_units_of_an_empty_list_gives_an_empty_list_memory,
    archive_units_of_an_empty_list_gives_an_empty_list_postgres
);
contract_case!(
    unit_usage_without_parameters_is_empty,
    unit_usage_without_parameters_is_empty_memory,
    unit_usage_without_parameters_is_empty_postgres
);

/// Postgres only: `lock_unit_for_update` holds `FOR UPDATE` on the unit row and on no other.
///
/// `FOR UPDATE` blocks every other row lock, also `FOR KEY SHARE`.
#[tokio::test]
async fn postgres_only_lock_unit_for_update_holds_for_update_on_the_unit_row() {
    let h = PgHarness::create().await;
    let built = machine_with_tree(80, "M80");
    insert(&h, &built).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let locked_id = *tree_unit_id(80, U1_2).as_uuid();
    let other_id = *tree_unit_id(80, U1_10).as_uuid();
    let mut tx = begin(&h).await;

    tx.plant()
        .lock_unit_for_update(built.id(), &path("U1.2"))
        .await
        .unwrap()
        .expect("the unit exists");

    for mode in ["KEY SHARE", "SHARE", "NO KEY UPDATE", "UPDATE"] {
        assert_eq!(
            try_row_lock(&mut session, "unit", mode, locked_id)
                .await
                .as_deref(),
            Some(LOCK_NOT_AVAILABLE),
            "{mode}"
        );
    }
    assert_eq!(
        try_row_lock(&mut session, "unit", "UPDATE", other_id).await,
        None
    );
    drop(tx);
    crate::contract::locks::wait_until_released(&mut session, "unit", locked_id).await;
}

/// Postgres only: `lock_unit_tree_for_update` holds `FOR UPDATE` on the tree and on no other unit.
#[tokio::test]
async fn postgres_only_lock_unit_tree_holds_for_update_on_the_tree_only() {
    let h = PgHarness::create().await;
    let built = machine_with_tree(81, "M81");
    let other = machine_with_tree(82, "M82");
    insert(&h, &built).await;
    insert(&h, &other).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let mut tx = begin(&h).await;

    tx.plant()
        .lock_unit_tree_for_update(built.id(), &path("U1.1"))
        .await
        .unwrap();

    for offset in [U1_1, U1_1_1, U1_1_2] {
        let id = *tree_unit_id(81, offset).as_uuid();
        assert_eq!(
            try_row_lock(&mut session, "unit", "KEY SHARE", id)
                .await
                .as_deref(),
            Some(LOCK_NOT_AVAILABLE),
            "offset {offset}"
        );
    }
    for offset in [U1_2, U1_10, U2] {
        let id = *tree_unit_id(81, offset).as_uuid();
        assert_eq!(
            try_row_lock(&mut session, "unit", "UPDATE", id).await,
            None,
            "offset {offset}"
        );
    }
    let sibling_machine = *tree_unit_id(82, U1_1).as_uuid();
    assert_eq!(
        try_row_lock(&mut session, "unit", "UPDATE", sibling_machine).await,
        None
    );
}

/// Postgres only: a lock of a missing unit takes no lock and does not fail.
#[tokio::test]
async fn postgres_only_a_missing_unit_takes_no_lock() {
    let h = PgHarness::create().await;
    let built = machine(83, "M83");
    insert(&h, &built).await;
    let mut tx = begin(&h).await;

    let locked = tx
        .plant()
        .lock_unit_for_update(built.id(), &path("U9"))
        .await
        .unwrap();
    let tree = tx
        .plant()
        .lock_unit_tree_for_update(built.id(), &path("U9"))
        .await
        .unwrap();

    assert!(locked.is_none());
    assert!(tree.is_empty());
}

/// The units and the references of the rows that `usage_fixtures` inserts.
struct UsageFixtures {
    /// The four units `U1` to `U4`.
    all_units: Vec<UnitId>,
    /// `U1`: a live draft and an approved version use it.
    draft_unit: UnitId,
    /// `U2`: only an approved version uses it.
    approved_unit: UnitId,
    /// The reference number of the recipe with the live draft.
    draft_reference: ReferenceNumber,
    /// The reference number of the recipe with the approved version.
    approved_reference: ReferenceNumber,
}

async fn reference_of(conn: &mut PgConnection, recipe: Uuid) -> ReferenceNumber {
    let text: String = sqlx::query_scalar("SELECT reference_number FROM recipe WHERE id = $1")
        .bind(recipe)
        .fetch_one(conn)
        .await
        .unwrap();
    ReferenceNumber::parse(&text).unwrap()
}

/// Insert a recipe with a live draft and one parameter on each of `units`.
async fn recipe_with_parameters(
    conn: &mut PgConnection,
    plant: &Draft,
    tag: &str,
    units: &[Uuid],
) -> (Uuid, Uuid) {
    let recipe = fixtures::recipe(conn, plant.machine, TWIN_CAT, plant.user).await;
    let version = fixtures::draft_version(conn, recipe, TWIN_CAT, 1, plant.user).await;
    for (index, unit) in units.iter().enumerate() {
        let parameter_tag = format!("{tag}{index}");
        fixtures::parameter(conn, version, TWIN_CAT, *unit, plant.dint, &parameter_tag).await;
    }
    (recipe, version)
}

/// Insert four recipes on the plant. Only the first two count for `unit_usage`.
///
/// 1. A live draft with a parameter on `U1`.
/// 2. An approved version with parameters on `U1` and `U2`.
/// 3. A retired version with a parameter on `U3`.
/// 4. A discarded draft with a parameter on `U4`.
async fn usage_fixtures(conn: &mut PgConnection, plant: &Draft) -> UsageFixtures {
    plant.parameter(conn, "T1").await;
    let second = fixtures::unit(conn, plant.machine, "U2").await;
    let third = fixtures::unit(conn, plant.machine, "U3").await;
    let fourth = fixtures::unit(conn, plant.machine, "U4").await;
    let (approved_recipe, approved) =
        recipe_with_parameters(conn, plant, "A", &[plant.unit, second]).await;
    let (_, retired) = recipe_with_parameters(conn, plant, "B", &[third]).await;
    let (_, discarded) = recipe_with_parameters(conn, plant, "C", &[fourth]).await;
    fixtures::approve(conn, approved, plant.user).await;
    fixtures::approve(conn, retired, plant.user).await;
    fixtures::retire(conn, retired).await;
    sqlx::query("UPDATE recipe_version SET deleted_at = now() WHERE id = $1")
        .bind(discarded)
        .execute(&mut *conn)
        .await
        .unwrap();
    UsageFixtures {
        all_units: [plant.unit, second, third, fourth]
            .into_iter()
            .map(UnitId::from)
            .collect(),
        draft_unit: UnitId::from(plant.unit),
        approved_unit: UnitId::from(second),
        draft_reference: reference_of(conn, plant.recipe).await,
        approved_reference: reference_of(conn, approved_recipe).await,
    }
}

/// Postgres only: `unit_usage` finds a live draft and an approved version and nothing else.
///
/// The case seeds the rows with the SQL builders of `fixtures` as `recipe_migrator`, because it
/// needs a retired version and a discarded draft with parameters in one plant, and
/// it is not a cross-adapter case. `MemoryStore` has its own tests of the same rules.
#[tokio::test]
async fn postgres_only_unit_usage_reads_live_drafts_and_approved_versions() {
    let h = PgHarness::create().await;
    let mut conn = h.database.connect(Role::Migrator).await;
    let plant = Draft::create(&mut conn).await;
    let fixtures = usage_fixtures(&mut conn, &plant).await;
    let machine = MachineId::from(plant.machine);
    let mut tx = begin(&h).await;

    let usage = tx
        .plant()
        .unit_usage(machine, &fixtures.all_units)
        .await
        .unwrap();
    let wrong_machine = tx
        .plant()
        .unit_usage(
            MachineId::from(Uuid::from_u128(0xdead)),
            &fixtures.all_units,
        )
        .await
        .unwrap();
    let only_second = tx
        .plant()
        .unit_usage(machine, &[fixtures.approved_unit])
        .await
        .unwrap();

    let mut expected = UnitUsage::default();
    expected.add_draft(fixtures.draft_unit, fixtures.draft_reference.clone());
    expected.add_approved(fixtures.draft_unit, fixtures.approved_reference.clone());
    expected.add_approved(fixtures.approved_unit, fixtures.approved_reference.clone());
    assert_eq!(usage, expected);
    assert_eq!(wrong_machine, UnitUsage::default());
    let mut expected_second = UnitUsage::default();
    expected_second.add_approved(fixtures.approved_unit, fixtures.approved_reference.clone());
    assert_eq!(only_second, expected_second);
}

/// The number of retired versions in the case of the next test.
const MANY_RETIRED_VERSIONS: i32 = 40;

/// Postgres only: many retired versions of a recipe add nothing to `unit_usage`.
///
/// The query starts from the live versions of the machine (review Q5). The retired versions use
/// the unit `U1`, and a live draft of another recipe uses `U2`.
#[tokio::test]
async fn postgres_only_unit_usage_ignores_many_retired_versions() {
    let h = PgHarness::create().await;
    let mut conn = h.database.connect(Role::Migrator).await;
    let plant = Draft::create(&mut conn).await;
    let second = fixtures::unit(&mut conn, plant.machine, "U2").await;
    let history = fixtures::recipe(&mut conn, plant.machine, TWIN_CAT, plant.user).await;
    for number in 1..=MANY_RETIRED_VERSIONS {
        let version =
            fixtures::draft_version(&mut conn, history, TWIN_CAT, number, plant.user).await;
        let tag = format!("R{number}");
        fixtures::parameter(&mut conn, version, TWIN_CAT, plant.unit, plant.dint, &tag).await;
        fixtures::approve(&mut conn, version, plant.user).await;
        fixtures::retire(&mut conn, version).await;
    }
    let (live_recipe, _) = recipe_with_parameters(&mut conn, &plant, "L", &[second]).await;
    let reference = reference_of(&mut conn, live_recipe).await;
    let units = [UnitId::from(plant.unit), UnitId::from(second)];
    let mut tx = begin(&h).await;

    let usage = tx
        .plant()
        .unit_usage(MachineId::from(plant.machine), &units)
        .await
        .unwrap();

    let mut expected = UnitUsage::default();
    expected.add_draft(UnitId::from(second), reference);
    assert_eq!(usage, expected);
}
