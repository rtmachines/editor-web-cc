//! Contract cases of `TypeCatalog::type_rows`.

use std::collections::BTreeSet;

use recipe_domain::plant::{PlcPlatform, PlcType, PlcTypeId, Primitive, RecordShape, StringKind};

use crate::contract::harness::{Harness, begin, commit};
use crate::contract::seed::{
    ensure, insert, machine, seed_type, string_record, struct_record, type_id,
};

/// The primitives that both adapters seed without a length.
const SHARED_PRIMITIVES: [Primitive; 7] = [
    Primitive::Bool,
    Primitive::Int,
    Primitive::Dint,
    Primitive::Real,
    Primitive::Lreal,
    Primitive::Time,
    Primitive::Ltime,
];

fn has_primitive(rows: &[PlcType], wanted: Primitive, length: Option<u16>) -> bool {
    rows.iter().any(|row| {
        row.machine().is_none()
            && matches!(row.shape(),
                RecordShape::Primitive { primitive, string_length, .. }
                    if *primitive == wanted && *string_length == length)
    })
}

async fn type_rows_gives_the_seed_rows_and_the_rows_of_the_machine_only(h: &dyn Harness) {
    let first = machine(100, "M100");
    let second = machine(101, "M101");
    insert(h, &first).await;
    insert(h, &second).await;
    let dint = seed_type(h, first.id(), Primitive::Dint).await;
    let boolean = seed_type(h, first.id(), Primitive::Bool).await;
    let own_string = string_record(&first, type_id(1), StringKind::String, 20);
    let foreign_string = string_record(&second, type_id(2), StringKind::String, 20);
    let own_struct = struct_record(
        &first,
        type_id(3),
        "Pump",
        1,
        None,
        &[("speed", dint), ("running", boolean)],
    );
    ensure(h, first.id(), own_string.clone()).await;
    ensure(h, second.id(), foreign_string).await;
    let mut tx = begin(h).await;
    let struct_row = PlcType::try_from(own_struct.clone()).unwrap();
    tx.types().insert_struct(&struct_row).await.unwrap();
    commit(h, tx, first.id()).await;
    let mut tx = begin(h).await;

    let rows = tx.types().type_rows(first.id()).await.unwrap();

    let ids: BTreeSet<PlcTypeId> = rows.iter().map(PlcType::id).collect();
    assert!(ids.contains(&type_id(1)));
    assert!(ids.contains(&type_id(3)));
    assert!(!ids.contains(&type_id(2)), "a row of another machine");
    assert_eq!(ids.len(), rows.len(), "no row twice");
    assert!(
        rows.iter()
            .all(|row| row.machine().is_none() || row.machine() == Some(first.id()))
    );
    assert_eq!(rows.iter().filter(|row| row.machine().is_some()).count(), 2);
    for primitive in SHARED_PRIMITIVES {
        assert!(has_primitive(&rows, primitive, None), "{primitive:?}");
    }
    assert!(has_primitive(&rows, Primitive::String, Some(80)));
    assert!(has_primitive(&rows, Primitive::Wstring, Some(80)));
    assert!(has_primitive(&rows, Primitive::String, Some(254)));
    let find = |id: PlcTypeId| rows.iter().find(|row| row.id() == id).unwrap();
    assert_eq!(find(type_id(1)), &PlcType::try_from(own_string).unwrap());
    assert_eq!(find(type_id(3)), &PlcType::try_from(own_struct).unwrap());
}

async fn the_seed_rows_name_the_platforms_of_the_matrix(h: &dyn Harness) {
    let built = machine(102, "M102");
    insert(h, &built).await;
    let mut tx = begin(h).await;

    let rows = tx.types().type_rows(built.id()).await.unwrap();

    assert!(rows.iter().all(|row| row.machine().is_none()));
    let platforms_of = |primitive: Primitive, length: Option<u16>| -> BTreeSet<PlcPlatform> {
        rows.iter()
            .find(|row| {
                matches!(row.shape(),
                    RecordShape::Primitive { primitive: found, string_length, .. }
                        if *found == primitive && *string_length == length)
            })
            .map(|row| row.platforms().clone())
            .unwrap()
    };
    assert_eq!(
        platforms_of(Primitive::Int, None),
        PlcPlatform::ALL.into_iter().collect::<BTreeSet<_>>()
    );
    assert_eq!(
        platforms_of(Primitive::Ltime, None),
        BTreeSet::from([
            PlcPlatform::CodesysIec,
            PlcPlatform::TwinCat3,
            PlcPlatform::S7_1500,
        ])
    );
    assert!(!platforms_of(Primitive::Lreal, None).contains(&PlcPlatform::S7_300));
    assert!(platforms_of(Primitive::String, Some(80)).contains(&PlcPlatform::TwinCat3));
    assert!(!platforms_of(Primitive::String, Some(254)).contains(&PlcPlatform::TwinCat3));
}

contract_case!(
    type_rows_gives_the_seed_rows_and_the_rows_of_the_machine_only,
    type_rows_gives_the_seed_rows_and_the_rows_of_the_machine_only_memory,
    type_rows_gives_the_seed_rows_and_the_rows_of_the_machine_only_postgres
);
contract_case!(
    the_seed_rows_name_the_platforms_of_the_matrix,
    the_seed_rows_name_the_platforms_of_the_matrix_memory,
    the_seed_rows_name_the_platforms_of_the_matrix_postgres
);
