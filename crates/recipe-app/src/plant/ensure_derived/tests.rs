use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::audit::{AggregateType, AuditAction};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, DerivedTypeReason, MemberName, PlantError, PlatformTypes,
    PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, TypeExpr, TypeMember,
    TypeName, TypeTrees,
};
use recipe_domain::recipe::{
    ArrayShape, Draft, RebindRequest, StoredDraft, StoredParameter, StoredValue, VersionState,
};
use serde_json::json;

use super::DerivedTypes;
use crate::context::audit_context;
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, parameter_on, reference, tid, uid, user, vid,
};
use crate::memory::{MemoryTransaction, seed_type_id};
use crate::ports::{Isolation, Transaction};

/// One open transaction with a working set over the rows of machine 1.
struct Harness {
    fixture: Fixture,
    tx: MemoryTransaction,
    types: DerivedTypes,
}

fn expr(text: &str) -> TypeExpr {
    TypeExpr::parse(text).unwrap()
}

fn working_set(rows: Vec<PlcType>, platform: PlcPlatform) -> DerivedTypes {
    DerivedTypes::new(
        mid(1),
        platform,
        rows,
        audit_context(&actor(Role::ProcessEngineer), &meta()),
    )
}

async fn harness(platform: PlcPlatform) -> Harness {
    let fixture = Fixture::new();
    fixture.seed_machine_on(platform, 1, "M1");
    open(fixture, platform).await
}

async fn open(fixture: Fixture, platform: PlcPlatform) -> Harness {
    let mut tx = fixture
        .store
        .begin_memory(Isolation::ReadCommitted)
        .unwrap();
    let rows = tx.types().type_rows(mid(1)).await.unwrap();
    Harness {
        fixture,
        tx,
        types: working_set(rows, platform),
    }
}

impl Harness {
    async fn ensure(&mut self, text: &str) -> Result<PlcTypeId, AppError> {
        self.types.ensure_expr(self.tx.types(), &expr(text)).await
    }

    /// The rows that belong to machine 1, as the transaction holds them.
    async fn own_rows(&mut self) -> Vec<PlcType> {
        let machine = Some(mid(1));
        self.tx
            .types()
            .type_rows(mid(1))
            .await
            .unwrap()
            .into_iter()
            .filter(|row| row.machine() == machine)
            .collect()
    }
}

fn platforms(platform: PlcPlatform) -> BTreeSet<PlcPlatform> {
    BTreeSet::from([platform])
}

fn dims(lower: i32, upper: i32) -> ArrayDimensions {
    ArrayDimensions::parse(vec![ArrayDimension::parse(lower, upper).unwrap()]).unwrap()
}

fn zone(id: u128, revision: u32, supersedes: Option<u128>) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: platforms(PlcPlatform::TwinCat3),
        shape: RecordShape::Struct {
            name: TypeName::parse("ST_Zone").unwrap(),
            revision,
            supersedes: supersedes.map(tid),
            members: vec![TypeMember {
                name: MemberName::parse("setpoint").unwrap(),
                type_id: seed_type_id(Primitive::Real, None).unwrap(),
            }],
        },
    })
    .unwrap()
}

fn array_row(id: u128, element: PlcTypeId, dimensions: ArrayDimensions) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: platforms(PlcPlatform::TwinCat3),
        shape: RecordShape::Array {
            element,
            dimensions,
        },
    })
    .unwrap()
}

fn reason_of(error: &AppError) -> Option<DerivedTypeReason> {
    match error {
        AppError::Plant(PlantError::DerivedTypeInvalid { reason, .. }) => Some(*reason),
        _ => None,
    }
}

#[tokio::test]
async fn the_first_sized_string_creates_one_row_and_one_entry() {
    let mut h = harness(PlcPlatform::TwinCat3).await;
    let id = h.ensure("STRING(20)").await.unwrap();
    let rows = h.own_rows().await;
    assert_eq!(rows.len(), 1);
    let row = rows.first().unwrap();
    assert_eq!(row.id(), id);
    assert_eq!(row.platforms(), &platforms(PlcPlatform::TwinCat3));
    assert!(matches!(
        row.shape(),
        RecordShape::Primitive {
            primitive: Primitive::String,
            size_bytes: 22,
            string_length: Some(20),
            ..
        }
    ));
    let entries = h.types.entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::DerivedTypeCreated);
    assert_eq!(entry.target().entity_type(), "plc_type");
    assert_eq!(entry.target().entity_id(), id.as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Machine);
    assert_eq!(entry.target().aggregate_id(), mid(1).as_uuid());
    assert!(entry.before().is_none());
    assert_eq!(
        entry.after().unwrap().get("id"),
        Some(&json!(id.to_string()))
    );
}

#[tokio::test]
async fn the_second_use_creates_no_row_and_no_entry() {
    let mut h = harness(PlcPlatform::TwinCat3).await;
    let first = h.ensure("STRING(20)").await.unwrap();
    let second = h.ensure("STRING(20)").await.unwrap();
    assert_eq!(first, second);
    assert_eq!(h.own_rows().await.len(), 1);
    assert_eq!(h.types.entries().len(), 1);
}

#[tokio::test]
async fn a_stale_working_set_finds_the_row_through_the_catalog() {
    let mut h = harness(PlcPlatform::TwinCat3).await;
    let stale_rows = h.tx.types().type_rows(mid(1)).await.unwrap();
    let first = h.ensure("STRING(20)").await.unwrap();
    let mut stale = working_set(stale_rows, PlcPlatform::TwinCat3);
    let second = stale
        .ensure_expr(h.tx.types(), &expr("STRING(20)"))
        .await
        .unwrap();
    assert_eq!(first, second);
    assert!(stale.entries().is_empty());
    assert_eq!(h.own_rows().await.len(), 1);
}

#[tokio::test]
async fn a_string_without_length_uses_the_seed_row_of_the_platform() {
    let mut h = harness(PlcPlatform::TwinCat3).await;
    let id = h.ensure("STRING").await.unwrap();
    assert_eq!(id, seed_type_id(Primitive::String, Some(80)).unwrap());
    assert!(h.own_rows().await.is_empty());
    assert!(h.types.entries().is_empty());
}

#[tokio::test]
async fn a_string_without_length_on_s7_uses_the_254_seed_row() {
    let mut h = harness(PlcPlatform::S7_1500).await;
    let id = h.ensure("WSTRING").await.unwrap();
    assert_eq!(id, seed_type_id(Primitive::Wstring, Some(254)).unwrap());
    assert!(h.types.entries().is_empty());
}

#[tokio::test]
async fn a_primitive_is_the_global_row() {
    let mut h = harness(PlcPlatform::TwinCat3).await;
    let id = h.ensure("INT").await.unwrap();
    assert_eq!(id, seed_type_id(Primitive::Int, None).unwrap());
    assert!(h.types.entries().is_empty());
}

#[tokio::test]
async fn an_array_of_real_has_the_global_real_as_element() {
    let mut h = harness(PlcPlatform::TwinCat3).await;
    let id = h.ensure("ARRAY[1..8] OF REAL").await.unwrap();
    let rows = h.own_rows().await;
    assert_eq!(rows.len(), 1);
    let row = rows.first().unwrap();
    assert_eq!(row.id(), id);
    assert_eq!(
        row.shape(),
        &RecordShape::Array {
            element: seed_type_id(Primitive::Real, None).unwrap(),
            dimensions: dims(1, 8),
        }
    );
    assert_eq!(row.platforms(), &platforms(PlcPlatform::TwinCat3));
    assert_eq!(h.types.entries().len(), 1);
}

#[tokio::test]
async fn nested_arrays_create_the_inner_row_first() {
    let mut h = harness(PlcPlatform::TwinCat3).await;
    let outer = h.ensure("ARRAY[1..2] OF ARRAY[1..3] OF INT").await.unwrap();
    let rows = h.own_rows().await;
    assert_eq!(rows.len(), 2);
    let entries = h.types.entries();
    assert_eq!(entries.len(), 2);
    let inner = *entries.first().unwrap().target().entity_id();
    assert_eq!(
        entries.get(1).unwrap().target().entity_id(),
        outer.as_uuid()
    );
    let inner_row = rows
        .iter()
        .find(|row| row.id().as_uuid() == &inner)
        .unwrap();
    assert_eq!(
        inner_row.shape(),
        &RecordShape::Array {
            element: seed_type_id(Primitive::Int, None).unwrap(),
            dimensions: dims(1, 3),
        }
    );
    let outer_row = rows.iter().find(|row| row.id() == outer).unwrap();
    assert_eq!(
        outer_row.shape(),
        &RecordShape::Array {
            element: PlcTypeId::from(inner),
            dimensions: dims(1, 2),
        }
    );
}

#[tokio::test]
async fn an_array_of_a_stored_struct_uses_the_struct_row() {
    let h = harness(PlcPlatform::TwinCat3).await;
    h.fixture.store.seed_type(&zone(901, 1, None));
    let mut h = open(h.fixture, PlcPlatform::TwinCat3).await;
    let id = h.ensure("ARRAY[1..4] OF ST_Zone@1").await.unwrap();
    let rows = h.own_rows().await;
    let row = rows.iter().find(|row| row.id() == id).unwrap();
    assert_eq!(
        row.shape(),
        &RecordShape::Array {
            element: tid(901),
            dimensions: dims(1, 4),
        }
    );
    assert_eq!(h.ensure("ST_Zone@1").await.unwrap(), tid(901));
}

#[tokio::test]
async fn wstring_on_s7_300_is_not_on_the_platform() {
    let mut h = harness(PlcPlatform::S7_300).await;
    let error = h.ensure("WSTRING(10)").await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert_eq!(reason_of(&error), Some(DerivedTypeReason::NotOnPlatform));
    let error = h.ensure("WSTRING").await.unwrap_err();
    assert_eq!(reason_of(&error), Some(DerivedTypeReason::NotOnPlatform));
    assert!(h.types.entries().is_empty());
}

#[tokio::test]
async fn a_primitive_missing_on_the_platform_is_not_on_the_platform() {
    let mut h = harness(PlcPlatform::S7_300).await;
    let error = h.ensure("LREAL").await.unwrap_err();
    assert_eq!(reason_of(&error), Some(DerivedTypeReason::NotOnPlatform));
}

#[tokio::test]
async fn a_string_above_the_range_is_out_of_range() {
    let mut h = harness(PlcPlatform::S7_1500).await;
    let error = h.ensure("STRING(300)").await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert_eq!(reason_of(&error), Some(DerivedTypeReason::OutOfRange));
    assert!(h.own_rows().await.is_empty());
}

#[tokio::test]
async fn an_unknown_struct_revision_is_unknown_struct() {
    let mut h = harness(PlcPlatform::TwinCat3).await;
    let error = h.ensure("ST_Zone@9").await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert_eq!(reason_of(&error), Some(DerivedTypeReason::UnknownStruct));
    h.fixture.store.seed_type(&zone(901, 1, None));
    let mut h = open(h.fixture, PlcPlatform::TwinCat3).await;
    let error = h.ensure("ST_Zone@9").await.unwrap_err();
    assert_eq!(reason_of(&error), Some(DerivedTypeReason::UnknownStruct));
}

#[tokio::test]
async fn an_array_above_the_leaf_limit_gives_plt_018() {
    let mut h = harness(PlcPlatform::TwinCat3).await;
    let error = h.ensure("ARRAY[1..16385] OF INT").await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_018);
}

#[tokio::test]
async fn an_array_at_the_leaf_limit_is_accepted() {
    let mut h = harness(PlcPlatform::TwinCat3).await;
    assert!(h.ensure("ARRAY[1..16384] OF INT").await.is_ok());
}

#[tokio::test]
async fn the_working_set_gives_the_platform_types_with_the_new_rows() {
    let mut h = harness(PlcPlatform::TwinCat3).await;
    let id = h.ensure("ARRAY[1..8] OF STRING(20)").await.unwrap();
    let types = h.types.platform_types().unwrap();
    assert!(types.get(id).is_some());
    assert_eq!(
        types.rows().filter(|row| row.machine().is_some()).count(),
        2
    );
}

/// The draft with a parameter of the type `ARRAY[1..4] OF ST_Zone@1` and the shape that a rebind to
/// `ST_Zone@2` needs.
async fn rebind_shape(fixture: &Fixture) -> ArrayShape {
    let element = array_row(903, tid(901), dims(1, 4));
    for row in [zone(901, 1, None), zone(902, 2, Some(901)), element] {
        fixture.store.seed_type(&row);
    }
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let rows = tx.types().type_rows(mid(1)).await.unwrap();
    let platform_types = PlatformTypes::collect(mid(1), PlcPlatform::TwinCat3, rows).unwrap();
    let trees = TypeTrees::resolve([tid(901), tid(902), tid(903)], &platform_types);
    let parameter = StoredParameter {
        plc_type: tid(903),
        value: StoredValue::from(json!([
            {"setpoint": 1}, {"setpoint": 1}, {"setpoint": 1}, {"setpoint": 1}
        ])),
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::TwinCat3, 1, "ZONES", uid(101), 0)
    };
    let draft = Draft::load(StoredDraft {
        reference: reference("R-100"),
        machine: mid(1),
        platform: PlcPlatform::TwinCat3,
        version: vid(1),
        number: 1,
        state: VersionState::LiveDraft,
        row_version: 1,
        created_by: user(2),
        editors: Vec::new(),
        parameters: vec![parameter],
    })
    .unwrap();
    let request = RebindRequest {
        from: tid(901),
        to: tid(902),
        new_member_values: BTreeMap::new(),
    };
    let plan = draft.plan_rebind(&request, &trees).unwrap();
    let shapes = plan.array_shapes();
    assert_eq!(shapes.len(), 1);
    shapes.into_iter().next().unwrap()
}

#[tokio::test]
async fn an_array_shape_creates_the_row_of_the_equivalent_type_expression() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let shape = rebind_shape(&fixture).await;
    let mut by_shape = open(fixture, PlcPlatform::TwinCat3).await;
    let shape_id = by_shape
        .types
        .ensure_shape(by_shape.tx.types(), &shape)
        .await
        .unwrap();
    assert_eq!(by_shape.types.entries().len(), 1);

    let mut by_expr = by_shape;
    by_expr.types = working_set(
        by_expr.tx.types().type_rows(mid(1)).await.unwrap(),
        PlcPlatform::TwinCat3,
    );
    let expr_id = by_expr.ensure("ARRAY[1..4] OF ST_Zone@2").await.unwrap();
    assert_eq!(shape_id, expr_id);
    assert!(by_expr.types.entries().is_empty());
    let rows = by_expr.own_rows().await;
    let row = rows.iter().find(|row| row.id() == shape_id).unwrap();
    assert_eq!(
        row.shape(),
        &RecordShape::Array {
            element: tid(902),
            dimensions: dims(1, 4),
        }
    );
}
