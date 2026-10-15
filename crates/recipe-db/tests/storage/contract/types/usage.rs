//! Contract cases of `TypeCatalog::type_usage` (design D4, spec "Type usage").
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The walk follows
//! the member links and the element links of the type rows. It never follows `supersedes`.

use recipe_app::error::StorageError;
use recipe_app::ports::TypeUsageRow;
use recipe_domain::plant::{Machine, MachineId, PlcType, PlcTypeId, Primitive};
use recipe_domain::recipe::{
    ParameterTag, RecipeVersionId, ReferenceNumber, StoredParameter, VersionStatus,
};

use crate::contract::harness::{Harness, begin, commit};
use crate::contract::recipe_seed::{
    Plant, approve, create_recipe, create_version, parameter, recipe_id, retire, seed, version_id,
};
use crate::contract::seed::{
    array_record, ensure, insert, machine, machine_id, seed_type, struct_record, type_id,
};

/// The page that holds every row of a case.
const WHOLE: (u64, u32) = (0, 100);

/// A committed machine with its first unit and the seed type `DINT`.
struct World {
    machine: Machine,
    plant: Plant,
}

async fn world(h: &dyn Harness, n: u128) -> World {
    let built = machine(n, &format!("M{n}"));
    insert(h, &built).await;
    let unit = built.units().next().unwrap().id();
    let dint = seed_type(h, built.id(), Primitive::Dint).await;
    let plant = Plant {
        machine: built.id(),
        unit,
        dint,
    };
    World {
        machine: built,
        plant,
    }
}

/// Commit the struct `name` in the revision `revision` and return its row identifier.
///
/// `members` lists `(name, type)` in order.
async fn define(
    h: &dyn Harness,
    world: &World,
    id: u128,
    (name, revision): (&str, u32),
    supersedes: Option<PlcTypeId>,
    members: &[(&str, PlcTypeId)],
) -> PlcTypeId {
    let record = struct_record(
        &world.machine,
        type_id(id),
        name,
        revision,
        supersedes,
        members,
    );
    let row = PlcType::try_from(record).unwrap();
    let mut tx = begin(h).await;
    tx.types().insert_struct(&row).await.unwrap();
    commit(h, tx, world.machine.id()).await;
    row.id()
}

/// Commit the array type `ARRAY[1..4] OF element` and return its row identifier.
async fn array_of(h: &dyn Harness, world: &World, id: u128, element: PlcTypeId) -> PlcTypeId {
    let record = array_record(&world.machine, type_id(id), element, &[(1, 4)]);
    ensure(h, world.machine.id(), record).await.id
}

/// The two revisions of `ST_Zone`. The second one supersedes the first one.
async fn zones(h: &dyn Harness, world: &World) -> (PlcTypeId, PlcTypeId) {
    let dint = world.plant.dint;
    let first = define(h, world, 10, ("ST_Zone", 1), None, &[("temp", dint)]).await;
    let second = define(h, world, 11, ("ST_Zone", 2), Some(first), &[("temp", dint)]).await;
    (first, second)
}

/// A parameter of the type `plc_type`. The number `n` sets its identifiers.
fn typed(world: &World, n: u128, tag: &str, plc_type: PlcTypeId) -> StoredParameter {
    StoredParameter {
        plc_type,
        ..parameter(&world.plant, n, tag)
    }
}

/// Seed the recipe number `n`. Its first version, a live draft, holds the parameter `tag`.
async fn recipe_with(
    h: &dyn Harness,
    world: &World,
    n: u128,
    (reference, tag): (&str, &str),
    plc_type: PlcTypeId,
) -> RecipeVersionId {
    let created = create_recipe(h, &world.plant, n, reference).await;
    let version = created.versions()[0].id();
    seed(h, version, &typed(world, n, tag, plc_type)).await;
    version
}

/// Read one page of the use of `type_id` in a transaction that nothing commits.
async fn usage(
    h: &dyn Harness,
    machine: MachineId,
    type_id: PlcTypeId,
    (offset, limit): (u64, u32),
) -> Vec<TypeUsageRow> {
    let mut tx = begin(h).await;
    tx.types()
        .type_usage(machine, type_id, offset, limit)
        .await
        .unwrap()
}

/// The row that a case expects.
fn row(
    (reference, number, status): (&str, u32, VersionStatus),
    tag: &str,
    parameter_type: PlcTypeId,
) -> TypeUsageRow {
    TypeUsageRow {
        reference: ReferenceNumber::parse(reference).unwrap(),
        number,
        status,
        tag: ParameterTag::parse(tag).unwrap(),
        parameter_type,
    }
}

async fn a_member_and_the_type_itself_are_uses(h: &dyn Harness) {
    let w = world(h, 1).await;
    let (_, zone) = zones(h, &w).await;
    let holder = define(h, &w, 12, ("ST_Holder", 1), None, &[("zone", zone)]).await;
    let version = recipe_with(h, &w, 100, ("R-100", "HOLDER"), holder).await;
    seed(h, version, &typed(&w, 101, "ZONE", zone)).await;
    let draft = ("R-100", 1, VersionStatus::Draft);

    let of_zone = usage(h, w.machine.id(), zone, WHOLE).await;
    let of_holder = usage(h, w.machine.id(), holder, WHOLE).await;
    let of_another_machine = usage(h, machine_id(999), zone, WHOLE).await;

    assert_eq!(
        of_zone,
        [row(draft, "HOLDER", holder), row(draft, "ZONE", zone)]
    );
    assert_eq!(of_holder, [row(draft, "HOLDER", holder)]);
    assert!(of_another_machine.is_empty(), "{of_another_machine:?}");
}

async fn an_array_element_is_a_use_and_an_unused_revision_has_none(h: &dyn Harness) {
    let w = world(h, 2).await;
    let (first, second) = zones(h, &w).await;
    let zones_array = array_of(h, &w, 20, second).await;
    let line = define(h, &w, 13, ("ST_Line", 1), None, &[("zones", zones_array)]).await;
    let version = recipe_with(h, &w, 100, ("R-100", "LINE"), line).await;
    seed(h, version, &typed(&w, 101, "ZONES", zones_array)).await;
    let draft = ("R-100", 1, VersionStatus::Draft);

    let used = usage(h, w.machine.id(), second, WHOLE).await;
    let unused = usage(h, w.machine.id(), first, WHOLE).await;

    assert_eq!(
        used,
        [row(draft, "LINE", line), row(draft, "ZONES", zones_array)]
    );
    assert!(unused.is_empty(), "{unused:?}");
}

async fn the_use_goes_through_two_levels(h: &dyn Harness) {
    let w = world(h, 3).await;
    let (_, zone) = zones(h, &w).await;
    let zones_array = array_of(h, &w, 20, zone).await;
    let line = define(h, &w, 13, ("ST_Line", 1), None, &[("zones", zones_array)]).await;
    let site = define(h, &w, 14, ("ST_Site", 1), None, &[("line", line)]).await;
    recipe_with(h, &w, 100, ("R-100", "SITE"), site).await;
    let draft = ("R-100", 1, VersionStatus::Draft);

    let of_zone = usage(h, w.machine.id(), zone, WHOLE).await;
    let of_line = usage(h, w.machine.id(), line, WHOLE).await;

    assert_eq!(of_zone, [row(draft, "SITE", site)]);
    assert_eq!(of_line, [row(draft, "SITE", site)]);
}

async fn a_parameter_that_holds_the_revision_twice_gives_one_row(h: &dyn Harness) {
    let w = world(h, 4).await;
    let (_, zone) = zones(h, &w).await;
    let zones_array = array_of(h, &w, 20, zone).await;
    let members = [("a", zone), ("b", zone)];
    let pair = define(h, &w, 15, ("ST_Pair", 1), None, &members).await;
    let mixed = [("a", zone), ("b", zones_array)];
    let diamond = define(h, &w, 16, ("ST_Diamond", 1), None, &mixed).await;
    let version = recipe_with(h, &w, 400, ("R-400", "PAIR"), pair).await;
    seed(h, version, &typed(&w, 401, "DIAMOND", diamond)).await;
    let draft = ("R-400", 1, VersionStatus::Draft);

    let rows = usage(h, w.machine.id(), zone, WHOLE).await;

    assert_eq!(
        rows,
        [row(draft, "DIAMOND", diamond), row(draft, "PAIR", pair)]
    );
}

async fn a_deleted_recipe_and_a_discarded_draft_give_no_row(h: &dyn Harness) {
    let w = world(h, 5).await;
    let (_, zone) = zones(h, &w).await;
    let deleted = recipe_with(h, &w, 200, ("R-200", "OLD"), zone).await;
    approve(h, deleted).await;
    retire(h, deleted).await;
    let mut tx = begin(h).await;
    tx.recipes()
        .lock_recipe(recipe_id(200))
        .await
        .unwrap()
        .unwrap();
    tx.recipes()
        .mark_recipe_deleted(recipe_id(200))
        .await
        .unwrap();
    commit(h, tx, w.machine.id()).await;
    let discarded = recipe_with(h, &w, 300, ("R-300", "GONE"), zone).await;
    let mut tx = begin(h).await;
    tx.recipes().lock_version(discarded).await.unwrap().unwrap();
    tx.recipes().discard_version(discarded).await.unwrap();
    commit(h, tx, w.machine.id()).await;
    recipe_with(h, &w, 400, ("R-400", "LIVE"), zone).await;

    let rows = usage(h, w.machine.id(), zone, WHOLE).await;

    assert_eq!(
        rows,
        [row(("R-400", 1, VersionStatus::Draft), "LIVE", zone)]
    );
}

async fn the_three_states_are_listed_with_the_newest_version_first(h: &dyn Harness) {
    let w = world(h, 6).await;
    let (_, zone) = zones(h, &w).await;
    let first = recipe_with(h, &w, 500, ("R-500", "KEEP"), zone).await;
    approve(h, first).await;
    let ids = |version: u128| (recipe_id(500), version_id(version));
    create_version(h, ids(501), 2, Some(first)).await;
    retire(h, first).await;
    approve(h, version_id(501)).await;
    create_version(h, ids(502), 3, Some(version_id(501))).await;

    let rows = usage(h, w.machine.id(), zone, WHOLE).await;

    assert_eq!(
        rows,
        [
            row(("R-500", 3, VersionStatus::Draft), "KEEP", zone),
            row(("R-500", 2, VersionStatus::Approved), "KEEP", zone),
            row(("R-500", 1, VersionStatus::Retired), "KEEP", zone),
        ]
    );
}

async fn rows_come_in_the_byte_order_of_the_reference_and_the_tag(h: &dyn Harness) {
    let w = world(h, 7).await;
    let (_, zone) = zones(h, &w).await;
    recipe_with(h, &w, 601, ("R-9", "B"), zone).await;
    let ten = recipe_with(h, &w, 602, ("R-10", "B"), zone).await;
    seed(h, ten, &typed(&w, 603, "A_1", zone)).await;
    seed(h, ten, &typed(&w, 604, "A1", zone)).await;
    recipe_with(h, &w, 605, ("R_1", "A"), zone).await;
    let draft = |reference| (reference, 1, VersionStatus::Draft);

    let rows = usage(h, w.machine.id(), zone, WHOLE).await;

    assert_eq!(
        rows,
        [
            row(draft("R-10"), "A1", zone),
            row(draft("R-10"), "A_1", zone),
            row(draft("R-10"), "B", zone),
            row(draft("R-9"), "B", zone),
            row(draft("R_1"), "A", zone),
        ]
    );
}

async fn offset_and_limit_cut_the_page(h: &dyn Harness) {
    let w = world(h, 8).await;
    let (_, zone) = zones(h, &w).await;
    let version = recipe_with(h, &w, 700, ("R-700", "T1"), zone).await;
    for (n, tag) in [(701, "T2"), (702, "T3"), (703, "T4"), (704, "T5")] {
        seed(h, version, &typed(&w, n, tag, zone)).await;
    }
    let tags = |rows: &[TypeUsageRow]| -> Vec<String> {
        rows.iter().map(|row| row.tag.to_string()).collect()
    };

    let first = usage(h, w.machine.id(), zone, (0, 3)).await;
    let middle = usage(h, w.machine.id(), zone, (1, 2)).await;
    let tail = usage(h, w.machine.id(), zone, (4, 10)).await;
    let beyond = usage(h, w.machine.id(), zone, (5, 10)).await;
    let none = usage(h, w.machine.id(), zone, (0, 0)).await;

    assert_eq!(tags(&first), ["T1", "T2", "T3"]);
    assert_eq!(tags(&middle), ["T2", "T3"]);
    assert_eq!(tags(&tail), ["T5"]);
    assert!(beyond.is_empty(), "{beyond:?}");
    assert!(none.is_empty(), "{none:?}");
}

async fn an_offset_above_bigint_is_a_storage_error_not_an_empty_list(h: &dyn Harness) {
    let w = world(h, 9).await;
    let (_, zone) = zones(h, &w).await;
    let largest = u64::try_from(i64::MAX).unwrap();
    let mut tx = begin(h).await;

    let at_limit = tx
        .types()
        .type_usage(w.machine.id(), zone, largest, 10)
        .await;
    let beyond = tx
        .types()
        .type_usage(w.machine.id(), zone, largest.checked_add(1).unwrap(), 10)
        .await;

    assert!(at_limit.unwrap().is_empty());
    assert!(matches!(
        beyond.unwrap_err(),
        StorageError::Other { sqlstate: None, .. }
    ));
}

contract_case!(
    an_offset_above_bigint_is_a_storage_error_not_an_empty_list,
    an_offset_above_bigint_is_a_storage_error_not_an_empty_list_memory,
    an_offset_above_bigint_is_a_storage_error_not_an_empty_list_postgres
);
contract_case!(
    a_member_and_the_type_itself_are_uses,
    a_member_and_the_type_itself_are_uses_memory,
    a_member_and_the_type_itself_are_uses_postgres
);
contract_case!(
    an_array_element_is_a_use_and_an_unused_revision_has_none,
    an_array_element_is_a_use_and_an_unused_revision_has_none_memory,
    an_array_element_is_a_use_and_an_unused_revision_has_none_postgres
);
contract_case!(
    the_use_goes_through_two_levels,
    the_use_goes_through_two_levels_memory,
    the_use_goes_through_two_levels_postgres
);
contract_case!(
    a_parameter_that_holds_the_revision_twice_gives_one_row,
    a_parameter_that_holds_the_revision_twice_gives_one_row_memory,
    a_parameter_that_holds_the_revision_twice_gives_one_row_postgres
);
contract_case!(
    a_deleted_recipe_and_a_discarded_draft_give_no_row,
    a_deleted_recipe_and_a_discarded_draft_give_no_row_memory,
    a_deleted_recipe_and_a_discarded_draft_give_no_row_postgres
);
contract_case!(
    the_three_states_are_listed_with_the_newest_version_first,
    the_three_states_are_listed_with_the_newest_version_first_memory,
    the_three_states_are_listed_with_the_newest_version_first_postgres
);
contract_case!(
    rows_come_in_the_byte_order_of_the_reference_and_the_tag,
    rows_come_in_the_byte_order_of_the_reference_and_the_tag_memory,
    rows_come_in_the_byte_order_of_the_reference_and_the_tag_postgres
);
contract_case!(
    offset_and_limit_cut_the_page,
    offset_and_limit_cut_the_page_memory,
    offset_and_limit_cut_the_page_postgres
);
