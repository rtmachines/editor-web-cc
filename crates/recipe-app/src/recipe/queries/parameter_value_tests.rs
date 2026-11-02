//! Tests of the value texts of `ListParameters` and of `value_text` (design D1, A43, ADR-0108).

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as _;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, PlatformTypes, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord,
    Primitive, RecordShape, TypeTree,
};
use recipe_domain::recipe::{Ordinal, ParameterKey, StoredParameter, StoredValue, ValueFault};
use serde_json::json;

use super::parameters::{ListParameters, ListParametersInput, ParameterList, value_text};
use super::parameters_tests::typed_world;
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, parameter_on, pkey, reference, tid, uid, vid,
};
use crate::memory::seed_type_id;

/// The platform of the test machine.
const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;

/// Seed one parameter on `U1` with the given global type row and value.
fn seed_parameter(
    fixture: &Fixture,
    (n, tag): (u128, &str),
    (plc_type, value): (PlcTypeId, StoredValue),
) {
    let parameter = StoredParameter {
        plc_type,
        value,
        limits: Vec::new(),
        ordinal: Ordinal::parse(u32::try_from(n).unwrap()).unwrap(),
        ..parameter_on(PLATFORM, n, tag, uid(101), 0)
    };
    fixture.store.seed_parameter(vid(101), &parameter);
}

fn seeded(primitive: Primitive, length: Option<u16>) -> PlcTypeId {
    seed_type_id(primitive, length).unwrap()
}

/// `typed_world` (INT 1, `ARRAY` 2, `STRING(20)` 3, struct 11) with `REAL` 4, `TIME` 5, and
/// an empty `STRING(80)` 6.
fn value_world() -> Fixture {
    let fixture = typed_world();
    let real = seeded(Primitive::Real, None);
    let time = seeded(Primitive::Time, None);
    let string = seeded(Primitive::String, Some(80));
    seed_parameter(
        &fixture,
        (4, "RATE"),
        (real, StoredValue::from(json!(85.5))),
    );
    seed_parameter(
        &fixture,
        (5, "HOLD"),
        (time, StoredValue::from(json!(90_000))),
    );
    seed_parameter(
        &fixture,
        (6, "EMPTY"),
        (string, StoredValue::from(json!(""))),
    );
    fixture
}

async fn list(fixture: &Fixture) -> ParameterList {
    ListParameters::new(fixture.context())
        .execute(
            &actor(Role::Operator),
            &meta(),
            ListParametersInput {
                reference: reference("R-1"),
                number: 1,
                unit: None,
                page: 0,
            },
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn list_parameters_gives_the_leaf_text_of_each_primitive_and_none_for_a_composite() {
    let fixture = value_world();

    let result = list(&fixture).await;

    let texts: BTreeMap<ParameterKey, &str> = result
        .value_texts
        .iter()
        .map(|(key, text)| (*key, text.as_str()))
        .collect();
    let expected = BTreeMap::from([
        (pkey(1), "120"),
        (pkey(3), "abc"),
        (pkey(4), "85.5"),
        (pkey(5), "T#1m30s"),
        (pkey(6), ""),
    ]);
    assert_eq!(
        texts, expected,
        "no entry for the array 2 and the struct 11"
    );
}

/// The rows `INT`, `TIME`, and `ARRAY[3..4] OF INT` (type 910) of the machine 1.
fn rows() -> Vec<PlcType> {
    let int = seeded(Primitive::Int, None);
    let primitive = |primitive: Primitive| {
        PlcType::try_from(PlcTypeRecord {
            id: seeded(primitive, None),
            machine: None,
            platforms: BTreeSet::from([PLATFORM]),
            shape: RecordShape::Primitive {
                primitive,
                leaf_class: primitive.leaf_class(),
                size_bytes: primitive.fixed_size_bytes().unwrap(),
                string_length: None,
            },
        })
        .unwrap()
    };
    let array = PlcType::try_from(PlcTypeRecord {
        id: tid(910),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Array {
            element: int,
            dimensions: ArrayDimensions::parse(vec![ArrayDimension::parse(3, 4).unwrap()]).unwrap(),
        },
    })
    .unwrap();
    vec![primitive(Primitive::Int), primitive(Primitive::Time), array]
}

fn tree(id: PlcTypeId) -> TypeTree {
    let types = PlatformTypes::collect(mid(1), PLATFORM, rows()).unwrap();
    TypeTree::resolve(id, &types).unwrap()
}

#[test]
fn value_text_writes_the_leaf_text_of_a_primitive_and_none_for_an_array() {
    let int = tree(seeded(Primitive::Int, None));
    let time = tree(seeded(Primitive::Time, None));
    let array = tree(tid(910));

    let number = value_text(&int, &StoredValue::from(json!(7))).unwrap();
    let duration = value_text(&time, &StoredValue::from(json!(90_000))).unwrap();
    let composite = value_text(&array, &StoredValue::from(json!([1, 2]))).unwrap();

    assert_eq!(number.as_deref(), Some("7"));
    assert_eq!(duration.as_deref(), Some("T#1m30s"));
    assert_eq!(composite, None);
}

#[test]
fn value_text_gives_a_fault_for_a_stored_value_that_does_not_match_its_tree() {
    let int = tree(seeded(Primitive::Int, None));

    let fault = value_text(&int, &StoredValue::from(json!("seven")));

    assert!(
        matches!(fault, Err(ValueFault::Mismatch { .. })),
        "got {fault:?}"
    );
}

#[tokio::test]
async fn list_parameters_gives_a_storage_defect_for_a_stored_value_that_does_not_match_its_type() {
    let fixture = typed_world();
    let int = seeded(Primitive::Int, None);
    seed_parameter(
        &fixture,
        (7, "BAD"),
        (int, StoredValue::from(json!("seven"))),
    );

    let error = ListParameters::new(fixture.context())
        .execute(
            &actor(Role::Operator),
            &meta(),
            ListParametersInput {
                reference: reference("R-1"),
                number: 1,
                unit: None,
                page: 0,
            },
        )
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(matches!(error, AppError::Storage(_)), "got {error:?}");
    let mut chain = Vec::new();
    let mut source = error.source();
    while let Some(inner) = source {
        chain.push(inner.to_string());
        source = inner.source();
    }
    let chain = chain.join(": ");
    let fault = value_text(&tree(int), &StoredValue::from(json!("seven"))).unwrap_err();
    assert!(chain.contains(&pkey(7).to_string()), "{chain}");
    assert!(chain.contains(&fault.to_string()), "{chain}");
    assert!(!chain.contains("seven"), "{chain}");
}
